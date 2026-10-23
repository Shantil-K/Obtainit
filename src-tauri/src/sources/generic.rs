//! lifter's generic methods: scrape an HTML page or query a JSON endpoint.

use anyhow::{bail, Result};
use lifter_core::scrape::{self, HtmlQuery, JsonQuery};
use url::Url;

use super::{request_opts, Meta};
use crate::model::{App, RawAsset, Release, Settings};

pub fn metadata(url: &str) -> Meta {
    let host = Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string()))
        .unwrap_or_default();
    let name = Url::parse(url)
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|s| s.filter(|x| !x.is_empty()).map(String::from).last())
        })
        .map(|s| s.split('.').next().unwrap_or(&s).to_string())
        .filter(|s| !s.is_empty() && s != "index" && s != "releases" && s != "download" && s != "downloads")
        .unwrap_or_else(|| host.split('.').next().unwrap_or("app").to_string());
    Meta {
        name,
        author: host.clone(),
        description: String::new(),
        icon_url: Url::parse(url)
            .ok()
            .map(|u| format!("{}/favicon.ico", u.origin().ascii_serialization())),
        url: url.to_string(),
    }
}

fn to_release(app: &App, hit: scrape::Hit) -> Result<Vec<Release>> {
    if hit.links.is_empty() {
        bail!("no download links matched on {}", app.url);
    }
    // Without a version on the page, fall back to the file name of the first
    // match so a changed link still counts as an update.
    let version = hit
        .version
        .clone()
        .or_else(|| hit.links.first().map(|l| scrape::file_name_of(&l.url)))
        .unwrap_or_default();
    Ok(vec![Release {
        tag: version.clone(),
        title: version,
        notes: String::new(),
        url: app.url.clone(),
        published_at: None,
        prerelease: false,
        commit: hit.commit,
        assets: hit
            .links
            .into_iter()
            .map(|l| RawAsset {
                name: scrape::file_name_of(&l.url),
                url: l.url,
                ..Default::default()
            })
            .collect(),
    }])
}

pub fn html_release(app: &App, s: &Settings) -> Result<Vec<Release>> {
    let h = &app.settings.html;
    let q = HtmlQuery {
        link_selector: h.link_selector.clone(),
        link_text_regex: h.link_text_regex.clone(),
        version_selector: h.version_selector.clone(),
    };
    let hit = scrape::fetch_html(&app.url, &q, &request_opts(app, s))?;
    to_release(app, hit)
}

pub fn json_release(app: &App, s: &Settings) -> Result<Vec<Release>> {
    let j = &app.settings.json;
    if j.urls_path.trim().is_empty() {
        bail!("set the JSONPath for download URLs in this app's settings");
    }
    let q = JsonQuery {
        version_path: j.version_path.clone(),
        commit_path: j.commit_path.clone(),
        urls_path: j.urls_path.clone(),
        url_regex: String::new(),
    };
    let hit = scrape::fetch_json(&app.url, &q, &request_opts(app, s))?;
    to_release(app, hit)
}
