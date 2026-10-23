//! Release sources. GitHub / GitLab / Forgejo use their APIs; `html` and
//! `json` are lifter's two generic methods for any other site.

mod forgejo;
mod generic;
mod github;
mod gitlab;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use lifter_core::http::RequestOptions;
use serde::Serialize;
use url::Url;

use crate::model::{App, Release, Settings, SourceKind};

/// Repository details shown when adding an app.
#[derive(Serialize, Clone, Debug, Default)]
pub struct Meta {
    pub name: String,
    pub author: String,
    pub description: String,
    pub icon_url: Option<String>,
    /// The canonical URL (follows repository renames).
    pub url: String,
}

pub fn parse_time(s: Option<&str>) -> Option<DateTime<Utc>> {
    s.and_then(|s| DateTime::parse_from_rfc3339(s).ok()).map(|d| d.with_timezone(&Utc))
}

/// Accepts `owner/repo` shorthand and URLs with or without a scheme.
pub fn normalize_url(input: &str) -> String {
    let s = input.trim().trim_end_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s);
    if s.contains("://") {
        return s.to_string();
    }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 && !parts[0].contains('.') && parts.iter().all(|p| !p.is_empty()) {
        return format!("https://github.com/{s}");
    }
    format!("https://{s}")
}

/// Guess the source from a URL without network access.
pub fn guess_kind(url: &str) -> SourceKind {
    let Ok(u) = Url::parse(url) else {
        return SourceKind::Html;
    };
    let host = u.host_str().unwrap_or("").to_lowercase();
    let segs = u.path_segments().map(|s| s.filter(|x| !x.is_empty()).count()).unwrap_or(0);
    let path = u.path().to_lowercase();
    if path.ends_with(".json") || host.starts_with("api.") && host != "api.github.com" {
        return SourceKind::Json;
    }
    if host == "github.com" || host == "www.github.com" {
        return if segs >= 2 { SourceKind::Github } else { SourceKind::Html };
    }
    if host == "gitlab.com" || host.contains("gitlab") {
        return SourceKind::Gitlab;
    }
    if host == "codeberg.org" || host.contains("gitea") || host.contains("forgejo") {
        return SourceKind::Forgejo;
    }
    SourceKind::Html
}

/// Like [`guess_kind`], but probes unknown hosts for a Forgejo/Gitea or
/// GitLab API before falling back to HTML.
pub fn detect_kind(url: &str) -> SourceKind {
    let guess = guess_kind(url);
    if guess != SourceKind::Html {
        return guess;
    }
    let Ok(u) = Url::parse(url) else { return guess };
    let segs = u.path_segments().map(|s| s.filter(|x| !x.is_empty()).count()).unwrap_or(0);
    if segs < 2 {
        return guess;
    }
    let origin = u.origin().ascii_serialization();
    let quick = RequestOptions { max_attempts: 1, ..Default::default() };
    if lifter_core::http::get_json(&format!("{origin}/api/v1/version"), &quick)
        .ok()
        .is_some_and(|v| v.get("version").is_some())
    {
        return SourceKind::Forgejo;
    }
    if let Ok(resp) = lifter_core::http::get(&format!("{origin}/api/v4/version"), &quick) {
        if resp.headers().contains_key("x-gitlab-meta") || resp.status().is_success() {
            return SourceKind::Gitlab;
        }
    }
    guess
}

pub fn request_opts(app: &App, _s: &Settings) -> RequestOptions {
    let mut o = match app.source {
        SourceKind::Html => RequestOptions::browser(),
        _ => RequestOptions::default(),
    };
    o.allow_insecure = app.settings.allow_insecure;
    o
}

pub fn metadata(kind: SourceKind, url: &str, s: &Settings) -> Result<Meta> {
    match kind {
        SourceKind::Github => github::metadata(url, s),
        SourceKind::Gitlab => gitlab::metadata(url, s),
        SourceKind::Forgejo => forgejo::metadata(url, s),
        SourceKind::Html | SourceKind::Json => Ok(generic::metadata(url)),
    }
}

/// All releases, newest first as the source reports them.
pub fn releases(app: &App, s: &Settings) -> Result<Vec<Release>> {
    let list = match app.source {
        SourceKind::Github => github::releases(app, s)?,
        SourceKind::Gitlab => gitlab::releases(app, s)?,
        SourceKind::Forgejo => forgejo::releases(app, s)?,
        SourceKind::Html => generic::html_release(app, s)?,
        SourceKind::Json => generic::json_release(app, s)?,
    };
    if list.is_empty() {
        bail!("no releases found");
    }
    Ok(list)
}

/// Split a repository URL into (origin, path segments).
pub fn split_repo(url: &str, min_segments: usize) -> Result<(String, Vec<String>)> {
    let u = Url::parse(url)?;
    let segs: Vec<String> = u
        .path_segments()
        .map(|s| s.filter(|x| !x.is_empty()).map(String::from).collect())
        .unwrap_or_default();
    if segs.len() < min_segments {
        bail!("expected a repository URL like https://{}/owner/repo", u.host_str().unwrap_or("host"));
    }
    Ok((u.origin().ascii_serialization(), segs))
}
