use anyhow::{Context, Result};
use lifter_core::http::{self, RequestOptions};
use serde_json::Value;

use super::{parse_time, split_repo, Meta};
use crate::model::{App, RawAsset, Release, Settings};

fn opts(s: &Settings) -> RequestOptions {
    let o = RequestOptions::default()
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    let token = if s.github_token.trim().is_empty() {
        std::env::var("GITHUB_TOKEN").unwrap_or_default()
    } else {
        s.github_token.trim().to_string()
    };
    if token.is_empty() {
        o
    } else {
        o.header("Authorization", format!("Bearer {token}"))
    }
}

/// owner/repo from any github.com URL (extra path parts are ignored).
pub fn repo_of(url: &str) -> Result<String> {
    let (_, segs) = split_repo(url, 2)?;
    Ok(format!("{}/{}", segs[0], segs[1].trim_end_matches(".git")))
}

pub fn metadata(url: &str, s: &Settings) -> Result<Meta> {
    let repo = repo_of(url)?;
    let v = http::get_json(&format!("https://api.github.com/repos/{repo}"), &opts(s))
        .with_context(|| format!("looking up GitHub repository {repo}"))?;
    let full = v["full_name"].as_str().unwrap_or(&repo).to_string();
    Ok(Meta {
        name: v["name"].as_str().unwrap_or_default().to_string(),
        author: v["owner"]["login"].as_str().unwrap_or_default().to_string(),
        description: v["description"].as_str().unwrap_or_default().to_string(),
        icon_url: v["owner"]["avatar_url"].as_str().map(String::from),
        url: format!("https://github.com/{full}"),
    })
}

fn parse_release(r: &Value) -> Release {
    let assets = r["assets"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|x| RawAsset {
                    name: x["name"].as_str().unwrap_or_default().to_string(),
                    url: x["browser_download_url"].as_str().unwrap_or_default().to_string(),
                    size: x["size"].as_u64().unwrap_or(0),
                    sha256: x["digest"]
                        .as_str()
                        .and_then(|d| d.strip_prefix("sha256:"))
                        .map(String::from),
                    updated_at: parse_time(x["updated_at"].as_str()),
                })
                .collect()
        })
        .unwrap_or_default();
    Release {
        tag: r["tag_name"].as_str().unwrap_or_default().to_string(),
        title: r["name"].as_str().unwrap_or_default().to_string(),
        notes: r["body"].as_str().unwrap_or_default().to_string(),
        url: r["html_url"].as_str().unwrap_or_default().to_string(),
        published_at: parse_time(r["published_at"].as_str().or(r["created_at"].as_str())),
        prerelease: r["prerelease"].as_bool().unwrap_or(false),
        commit: None,
        assets,
    }
}

pub fn releases(app: &App, s: &Settings) -> Result<Vec<Release>> {
    let repo = repo_of(&app.url)?;
    let o = opts(s);
    let list = http::get_json(
        &format!("https://api.github.com/repos/{repo}/releases?per_page=100"),
        &o,
    )?;
    let mut out: Vec<Release> = list
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|r| !r["draft"].as_bool().unwrap_or(false))
                .map(parse_release)
                .collect()
        })
        .unwrap_or_default();

    // Obtainium's "verify the 'latest' tag": trust GitHub's own idea of the
    // latest release and put it first.
    if app.settings.verify_latest_tag {
        let latest = http::get_json(&format!("https://api.github.com/repos/{repo}/releases/latest"), &o)?;
        let tag = latest["tag_name"].as_str().unwrap_or_default().to_string();
        out.retain(|r| r.tag != tag);
        out.insert(0, parse_release(&latest));
    }
    Ok(out)
}
