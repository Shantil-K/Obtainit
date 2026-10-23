use anyhow::{Context, Result};
use lifter_core::http::{self, RequestOptions};
use serde_json::Value;

use super::{parse_time, split_repo, Meta};
use crate::model::{App, RawAsset, Release, Settings};

fn opts(s: &Settings) -> RequestOptions {
    let o = RequestOptions::default();
    if s.gitlab_token.trim().is_empty() {
        o
    } else {
        o.header("PRIVATE-TOKEN", s.gitlab_token.trim())
    }
}

/// (origin, url-encoded project path). Stops at GitLab's `/-/` separator.
fn project_of(url: &str) -> Result<(String, String, String)> {
    let (origin, segs) = split_repo(url, 2)?;
    let path: Vec<&str> = segs.iter().map(|s| s.as_str()).take_while(|s| *s != "-").collect();
    let path = path.join("/");
    Ok((origin, path.replace('/', "%2F"), path))
}

pub fn metadata(url: &str, s: &Settings) -> Result<Meta> {
    let (origin, enc, path) = project_of(url)?;
    let v = http::get_json(&format!("{origin}/api/v4/projects/{enc}"), &opts(s))
        .with_context(|| format!("looking up GitLab project {path}"))?;
    Ok(Meta {
        name: v["name"].as_str().unwrap_or_default().to_string(),
        author: v["namespace"]["name"].as_str().unwrap_or_default().to_string(),
        description: v["description"].as_str().unwrap_or_default().to_string(),
        icon_url: v["avatar_url"]
            .as_str()
            .or(v["namespace"]["avatar_url"].as_str())
            .map(|a| if a.starts_with('/') { format!("{origin}{a}") } else { a.to_string() }),
        url: v["web_url"].as_str().map(String::from).unwrap_or(format!("{origin}/{path}")),
    })
}

pub fn releases(app: &App, s: &Settings) -> Result<Vec<Release>> {
    let (origin, enc, path) = project_of(&app.url)?;
    let list = http::get_json(
        &format!("{origin}/api/v4/projects/{enc}/releases?per_page=100"),
        &opts(s),
    )?;
    let out = list
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|r| !r["upcoming_release"].as_bool().unwrap_or(false))
                .map(|r: &Value| {
                    let assets = r["assets"]["links"]
                        .as_array()
                        .map(|l| {
                            l.iter()
                                .map(|x| RawAsset {
                                    name: x["name"].as_str().unwrap_or_default().to_string(),
                                    url: x["direct_asset_url"]
                                        .as_str()
                                        .or(x["url"].as_str())
                                        .unwrap_or_default()
                                        .to_string(),
                                    ..Default::default()
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let tag = r["tag_name"].as_str().unwrap_or_default().to_string();
                    Release {
                        title: r["name"].as_str().unwrap_or_default().to_string(),
                        notes: r["description"].as_str().unwrap_or_default().to_string(),
                        url: r["_links"]["self"]
                            .as_str()
                            .map(String::from)
                            .unwrap_or(format!("{origin}/{path}/-/releases/{tag}")),
                        published_at: parse_time(r["released_at"].as_str().or(r["created_at"].as_str())),
                        prerelease: false,
                        commit: r["commit"]["id"].as_str().map(String::from),
                        tag,
                        assets,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(out)
}
