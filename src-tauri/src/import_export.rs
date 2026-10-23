//! Import from Obtainit backups, Obtainium exports, lifter.config files and
//! plain URL lists; export to Obtainit JSON or lifter.config.

use anyhow::{bail, Result};
use chrono::Utc;
use lifter_core::{assets, config};
use serde::Serialize;
use serde_json::Value;

use crate::model::{App, AppSettings, Installed, InstallKind, Settings, SourceKind};
use crate::sources;

#[derive(Serialize, Default)]
pub struct Report {
    pub format: String,
    pub added: Vec<App>,
    pub skipped: Vec<String>,
}

pub fn new_app(url: &str, source: SourceKind, name: &str, settings: AppSettings) -> App {
    App {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        author: String::new(),
        source,
        url: url.to_string(),
        icon_url: None,
        description: String::new(),
        categories: vec![],
        settings,
        installed: None,
        latest: None,
        last_error: None,
        last_checked: None,
        added_at: Utc::now(),
        favorite: false,
        notified_version: None,
    }
}

fn repo_name(url: &str) -> String {
    url.trim_end_matches('/').rsplit('/').next().unwrap_or(url).to_string()
}

fn github_icon(url: &str) -> Option<String> {
    let (_, segs) = sources::split_repo(url, 2).ok()?;
    url.contains("github.com").then(|| format!("https://github.com/{}.png?size=128", segs[0]))
}

/// Parse a file's text and return the apps it describes.
pub fn parse(text: &str, defaults: &Settings) -> Result<Report> {
    let t = text.trim_start_matches('\u{feff}').trim();
    // lifter.config also starts with "[", so only treat it as JSON if it parses.
    let json: Option<Value> = if t.starts_with('{') {
        Some(serde_json::from_str(t)?)
    } else if t.starts_with('[') {
        serde_json::from_str(t).ok()
    } else {
        None
    };
    if let Some(v) = json {
        if v.get("format").and_then(Value::as_str) == Some("obtainit") {
            return obtainit(&v);
        }
        if v.get("apps").is_some() || v.is_array() {
            return obtainium(&v, defaults);
        }
        bail!("this JSON file isn't an Obtainit or Obtainium export");
    }
    if t.lines().any(|l| l.trim_start().starts_with('[')) {
        return lifter(t, defaults);
    }
    urls(t, defaults)
}

fn obtainit(v: &Value) -> Result<Report> {
    let apps: Vec<App> = serde_json::from_value(v["apps"].clone())?;
    Ok(Report { format: "Obtainit".into(), added: apps, skipped: vec![] })
}

fn urls(t: &str, defaults: &Settings) -> Result<Report> {
    let mut r = Report { format: "URL list".into(), ..Default::default() };
    for line in t.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let url = sources::normalize_url(line);
        if url::Url::parse(&url).is_err() {
            r.skipped.push(format!("{line}: not a URL"));
            continue;
        }
        let kind = sources::guess_kind(&url);
        let s = AppSettings { include_prereleases: defaults.include_prereleases_by_default, ..Default::default() };
        let mut a = new_app(&url, kind, &repo_name(&url), s);
        a.icon_url = github_icon(&url);
        r.added.push(a);
    }
    Ok(r)
}

fn obtainium(v: &Value, defaults: &Settings) -> Result<Report> {
    let mut r = Report { format: "Obtainium".into(), ..Default::default() };
    let list = v.get("apps").unwrap_or(v).as_array().cloned().unwrap_or_default();
    for item in list {
        let url = item["url"].as_str().unwrap_or_default();
        let name = item["name"].as_str().unwrap_or_else(|| url);
        let kind = sources::guess_kind(&sources::normalize_url(url));
        if kind == SourceKind::Html || kind == SourceKind::Json {
            r.skipped.push(format!("{name}: {url} is an Android-only source"));
            continue;
        }
        let extra: Value = item["additionalSettings"]
            .as_str()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_else(|| item["additionalSettings"].clone());
        let b = |k: &str| extra[k].as_bool().unwrap_or(false);
        let st = |k: &str| extra[k].as_str().unwrap_or_default().to_string();
        let mut s = AppSettings {
            include_prereleases: b("includePrereleases") || defaults.include_prereleases_by_default,
            fallback_to_older_releases: extra["fallbackToOlderReleases"].as_bool().unwrap_or(true),
            verify_latest_tag: b("verifyLatestTag"),
            release_title_filter: st("filterReleaseTitlesByRegEx"),
            invert_title_filter: b("invertReleaseTitlesFilter"),
            release_notes_filter: st("filterReleaseNotesByRegEx"),
            invert_notes_filter: b("invertReleaseNotesFilter"),
            use_latest_asset_date: b("useLatestAssetDateAsReleaseDate"),
            version_extraction_regex: st("versionExtractionRegEx"),
            match_group: st("matchGroupToUse"),
            track_only: b("trackOnly"),
            exempt_from_background: b("exemptFromBackgroundUpdates"),
            skip_notifications: b("skipUpdateNotifications"),
            allow_insecure: b("allowInsecure"),
            min_update_age_days: extra["minUpdateAgeDays"].as_u64().unwrap_or(0) as u32,
            ..Default::default()
        };
        s.sort_method = match st("sortMethodChoice").as_str() {
            "none" => "none".into(),
            "date" => "date".into(),
            "" => "date".into(),
            _ => "version".into(),
        };
        if b("releaseTitleAsVersion") {
            s.version_source = "title".into();
        }
        if b("releaseDateAsVersion") {
            s.version_source = "date".into();
        }
        let url = sources::normalize_url(url);
        let mut a = new_app(&url, kind, name, s);
        a.author = item["author"].as_str().unwrap_or_default().to_string();
        a.categories = item["categories"]
            .as_array()
            .map(|c| c.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        a.icon_url = github_icon(&url);
        r.added.push(a);
    }
    Ok(r)
}

fn lifter(t: &str, defaults: &Settings) -> Result<Report> {
    let mut r = Report { format: "lifter.config".into(), ..Default::default() };
    let platform = assets::Platform::current();
    for sec in config::parse(t)? {
        // lifter configs often list the same tool for several OSes; only
        // keep the ones meant for this machine (lifter's own scoring).
        let probe = sec.anchor_text.replace('\\', "");
        if !probe.is_empty() && assets::asset_score(&probe, platform) < 0 {
            r.skipped.push(format!("[{}]: built for another system", sec.name));
            continue;
        }
        let mut s = AppSettings {
            include_prereleases: defaults.include_prereleases_by_default,
            extract_targets: sec.targets.clone(),
            desired_filename: sec.desired_filename.clone(),
            ..Default::default()
        };
        if !sec.targets.is_empty() {
            s.archive_mode = "binaries".into();
        }
        let gh_project = if !sec.project.is_empty() && sec.page_url.contains("api.github.com/repos/") {
            Some(sec.project.clone())
        } else {
            sec.page_url
                .strip_prefix("https://api.github.com/repos/")
                .and_then(|rest| {
                    let p: Vec<&str> = rest.split('/').collect();
                    (p.len() >= 2).then(|| format!("{}/{}", p[0], p[1]))
                })
        };
        let (url, kind) = if let Some(p) = gh_project {
            s.asset_filter = sec.anchor_text.clone();
            (format!("https://github.com/{p}"), SourceKind::Github)
        } else if sec.method == "api_json" {
            s.asset_filter = sec.anchor_text.clone();
            s.json.version_path = sec.version_tag.clone();
            s.json.commit_path = sec.commit_tag.clone();
            s.json.urls_path = sec.anchor_tag.clone();
            (sec.page_url.clone(), SourceKind::Json)
        } else {
            s.html.link_selector = sec.anchor_tag.clone();
            s.html.link_text_regex = sec.anchor_text.clone();
            s.html.version_selector = sec.version_tag.clone();
            (sec.page_url.clone(), SourceKind::Html)
        };
        if !sec.commit.is_empty() {
            s.version_compare = "different".into();
        }
        let mut a = new_app(&url, kind, &sec.name, s);
        a.icon_url = github_icon(&url);
        if !sec.version.is_empty() {
            // lifter already put this version somewhere; track it until
            // Obtainit installs a newer one itself.
            a.installed = Some(Installed {
                version: sec.version.clone(),
                commit: (!sec.commit.is_empty()).then(|| sec.commit.clone()),
                kind: InstallKind::Binary,
                asset_name: String::new(),
                paths: vec![],
                executable: None,
                package_name: None,
                desktop_entry: None,
                installed_at: Utc::now(),
                sha256: None,
                manual: true,
            });
        }
        r.added.push(a);
    }
    Ok(r)
}

pub fn export_obtainit(apps: &[App], settings: &Settings) -> String {
    let mut s = settings.clone();
    s.github_token.clear();
    s.gitlab_token.clear();
    s.forgejo_token.clear();
    serde_json::to_string_pretty(&serde_json::json!({
        "format": "obtainit",
        "version": 1,
        "exported_at": Utc::now(),
        "apps": apps,
        "settings": s,
    }))
    .unwrap_or_default()
}

pub fn export_lifter(apps: &[App]) -> String {
    let sections: Vec<config::Section> = apps
        .iter()
        .filter_map(|a| {
            let s = &a.settings;
            let latest = a.latest.as_ref();
            // The asset regex lifter matches: the app's filter, or one built
            // from the currently chosen file (lifter's own generator).
            let anchor = if !s.asset_filter.is_empty() {
                s.asset_filter.clone()
            } else {
                latest
                    .and_then(|l| l.selected.as_ref().map(|n| assets::anchor_regex_for_asset(n, &l.tag)))
                    .unwrap_or_default()
            };
            let version = a.installed.as_ref().map(|i| i.version.clone()).unwrap_or_default();
            let mut sec = config::Section {
                name: a.name.clone(),
                anchor_text: anchor,
                version,
                targets: s.extract_targets.clone(),
                desired_filename: s.desired_filename.clone(),
                ..Default::default()
            };
            match a.source {
                SourceKind::Github => {
                    let (_, segs) = sources::split_repo(&a.url, 2).ok()?;
                    sec.template = "github_api_latest".into();
                    sec.project = format!("{}/{}", segs[0], segs[1]);
                }
                SourceKind::Forgejo => {
                    let (origin, segs) = sources::split_repo(&a.url, 2).ok()?;
                    sec.method = "api_json".into();
                    sec.page_url = format!("{origin}/api/v1/repos/{}/{}/releases/latest", segs[0], segs[1]);
                    sec.version_tag = "$.tag_name".into();
                    sec.anchor_tag = "$.assets.*.browser_download_url".into();
                }
                SourceKind::Gitlab => {
                    let (origin, segs) = sources::split_repo(&a.url, 2).ok()?;
                    let path: Vec<&str> = segs.iter().map(|s| s.as_str()).take_while(|s| *s != "-").collect();
                    sec.method = "api_json".into();
                    sec.page_url = format!("{origin}/api/v4/projects/{}/releases/permalink/latest", path.join("%2F"));
                    sec.version_tag = "$.tag_name".into();
                    sec.anchor_tag = "$.assets.links.*.direct_asset_url".into();
                }
                SourceKind::Json => {
                    sec.method = "api_json".into();
                    sec.page_url = a.url.clone();
                    sec.version_tag = s.json.version_path.clone();
                    sec.commit_tag = s.json.commit_path.clone();
                    sec.anchor_tag = s.json.urls_path.clone();
                }
                SourceKind::Html => {
                    sec.method = "html".into();
                    sec.page_url = a.url.clone();
                    sec.anchor_tag = s.html.link_selector.clone();
                    sec.anchor_text = s.html.link_text_regex.clone();
                    sec.version_tag = s.html.version_selector.clone();
                }
            }
            Some(sec)
        })
        .collect();
    config::render(&sections)
}
