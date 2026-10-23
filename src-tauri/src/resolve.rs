//! Turn a source's release list into "the version to install and the file
//! to download", applying the app's options (Obtainium's filters) and
//! lifter's platform scoring.

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use lifter_core::{archive, assets, version};

use crate::model::{App, AppSettings, Asset, InstallKind, Latest, Release, Settings};
use crate::{sources, system};

/// What kind of install a file name implies.
pub fn detect_kind(name: &str, settings: &AppSettings) -> InstallKind {
    let l = name.to_lowercase();
    use InstallKind::*;
    if l.ends_with(".appimage") {
        return AppImage;
    }
    if l.ends_with(".deb") {
        return Deb;
    }
    if l.ends_with(".rpm") {
        return Rpm;
    }
    if l.ends_with(".pkg.tar.zst") || l.ends_with(".pkg.tar.xz") {
        return Pacman;
    }
    if l.ends_with(".flatpak") || l.ends_with(".flatpakref") {
        return Flatpak;
    }
    if l.ends_with(".snap") {
        return Snap;
    }
    if l.ends_with(".msi") {
        return Msi;
    }
    if [".msix", ".msixbundle", ".appx", ".appxbundle"].iter().any(|e| l.ends_with(e)) {
        return Msix;
    }
    if l.ends_with(".exe") {
        return match settings.exe_mode.as_str() {
            "portable" => Binary,
            "installer" => Installer,
            _ => {
                if ["setup", "install"].iter().any(|w| l.contains(w)) {
                    Installer
                } else if l.contains("portable")
                    || ["pc-windows", "-msvc", "-gnu.", "windows_amd64", "windows-amd64", "windows_x86_64", "windows-x86_64", "windows_arm64", "win64.exe", "win32.exe"]
                        .iter()
                        .any(|w| l.contains(w))
                {
                    Binary
                } else {
                    Installer
                }
            }
        };
    }
    if l.ends_with(".com") {
        return Binary;
    }
    if archive::detect_kind(&l).is_some() {
        return Archive;
    }
    if [".dmg", ".pkg", ".apk", ".aab", ".ipa"].iter().any(|e| l.ends_with(e)) {
        return Unknown;
    }
    // lifter: no dot in the last 8 characters means a bare binary.
    let tail: String = l.chars().rev().take(8).collect();
    if !tail.contains('.') {
        return Binary;
    }
    Unknown
}

fn regex_or_err(src: &str, what: &str) -> Result<Option<regex::Regex>> {
    if src.trim().is_empty() {
        return Ok(None);
    }
    regex::Regex::new(src)
        .map(Some)
        .with_context(|| format!("invalid {what} regex {src:?}"))
}

/// Score and filter a release's assets for this machine. Returns all
/// non-auxiliary assets, best first, with `compatible` set.
pub fn rank_assets(release: &Release, app: &AppSettings, global: &Settings) -> Result<Vec<Asset>> {
    let filter = regex_or_err(&app.asset_filter, "asset filter")?;
    let platform = assets::Platform::current();
    let supported = system::supported_kinds();
    let forced: Option<InstallKind> = match app.install_kind.as_str() {
        "" | "auto" => None,
        k => serde_json::from_value(serde_json::Value::String(k.into())).ok(),
    };
    let n = global.kind_priority.len() as i32;

    let mut out: Vec<(i32, Asset)> = release
        .assets
        .iter()
        .filter(|a| !a.name.is_empty() && !assets::is_auxiliary_asset(&a.name))
        .map(|a| {
            let kind = detect_kind(&a.name, app);
            let score = assets::asset_score(&a.name, platform);
            let passes_filter = match &filter {
                Some(re) => re.is_match(&a.name) != app.invert_asset_filter,
                None => true,
            };
            let kind_ok = kind != InstallKind::Unknown
                && supported.contains(&kind)
                && forced.is_none_or(|f| f == kind);
            let platform_ok = !app.auto_filter_platform || score > 0;
            let compatible = passes_filter && kind_ok && platform_ok;
            let prio = global
                .kind_priority
                .iter()
                .position(|k| *k == kind)
                .map(|i| (n - i as i32) * 4)
                .unwrap_or(0);
            let rank = if compatible { 1000 } else { 0 } + score + prio;
            (
                rank,
                Asset {
                    name: a.name.clone(),
                    url: a.url.clone(),
                    size: a.size,
                    sha256: a.sha256.clone(),
                    updated_at: a.updated_at,
                    kind,
                    score,
                    compatible,
                },
            )
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(out.into_iter().map(|(_, a)| a).collect())
}

fn matches_filter(text: &str, re: &Option<regex::Regex>, invert: bool) -> bool {
    match re {
        Some(re) => re.is_match(text) != invert,
        None => true,
    }
}

fn release_date(r: &Release, s: &AppSettings) -> Option<chrono::DateTime<Utc>> {
    if s.use_latest_asset_date {
        r.assets.iter().filter_map(|a| a.updated_at).max().or(r.published_at)
    } else {
        r.published_at
    }
}

fn version_of(r: &Release, s: &AppSettings) -> Option<String> {
    let raw = match s.version_source.as_str() {
        "title" if !r.title.trim().is_empty() => r.title.trim().to_string(),
        "date" => release_date(r, s)
            .map(|d| d.format("%Y.%m.%d.%H%M").to_string())
            .unwrap_or_else(|| r.tag.clone()),
        _ => r.tag.clone(),
    };
    version::extract(&raw, &s.version_extraction_regex, &s.match_group)
}

/// Fetch and resolve the latest installable release for `app`.
pub fn resolve(app: &App, global: &Settings) -> Result<Latest> {
    let s = &app.settings;
    let mut list = sources::releases(app, global)?;
    let title_re = regex_or_err(&s.release_title_filter, "release title filter")?;
    let notes_re = regex_or_err(&s.release_notes_filter, "release notes filter")?;
    let verified_first = s.verify_latest_tag && app.source == crate::model::SourceKind::Github;

    list.retain(|r| {
        (s.include_prereleases || !r.prerelease)
            && matches_filter(&r.title, &title_re, s.invert_title_filter)
            && matches_filter(&r.notes, &notes_re, s.invert_notes_filter)
    });
    if list.is_empty() {
        return Err(anyhow!("no release matched this app's filters"));
    }

    if !verified_first {
        match s.sort_method.as_str() {
            "version" => list.sort_by(|a, b| {
                version::compare(
                    &version_of(b, s).unwrap_or_default(),
                    &version_of(a, s).unwrap_or_default(),
                )
            }),
            "none" => {}
            _ => list.sort_by(|a, b| release_date(b, s).cmp(&release_date(a, s))),
        }
    }

    // Minimum age: skip releases that are too fresh.
    let mut held_back = None;
    if s.min_update_age_days > 0 {
        let cutoff = Utc::now() - Duration::days(s.min_update_age_days as i64);
        let before = list.len();
        let first = list.first().map(|r| r.tag.clone());
        list.retain(|r| release_date(r, s).is_none_or(|d| d <= cutoff));
        if list.len() < before {
            held_back = first;
        }
        if list.is_empty() {
            return Err(anyhow!(
                "every matching release is newer than {} days",
                s.min_update_age_days
            ));
        }
    }

    // Newest release wins. With "fallback to older releases", walk back to
    // the newest one that has a file for this machine.
    let candidates: Vec<&Release> = list.iter().filter(|r| version_of(r, s).is_some()).collect();
    let first = *candidates
        .first()
        .ok_or_else(|| anyhow!("the version extraction regex matched no release"))?;
    let mut pick = None;
    if s.fallback_to_older_releases && !s.track_only {
        for r in &candidates {
            let ranked = rank_assets(r, s, global)?;
            if ranked.iter().any(|a| a.compatible) {
                pick = Some((*r, ranked));
                break;
            }
        }
    }
    let (r, ranked) = match pick {
        Some(p) => p,
        None => (first, rank_assets(first, s, global)?),
    };

    let selected = ranked.iter().find(|a| a.compatible).map(|a| a.name.clone());
    Ok(Latest {
        version: version_of(r, s).unwrap_or_else(|| r.tag.clone()),
        commit: r.commit.clone(),
        tag: r.tag.clone(),
        title: r.title.clone(),
        notes: r.notes.clone(),
        release_url: r.url.clone(),
        published_at: release_date(r, s),
        prerelease: r.prerelease,
        assets: ranked,
        selected,
        held_back,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        let s = AppSettings::default();
        assert_eq!(detect_kind("App-1.0.AppImage", &s), InstallKind::AppImage);
        assert_eq!(detect_kind("tool-x86_64-unknown-linux-musl.tar.gz", &s), InstallKind::Archive);
        assert_eq!(detect_kind("App-Setup-1.0.exe", &s), InstallKind::Installer);
        assert_eq!(detect_kind("tokei-x86_64-pc-windows-msvc.exe", &s), InstallKind::Binary);
        assert_eq!(detect_kind("dictomatic", &s), InstallKind::Binary);
        assert_eq!(detect_kind("app.pkg.tar.zst", &s), InstallKind::Pacman);
        assert_eq!(detect_kind("app.dmg", &s), InstallKind::Unknown);
    }
}
