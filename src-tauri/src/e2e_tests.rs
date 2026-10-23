//! Network tests against real releases. Run with:
//!   cargo test -p obtainit -- --ignored --nocapture

use std::path::Path;

use crate::import_export::new_app;
use crate::install;
use crate::model::{AppSettings, Settings, SourceKind};
use crate::resolve;

fn settings_in(dir: &Path) -> Settings {
    Settings {
        bin_dir: dir.join("bin").to_string_lossy().into_owned(),
        apps_dir: dir.join("apps").to_string_lossy().into_owned(),
        github_token: std::env::var("GITHUB_TOKEN").unwrap_or_default(),
        ..Default::default()
    }
}

/// Resolve, download and install; returns the install record.
fn install_from(url: &str, source: SourceKind, mut s: AppSettings, dir: &Path) -> crate::model::Installed {
    s.create_shortcut = false;
    let global = settings_in(dir);
    let name = url.trim_end_matches('/').rsplit('/').next().unwrap();
    let app = new_app(url, source, name, s);
    let latest = resolve::resolve(&app, &global).expect("resolve");
    println!("{url}: version {} (tag {})", latest.version, latest.tag);
    for a in latest.assets.iter().take(6) {
        println!("  {:>5} {:<9?} {} {}", a.score, a.kind, if a.compatible { "ok " } else { "   " }, a.name);
    }
    let sel = latest.selected.clone().expect("a file should be selected");
    let asset = latest.assets.iter().find(|a| a.name == sel).unwrap();
    let file = dir.join("dl").join(&asset.name);
    let dl = lifter_core::http::download_to(
        &asset.url,
        &lifter_core::http::RequestOptions::browser(),
        &file,
        &mut |_, _| {},
    )
    .expect("download");
    if let Some(sha) = &asset.sha256 {
        assert_eq!(sha, &dl.sha256, "checksum");
        println!("  sha256 verified");
    }
    let ctx = install::Ctx { app: &app, asset, file: &file, settings: &global, icon: None };
    let inst = install::install(&ctx, None).expect("install");
    println!("  installed as {:?}: {:?} exe={:?}", inst.kind, inst.paths, inst.executable);
    inst
}

fn runs(exe: &str, arg: &str) -> String {
    let out = std::process::Command::new(exe).arg(arg).output().expect("run");
    String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr)
}

#[test]
#[ignore]
fn ripgrep_archive_to_single_binary() {
    let d = tempfile::tempdir().unwrap();
    let inst = install_from("https://github.com/BurntSushi/ripgrep", SourceKind::Github, AppSettings::default(), d.path());
    let exe = inst.executable.unwrap();
    assert!(exe.ends_with("rg") || exe.ends_with("rg.exe"), "{exe}");
    assert!(runs(&exe, "--version").contains("ripgrep"));
}

#[test]
#[ignore]
#[cfg(target_os = "linux")]
fn appimage_install() {
    let d = tempfile::tempdir().unwrap();
    let s = AppSettings { include_prereleases: true, ..Default::default() };
    let inst = install_from("https://github.com/AppImage/appimagetool", SourceKind::Github, s, d.path());
    assert_eq!(inst.kind, crate::model::InstallKind::AppImage);
    let exe = inst.executable.unwrap();
    use std::os::unix::fs::PermissionsExt;
    assert!(std::fs::metadata(&exe).unwrap().permissions().mode() & 0o111 != 0);
}

#[test]
#[ignore]
#[cfg(target_os = "linux")]
fn neovim_as_folder() {
    let d = tempfile::tempdir().unwrap();
    let s = AppSettings {
        install_kind: "archive".into(),
        asset_filter: r"linux.*\.tar\.gz$".into(),
        ..Default::default()
    };
    let inst = install_from("https://github.com/neovim/neovim", SourceKind::Github, s, d.path());
    let exe = inst.executable.expect("main program found");
    assert!(exe.ends_with("bin/nvim"), "{exe}");
    assert!(runs(&exe, "--version").contains("NVIM"));
}

#[test]
#[ignore]
fn html_scrape_7zip() {
    let mut s = AppSettings::default();
    s.html.link_text_regex = r"7z(\d+)-linux-x64\.tar\.xz".into();
    s.auto_filter_platform = false;
    let app = new_app("https://www.7-zip.org/download.html", SourceKind::Html, "7-Zip", s);
    let latest = resolve::resolve(&app, &Settings::default()).expect("resolve");
    println!("7-zip version {} -> {:?}", latest.version, latest.selected);
    for a in &latest.assets {
        println!("  {}", a.url);
    }
    assert!(latest.version.chars().all(|c| c.is_ascii_digit()));
    assert!(latest.assets.iter().all(|a| a.url.starts_with("https://") && a.name.ends_with("-linux-x64.tar.xz")));
}

#[test]
fn imports_lifter_sample_config() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../lifter/lifter.config"));
    let Ok(text) = text else { return };
    let r = crate::import_export::parse(&text, &Settings::default()).unwrap();
    println!("lifter.config: {} added, {} skipped", r.added.len(), r.skipped.len());
    assert!(r.added.len() > 5);
    assert!(r.added.iter().any(|a| a.url == "https://github.com/BurntSushi/ripgrep"));
    // The Windows twin of each Linux tool is skipped on Linux.
    if cfg!(target_os = "linux") {
        assert!(r.skipped.iter().any(|s| s.contains("ripgrep Windows")));
    }
}
