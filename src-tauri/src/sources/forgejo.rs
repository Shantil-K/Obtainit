//! Codeberg, Forgejo and Gitea (same API).

use anyhow::{Context, Result};
use lifter_core::http::{self, RequestOptions};
use serde_json::Value;

use super::{parse_time, split_repo, Meta};
use crate::model::{App, RawAsset, Release, Settings};

fn opts(s: &Settings) -> RequestOptions {
    let o = RequestOptions::default();
    if s.forgejo_token.trim().is_empty() {
        o
    } else {
        o.header("Authorization", format!("token {}", s.forgejo_token.trim()))
    }
}

pub fn metadata(url: &str, s: &Settings) -> Result<Meta> {
    let (origin, segs) = split_repo(url, 2)?;
    let repo = format!("{}/{}", segs[0], segs[1]);
    let v = http::get_json(&format!("{origin}/api/v1/repos/{repo}"), &opts(s))
        .with_context(|| format!("looking up repository {repo}"))?;
    let avatar = v["avatar_url"]
        .as_str()
        .filter(|a| !a.is_empty())
        .or(v["owner"]["avatar_url"].as_str())
        .map(String::from);
    Ok(Meta {
        name: v["name"].as_str().unwrap_or_default().to_string(),
        author: v["owner"]["login"].as_str().unwrap_or_default().to_string(),
        description: v["description"].as_str().unwrap_or_default().to_string(),
        icon_url: avatar,
        url: v["html_url"].as_str().map(String::from).unwrap_or(format!("{origin}/{repo}")),
    })
}

pub fn releases(app: &App, s: &Settings) -> Result<Vec<Release>> {
    let (origin, segs) = split_repo(&app.url, 2)?;
    let repo = format!("{}/{}", segs[0], segs[1]);
    let list = http::get_json(&format!("{origin}/api/v1/repos/{repo}/releases?limit=50"), &opts(s))?;
    Ok(list
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|r| !r["draft"].as_bool().unwrap_or(false))
                .map(|r: &Value| Release {
                    tag: r["tag_name"].as_str().unwrap_or_default().to_string(),
                    title: r["name"].as_str().unwrap_or_default().to_string(),
                    notes: r["body"].as_str().unwrap_or_default().to_string(),
                    url: r["html_url"].as_str().unwrap_or_default().to_string(),
                    published_at: parse_time(r["published_at"].as_str().or(r["created_at"].as_str())),
                    prerelease: r["prerelease"].as_bool().unwrap_or(false),
                    commit: None,
                    assets: r["assets"]
                        .as_array()
                        .map(|x| {
                            x.iter()
                                .map(|x| RawAsset {
                                    name: x["name"].as_str().unwrap_or_default().to_string(),
                                    url: x["browser_download_url"].as_str().unwrap_or_default().to_string(),
                                    size: x["size"].as_u64().unwrap_or(0),
                                    sha256: None,
                                    updated_at: parse_time(x["created_at"].as_str()),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default())
}
