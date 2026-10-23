//! HTTP helpers. The retry loop is lifter's: retry on 403/408/425/429/5xx
//! with a growing sleep. Two changes for interactive use: fewer attempts by
//! default, and an exhausted GitHub/GitLab rate limit fails at once with a
//! clear message instead of sleeping for minutes.

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use log::*;
use sha2::{Digest, Sha256};

/// lifter sends a browser user agent; some download hosts reject anything else.
pub const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
pub const API_UA: &str = concat!("Obtainit/", env!("CARGO_PKG_VERSION"), " (lifter-core)");

const MAX_TEXT_BODY: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct RequestOptions {
    pub headers: Vec<(String, String)>,
    /// Total tries including the first. lifter used 10.
    pub max_attempts: u32,
    /// Permit plain `http://` URLs.
    pub allow_insecure: bool,
}

impl Default for RequestOptions {
    fn default() -> Self {
        RequestOptions {
            headers: vec![("User-Agent".into(), API_UA.into())],
            max_attempts: 4,
            allow_insecure: false,
        }
    }
}

impl RequestOptions {
    pub fn browser() -> Self {
        RequestOptions {
            headers: vec![("User-Agent".into(), BROWSER_UA.into())],
            ..Default::default()
        }
    }

    pub fn header(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        let k = k.into();
        self.headers.retain(|(hk, _)| !hk.eq_ignore_ascii_case(&k));
        self.headers.push((k, v.into()));
        self
    }
}

/// Like lifter's `http_agent`: every status comes back as `Ok(Response)` so
/// the retry loop can look at error bodies.
pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .build()
        .into()
}

pub type Response = ureq::http::Response<ureq::Body>;

/// Collapse whitespace and cap length so an error body fits on one line
/// (lifter's `one_line`).
pub fn one_line(body: &str) -> String {
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > 200 {
        format!("{}...", collapsed.chars().take(200).collect::<String>())
    } else {
        collapsed
    }
}

fn header_str<'a>(resp: &'a Response, name: &str) -> Option<&'a str> {
    resp.headers().get(name).and_then(|v| v.to_str().ok())
}

/// GET with lifter's retry/backoff. Returns the first 2xx response.
pub fn get(url: &str, opts: &RequestOptions) -> Result<Response> {
    if url.starts_with("http://") && !opts.allow_insecure {
        bail!("refusing insecure http:// URL {url} (enable \"Allow insecure HTTP\" for this app)");
    }
    let max = opts.max_attempts.max(1);
    let mut attempt = 0;
    loop {
        attempt += 1;
        let mut req = agent().get(url);
        for (k, v) in &opts.headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let mut resp = req.call().map_err(|e| anyhow!("fetching {url}: {e}"))?;
        let status = resp.status().as_u16();
        match status {
            200..=299 => return Ok(resp),
            403 | 429 if header_str(&resp, "x-ratelimit-remaining") == Some("0") => {
                let reset = header_str(&resp, "x-ratelimit-reset")
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|epoch| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(epoch);
                        format!(" (resets in {} min)", epoch.saturating_sub(now) / 60 + 1)
                    })
                    .unwrap_or_default();
                bail!("API rate limit reached for {url}{reset}. Add an access token in Settings to raise the limit.");
            }
            403 | 408 | 425 | 429 | 500 | 502 | 503 | 504 if attempt < max => {
                let body = resp.body_mut().read_to_string().unwrap_or_default();
                let wait = (attempt as u64 * 4).min(60);
                info!("Got status {status} fetching {url}: {}. Retrying in {wait}s", one_line(&body));
                std::thread::sleep(Duration::from_secs(wait));
            }
            _ => {
                let body = resp.body_mut().read_to_string().unwrap_or_default();
                bail!("fetching {url}: status {status}: {}", one_line(&body));
            }
        }
    }
}

pub fn get_string(url: &str, opts: &RequestOptions) -> Result<String> {
    let mut resp = get(url, opts)?;
    resp.body_mut()
        .with_config()
        .limit(MAX_TEXT_BODY)
        .read_to_string()
        .with_context(|| format!("reading response from {url}"))
}

pub fn get_json(url: &str, opts: &RequestOptions) -> Result<serde_json::Value> {
    let body = get_string(url, opts)?;
    serde_json::from_str(&body).with_context(|| format!("parsing JSON from {url}"))
}

/// Result of a finished download.
pub struct Downloaded {
    pub bytes: u64,
    pub sha256: String,
}

/// Stream `url` into `dest` (via `dest.part`, renamed on success), calling
/// `progress(received, total)` along the way. lifter refused to save an
/// error page over an existing binary; the 2xx check in [`get`] keeps that.
pub fn download_to(
    url: &str,
    opts: &RequestOptions,
    dest: &Path,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<Downloaded> {
    let mut resp = get(url, opts)?;
    let total = header_str(&resp, "content-length").and_then(|s| s.parse::<u64>().ok());
    let part = dest.with_extension(match dest.extension().and_then(|e| e.to_str()) {
        Some(e) => format!("{e}.part"),
        None => "part".into(),
    });
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(&part)
        .with_context(|| format!("creating {}", part.display()))?;
    let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut received = 0u64;
    let mut last_report = std::time::Instant::now();
    progress(0, total);
    loop {
        let n = reader
            .read(&mut buf)
            .with_context(|| format!("reading download from {url}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        received += n as u64;
        if last_report.elapsed() >= Duration::from_millis(150) {
            progress(received, total);
            last_report = std::time::Instant::now();
        }
    }
    file.flush()?;
    drop(file);
    if let Some(t) = total {
        if received != t {
            let _ = std::fs::remove_file(&part);
            bail!("download of {url} was cut short ({received} of {t} bytes)");
        }
    }
    std::fs::rename(&part, dest)?;
    progress(received, total.or(Some(received)));
    Ok(Downloaded {
        bytes: received,
        sha256: hex(&hasher.finalize()),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut f, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}
