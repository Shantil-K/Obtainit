//! Obtainit updating itself from its own GitHub releases, using the same
//! engine it uses for every other app. It works out how it was installed
//! (AppImage, .deb, .rpm, Windows installer or MSI) and fetches the matching
//! prebuilt file from the newest release.

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::import_export::new_app;
use crate::model::{AppSettings, Asset, InstallKind, SourceKind};
use crate::ops::{progress, state};
use crate::{resolve, store, system};

pub const REPO_URL: &str = "https://github.com/Shantil-K/Obtainit";
const PROGRESS_ID: &str = "obtainit-self";

/// How this copy of Obtainit was installed.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    AppImage,
    Deb,
    Rpm,
    /// Windows NSIS setup (per-user install).
    Nsis,
    Msi,
    /// Unpacked somewhere by hand: updates can be downloaded, not installed.
    Portable,
    /// A development build.
    Dev,
}

#[derive(Serialize, Clone, Debug)]
pub struct Status {
    pub current: String,
    pub latest: Option<String>,
    pub available: bool,
    pub method: Method,
    pub can_install: bool,
    pub asset: Option<Asset>,
    pub notes: String,
    pub release_url: String,
    pub error: Option<String>,
    pub checked_at: Option<DateTime<Utc>>,
}

static STATUS: Mutex<Option<Status>> = Mutex::new(None);
static NOTIFIED: Mutex<Option<String>> = Mutex::new(None);

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn exe() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

pub fn method() -> Method {
    if cfg!(debug_assertions) {
        return Method::Dev;
    }
    if cfg!(windows) {
        let exe = exe().to_string_lossy().to_lowercase();
        let local = dirs::data_local_dir().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !local.is_empty() && exe.starts_with(&local) {
            return Method::Nsis;
        }
        if exe.contains("\\program files") {
            return Method::Msi;
        }
        return Method::Portable;
    }
    if std::env::var_os("APPIMAGE").is_some() {
        return Method::AppImage;
    }
    let path = exe().to_string_lossy().into_owned();
    if system::has("dpkg") && system::run_stdout("dpkg", &["-S", &path]).is_some() {
        return Method::Deb;
    }
    if system::has("rpm") && system::run_stdout("rpm", &["-qf", &path]).is_some() {
        return Method::Rpm;
    }
    Method::Portable
}

fn kind_for(m: &Method) -> Option<InstallKind> {
    Some(match m {
        Method::AppImage => InstallKind::AppImage,
        Method::Deb => InstallKind::Deb,
        Method::Rpm => InstallKind::Rpm,
        Method::Nsis => InstallKind::Installer,
        Method::Msi => InstallKind::Msi,
        Method::Portable | Method::Dev => return None,
    })
}

pub fn cached() -> Option<Status> {
    STATUS.lock().unwrap().clone()
}

/// Look for a newer release of Obtainit.
pub fn check(h: &AppHandle) -> Status {
    let m = method();
    let global = state(h).store.lock().unwrap().settings.clone();
    let kind = kind_for(&m);
    let settings = AppSettings {
        install_kind: kind
            .and_then(|k| serde_json::to_value(k).ok())
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "auto".into()),
        exe_mode: "installer".into(),
        ..Default::default()
    };
    let app = new_app(REPO_URL, SourceKind::Github, "Obtainit", settings);
    let mut st = Status {
        current: current_version().into(),
        latest: None,
        available: false,
        method: m.clone(),
        can_install: false,
        asset: None,
        notes: String::new(),
        release_url: format!("{REPO_URL}/releases"),
        error: None,
        checked_at: Some(Utc::now()),
    };
    match resolve::resolve(&app, &global) {
        Ok(latest) => {
            st.available = lifter_core::version::is_newer(&latest.version, current_version());
            st.asset = latest
                .selected
                .as_ref()
                .and_then(|n| latest.assets.iter().find(|a| &a.name == n))
                .cloned();
            st.can_install = st.available && kind.is_some() && st.asset.is_some();
            st.latest = Some(latest.version);
            st.notes = latest.notes;
            st.release_url = latest.release_url;
        }
        Err(e) => st.error = Some(format!("{e:#}")),
    }
    log::info!(
        "self-update: running {} ({:?}), latest {:?}",
        st.current,
        st.method,
        st.latest
    );
    *STATUS.lock().unwrap() = Some(st.clone());
    let _ = h.emit("self-update", &st);
    st
}

/// Notify once per new version.
pub fn should_notify(st: &Status) -> bool {
    let Some(v) = st.latest.clone().filter(|_| st.available) else { return false };
    let mut n = NOTIFIED.lock().unwrap();
    if n.as_deref() == Some(v.as_str()) {
        return false;
    }
    *n = Some(v);
    true
}

/// Start a detached shell command that outlives this process.
fn detached(script: &str) -> Result<()> {
    log::info!("self-update: handing over to: {script}");
    if cfg!(windows) {
        system::command("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-Command", script])
            .spawn()?;
    } else {
        std::process::Command::new("sh").args(["-c", script]).spawn()?;
    }
    Ok(())
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Download the new version, install it, and restart Obtainit.
pub fn apply(h: &AppHandle) -> Result<()> {
    let st = match cached() {
        Some(s) if s.available => s,
        _ => check(h),
    };
    if !st.available {
        bail!("Obtainit is already up to date");
    }
    let asset = st.asset.clone().context("this release has no file for your system")?;
    if !st.can_install {
        bail!("this copy of Obtainit can't update itself ({:?} install); download the new version from the releases page", st.method);
    }

    let dir = store::downloads_dir().join("obtainit-self");
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(&asset.name);
    let mut last = 0u64;
    let dl = lifter_core::http::download_to(
        &asset.url,
        &lifter_core::http::RequestOptions::browser(),
        &file,
        &mut |r, t| {
            if r == 0 || r - last >= 256 * 1024 || t == Some(r) {
                last = r;
                progress(h, PROGRESS_ID, "downloading", r, t, None);
            }
        },
    )?;
    if let Some(expected) = &asset.sha256 {
        if !expected.eq_ignore_ascii_case(&dl.sha256) {
            let _ = std::fs::remove_file(&file);
            bail!("checksum mismatch for {} — update aborted", asset.name);
        }
        log::info!("self-update: sha256 verified");
    } else {
        log::warn!("self-update: release has no published checksum; relying on HTTPS only");
    }
    progress(h, PROGRESS_ID, "installing", 0, None, None);

    // Come back the way we were: hidden in the tray, or with a window.
    let hidden = h
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .map(|v| !v)
        .unwrap_or(false);
    let flag = if hidden { " --hidden" } else { "" };
    let path = file.to_string_lossy().into_owned();

    match st.method {
        Method::AppImage => {
            let target = PathBuf::from(std::env::var_os("APPIMAGE").context("APPIMAGE not set")?);
            let tmp = target.with_extension("obtainit-new");
            std::fs::copy(&file, &tmp)?;
            lifter_core::archive::set_executable(&tmp)?;
            lifter_core::archive::replace_file(&tmp, &target)?;
            detached(&format!("sleep 1; exec {}{flag}", sh_quote(&target.to_string_lossy())))?;
        }
        Method::Deb => {
            if system::has("apt-get") {
                system::run_privileged("apt-get", &["install", "-y", "--allow-downgrades", &path])?;
            } else {
                system::run_privileged("dpkg", &["-i", &path])?;
            }
            detached(&format!("sleep 1; exec {}{flag}", sh_quote(&exe().to_string_lossy())))?;
        }
        Method::Rpm => {
            if system::has("dnf") {
                system::run_privileged("dnf", &["install", "-y", &path])?;
            } else if system::has("zypper") {
                system::run_privileged("zypper", &["--non-interactive", "install", "--allow-unsigned-rpm", &path])?;
            } else {
                system::run_privileged("rpm", &["-Uvh", &path])?;
            }
            detached(&format!("sleep 1; exec {}{flag}", sh_quote(&exe().to_string_lossy())))?;
        }
        Method::Nsis | Method::Msi => {
            // The installer needs Obtainit closed: wait for us to exit,
            // install silently, then start the new version.
            let exe = exe().to_string_lossy().into_owned();
            let install = if st.method == Method::Nsis {
                format!("Start-Process -Wait -FilePath {} -ArgumentList '/S'", system::ps_quote(&path))
            } else {
                format!(
                    "Start-Process -Wait -FilePath msiexec -ArgumentList '/i',{},'/passive','/norestart'",
                    system::ps_quote(&path)
                )
            };
            let relaunch = if hidden {
                format!("Start-Process -FilePath {} -ArgumentList '--hidden'", system::ps_quote(&exe))
            } else {
                format!("Start-Process -FilePath {}", system::ps_quote(&exe))
            };
            detached(&format!(
                "Wait-Process -Id {} -ErrorAction SilentlyContinue; Start-Sleep -Seconds 1; {install}; {relaunch}",
                std::process::id()
            ))?;
        }
        Method::Portable | Method::Dev => unreachable!(),
    }
    log::info!("self-update: installed {}, restarting", st.latest.unwrap_or_default());
    h.exit(0);
    Ok(())
}
