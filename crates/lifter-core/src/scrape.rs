//! lifter's two discovery methods.
//!
//! * `api_json`: fetch a JSON document, read the version with a JSONPath
//!   (`version_tag`), collect download URLs with another JSONPath
//!   (`anchor_tag`) and keep the ones matching `anchor_text`.
//! * HTML: fetch a page, walk the links matched by a CSS selector
//!   (`anchor_tag`), keep the ones whose text matches `anchor_text`, and read
//!   the version from another CSS selector (`version_tag`).
//!
//! lifter stopped at the first match. These return every match so a UI can
//! show the choices.

use anyhow::{anyhow, Context, Result};
use itertools::Itertools;
use log::*;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::http::{self, RequestOptions};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct JsonQuery {
    /// JSONPath for the version string, e.g. `$.tag_name`.
    pub version_path: String,
    /// Optional JSONPath for a commit id (for tags like `nightly` that never change).
    pub commit_path: String,
    /// JSONPath for the download URLs, e.g. `$.assets.*.browser_download_url`.
    pub urls_path: String,
    /// Regex a URL must match (searched, as lifter did for JSON). Empty = all.
    pub url_regex: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HtmlQuery {
    /// CSS selector for candidate links. Empty = `a`.
    pub link_selector: String,
    /// Regex that must fully match the link text (lifter) or the file name
    /// at the end of the href. Empty = any link.
    pub link_text_regex: String,
    /// CSS selector whose text is the version. Empty = use the first capture
    /// group of `link_text_regex`, then the version number found in the link.
    pub version_selector: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub url: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Hit {
    pub version: Option<String>,
    pub commit: Option<String>,
    pub links: Vec<Link>,
}

fn json_strings<'a>(data: &'a serde_json::Value, path: &str) -> Result<Vec<&'a str>> {
    use jsonpath_rust::JsonPath;
    let found = data
        .query(path)
        .map_err(|e| anyhow!("bad JSONPath {path:?}: {e}"))?;
    Ok(found.into_iter().filter_map(|v| v.as_str()).collect())
}

pub fn extract_from_json(data: &serde_json::Value, q: &JsonQuery) -> Result<Hit> {
    let version = if q.version_path.is_empty() {
        None
    } else {
        json_strings(data, &q.version_path)?
            .first()
            .map(|s| s.to_string())
    };
    let commit = if q.commit_path.is_empty() {
        None
    } else {
        json_strings(data, &q.commit_path)?
            .first()
            .map(|s| s.to_string())
    };
    let re = regex::Regex::new(&q.url_regex).context("bad URL regex")?;
    let links = json_strings(data, &q.urls_path)?
        .into_iter()
        .filter(|u| re.is_match(u))
        .map(|u| Link {
            url: u.to_string(),
            text: file_name_of(u),
        })
        .collect();
    Ok(Hit {
        version,
        commit,
        links,
    })
}

pub fn fetch_json(url: &str, q: &JsonQuery, opts: &RequestOptions) -> Result<Hit> {
    let data = http::get_json(url, opts)?;
    extract_from_json(&data, q)
}

/// Last path segment of a URL, percent-decoded.
pub fn file_name_of(u: &str) -> String {
    let path = Url::parse(u)
        .ok()
        .and_then(|u| u.path_segments().and_then(|mut s| s.next_back().map(String::from)))
        .unwrap_or_else(|| u.rsplit('/').next().unwrap_or(u).to_string());
    percent_decode(&path)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(b) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn extract_from_html(body: &str, page_url: &str, q: &HtmlQuery) -> Result<Hit> {
    let doc = Html::parse_document(body);
    let sel_src = if q.link_selector.trim().is_empty() {
        "a"
    } else {
        q.link_selector.as_str()
    };
    let links_sel =
        Selector::parse(sel_src).map_err(|e| anyhow!("bad link selector {sel_src:?}: {e:?}"))?;
    let re = if q.link_text_regex.is_empty() {
        None
    } else {
        Some(regex::Regex::new(&format!("^(?:{})$", q.link_text_regex)).context("bad link regex")?)
    };
    let base = Url::parse(page_url).with_context(|| format!("bad page URL {page_url}"))?;

    let mut links = Vec::new();
    let mut captured_version = None;
    for el in doc.select(&links_sel) {
        let Some(href) = el.value().attr("href") else {
            continue;
        };
        let Ok(abs) = base.join(href.trim()) else {
            continue;
        };
        let url = abs.to_string();
        let text = el.text().join(" ").split_whitespace().join(" ");
        let fname = file_name_of(&url);
        if let Some(re) = &re {
            let caps = re.captures(&text).or_else(|| re.captures(&fname));
            let Some(caps) = caps else { continue };
            if captured_version.is_none() {
                captured_version = caps.get(1).map(|m| m.as_str().to_string());
            }
        }
        trace!("html link match: {text} -> {url}");
        if !links.iter().any(|l: &Link| l.url == url) {
            links.push(Link {
                url,
                text: if text.is_empty() { fname } else { text },
            });
        }
    }

    let version = if !q.version_selector.trim().is_empty() {
        let vs = Selector::parse(&q.version_selector)
            .map_err(|e| anyhow!("bad version selector {:?}: {e:?}", q.version_selector))?;
        doc.select(&vs)
            .next()
            .map(|e| e.text().join("").trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        captured_version.or_else(|| {
            links
                .first()
                .and_then(|l| crate::assets::simple_semver(&file_name_of(&l.url)).map(String::from))
        })
    };
    debug!("html scrape of {page_url}: {} links, version {version:?}", links.len());
    Ok(Hit {
        version,
        commit: None,
        links,
    })
}

pub fn fetch_html(url: &str, q: &HtmlQuery, opts: &RequestOptions) -> Result<Hit> {
    let body = http::get_string(url, opts)?;
    extract_from_html(&body, url, q)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_query_like_lifter_github_template() {
        let data: serde_json::Value = serde_json::from_str(
            r#"{"tag_name":"14.1.0","assets":[
              {"browser_download_url":"https://x/ripgrep-14.1.0-x86_64-unknown-linux-musl.tar.gz"},
              {"browser_download_url":"https://x/ripgrep-14.1.0-x86_64-pc-windows-msvc.zip"}]}"#,
        )
        .unwrap();
        let hit = extract_from_json(
            &data,
            &JsonQuery {
                version_path: "$.tag_name".into(),
                urls_path: "$.assets.*.browser_download_url".into(),
                url_regex: r"ripgrep-(\d+\.\d+\.\d+)-x86_64-unknown-linux-musl.tar.gz".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(hit.version.as_deref(), Some("14.1.0"));
        assert_eq!(hit.links.len(), 1);
    }

    #[test]
    fn html_query_resolves_relative_links_and_captures_version() {
        let body = r#"<html><body>
            <a href="/dl/tool-2.3.4-linux.tar.gz">tool-2.3.4-linux.tar.gz</a>
            <a href="other.zip">other</a></body></html>"#;
        let hit = extract_from_html(
            body,
            "https://example.com/downloads/index.html",
            &HtmlQuery {
                link_text_regex: r"tool-(\d+\.\d+\.\d+)-linux\.tar\.gz".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(hit.version.as_deref(), Some("2.3.4"));
        assert_eq!(hit.links[0].url, "https://example.com/dl/tool-2.3.4-linux.tar.gz");
    }

    #[test]
    fn file_name_is_decoded() {
        assert_eq!(file_name_of("https://h/a/My%20App.AppImage?x=1"), "My App.AppImage");
    }
}
