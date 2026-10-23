//! Core operations shared by UI commands and the background scheduler.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::install;
use crate::model::{App, InstallKind, Installed, Settings};
use crate::resolve;
use crate::store::{self, Store};

pub struct State {
    pub store: Mutex<Store>,
    pub busy: Mutex<HashSet<String>>,
}

#[derive(Serialize, Clone)]
pub struct Progress<'a> {
    pub id: &'a str,
    /// checking | downloading | verifying | installing | removing | done | error
    pub stage: &'a str,
    pub received: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

pub fn progress(h: &AppHandle, id: &str, stage: &str, received: u64, total: Option<u64>, message: Option<String>) {
    let _ = h.emit("progress", Progress { id, stage, received, total, message });
}

pub fn state(h: &AppHandle) -> tauri::State<'_, State> {
    h.state::<State>()
}

/// Snapshot of an app and the settings, taken without holding the lock
/// during network work.
pub fn snapshot(h: &AppHandle, id: &str) -> Result<(App, Settings)> {
    let st = state(h);
    let s = st.store.lock().unwrap();
    let app = s.get(id).cloned().ok_or_else(|| anyhow!("app not found"))?;
    Ok((app, s.settings.clone()))
}

/// Apply a change to a stored app, save, and tell the UI.
pub fn modify(h: &AppHandle, id: &str, f: impl FnOnce(&mut App)) -> Result<App> {
    let st = state(h);
    let mut s = st.store.lock().unwrap();
    let app = s.get_mut(id).ok_or_else(|| anyhow!("app not found"))?;
    f(app);
    let out = app.clone();
    s.save_apps();
    drop(s);
    let _ = h.emit("app-changed", &out);
    Ok(out)
}

/// Marks an app busy for the life of the guard.
pub struct Busy<'a> {
    h: &'a AppHandle,
    id: String,
}

impl<'a> Busy<'a> {
    pub fn take(h: &'a AppHandle, id: &str) -> Result<Self> {
        let st = state(h);
        let mut b = st.busy.lock().unwrap();
        if !b.insert(id.to_string()) {
            bail!("this app is already being worked on");
        }
        Ok(Busy { h, id: id.to_string() })
    }
}

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        state(self.h).busy.lock().unwrap().remove(&self.id);
    }
}

/// Check one app for its latest release.
pub fn check(h: &AppHandle, id: &str) -> Result<App> {
    let (app, settings) = snapshot(h, id)?;
    progress(h, id, "checking", 0, None, None);
    let result = resolve::resolve(&app, &settings);
    // System packages may have been updated outside Obtainit.
    let pkg_version = app.installed.as_ref().filter(|i| !i.manual).and_then(|i| match i.kind {
        InstallKind::Deb | InstallKind::Rpm | InstallKind::Pacman | InstallKind::Installer | InstallKind::Msi => {
            install::package_version_of(&app, i)
        }
        _ => None,
    });
    let out = modify(h, id, |a| {
        a.last_checked = Some(Utc::now());
        match &result {
            Ok(latest) => {
                a.latest = Some(latest.clone());
                a.last_error = None;
            }
            Err(e) => a.last_error = Some(format!("{e:#}")),
        }
        if let (Some(v), Some(inst)) = (pkg_version, a.installed.as_mut()) {
            // Package versions look like 1.2.3-1; keep ours if it matches.
            if !v.starts_with(lifter_core::version::normalize(&inst.version).as_str()) {
                if let Some(l) = &a.latest {
                    if v.starts_with(lifter_core::version::normalize(&l.version).as_str()) {
                        inst.version = l.version.clone();
                    }
                }
            }
        }
    })?;
    match &result {
        Ok(_) => progress(h, id, "done", 0, None, None),
        Err(e) => {
            log::warn!("[{}] check failed: {e:#}", app.name);
            progress(h, id, "error", 0, None, Some(format!("{e:#}")));
        }
    }
    Ok(out)
}

/// Check several apps in parallel.
pub fn check_many(h: &AppHandle, ids: Vec<String>) -> Vec<App> {
    let parallel = state(h).store.lock().unwrap().settings.parallel_checks.clamp(1, 16) as usize;
    let queue = Mutex::new(ids.into_iter());
    let results = Mutex::new(Vec::new());
    std::thread::scope(|sc| {
        for _ in 0..parallel {
            sc.spawn(|| loop {
                let next = queue.lock().unwrap().next();
                let Some(id) = next else { break };
                if let Ok(a) = check(h, &id) {
                    results.lock().unwrap().push(a);
                }
            });
        }
    });
    results.into_inner().unwrap()
}

fn icon_path(app: &App) -> Option<PathBuf> {
    let url = app.icon_url.as_ref()?;
    let dir = store::icons_dir();
    let ext = if url.contains(".ico") { "ico" } else if url.contains(".svg") { "svg" } else { "png" };
    let dest = dir.join(format!("{}.{ext}", app.id));
    if dest.exists() {
        return Some(dest);
    }
    let _ = std::fs::create_dir_all(&dir);
    let opts = lifter_core::http::RequestOptions { max_attempts: 1, ..Default::default() };
    lifter_core::http::download_to(url, &opts, &dest, &mut |_, _| {}).ok()?;
    Some(dest)
}

/// Download and install the latest (or a chosen) asset.
pub fn install(h: &AppHandle, id: &str, asset_name: Option<String>) -> Result<App> {
    let _busy = Busy::take(h, id)?;
    let (mut app, settings) = snapshot(h, id)?;
    if app.latest.is_none() {
        app = check(h, id)?;
    }
    let latest = app.latest.clone().ok_or_else(|| anyhow!("no release information yet"))?;
    let name = asset_name
        .or(latest.selected.clone())
        .ok_or_else(|| anyhow!("no file in the latest release suits this system; check the app's filters"))?;
    let asset = latest
        .assets
        .iter()
        .find(|a| a.name == name)
        .cloned()
        .ok_or_else(|| anyhow!("{name} is not in the latest release"))?;

    let res = (|| -> Result<Installed> {
        let dir = store::downloads_dir().join(&app.id);
        std::fs::create_dir_all(&dir)?;
        let file = dir.join(&asset.name);
        let opts = crate::sources::request_opts(&app, &settings);
        let opts = lifter_core::http::RequestOptions {
            headers: vec![("User-Agent".into(), lifter_core::http::BROWSER_UA.into())],
            ..opts
        };
        log::info!("[{}] downloading {}", app.name, asset.url);
        let mut last = 0u64;
        let dl = lifter_core::http::download_to(&asset.url, &opts, &file, &mut |r, t| {
            if r == 0 || r - last >= 256 * 1024 || t == Some(r) {
                last = r;
                progress(h, id, "downloading", r, t.or(Some(asset.size).filter(|s| *s > 0)), None);
            }
        })?;
        if app.settings.verify_checksum {
            if let Some(expected) = &asset.sha256 {
                progress(h, id, "verifying", dl.bytes, Some(dl.bytes), None);
                if !expected.eq_ignore_ascii_case(&dl.sha256) {
                    let _ = std::fs::remove_file(&file);
                    bail!("checksum mismatch for {} (expected {expected}, got {})", asset.name, dl.sha256);
                }
                log::info!("[{}] sha256 verified", app.name);
            }
        }
        progress(h, id, "installing", 0, None, None);
        let icon = if app.settings.create_shortcut { icon_path(&app) } else { None };
        let ctx = install::Ctx { app: &app, asset: &asset, file: &file, settings: &settings, icon };
        let mut inst = install::install(&ctx, app.installed.as_ref())?;
        inst.version = latest.version.clone();
        inst.commit = latest.commit.clone();
        inst.sha256 = Some(dl.sha256.clone());
        if !settings.keep_downloads {
            let _ = std::fs::remove_dir_all(&dir);
        }
        let post = app.settings.post_update_command.trim();
        if !post.is_empty() {
            let env = [
                ("OBTAINIT_APP", app.name.clone()),
                ("OBTAINIT_VERSION", inst.version.clone()),
                ("OBTAINIT_PATHS", inst.paths.join(if cfg!(windows) { ";" } else { ":" })),
                ("OBTAINIT_EXECUTABLE", inst.executable.clone().unwrap_or_default()),
            ];
            if let Err(e) = install::spawn_shell(post, &env) {
                log::warn!("[{}] post-update command failed: {e:#}", app.name);
            }
        }
        Ok(inst)
    })();

    match res {
        Ok(inst) => {
            log::info!("[{}] installed {}", app.name, inst.version);
            progress(h, id, "done", 0, None, Some(format!("Installed {}", inst.version)));
            modify(h, id, |a| {
                a.installed = Some(inst);
                a.last_error = None;
            })
        }
        Err(e) => {
            log::error!("[{}] install failed: {e:#}", app.name);
            progress(h, id, "error", 0, None, Some(format!("{e:#}")));
            Err(e)
        }
    }
}

pub fn uninstall(h: &AppHandle, id: &str) -> Result<App> {
    let _busy = Busy::take(h, id)?;
    let (app, settings) = snapshot(h, id)?;
    let inst = app.installed.clone().context("not installed")?;
    progress(h, id, "removing", 0, None, None);
    if let Err(e) = install::uninstall(&app, &inst, &settings) {
        progress(h, id, "error", 0, None, Some(format!("{e:#}")));
        return Err(e);
    }
    progress(h, id, "done", 0, None, Some("Uninstalled".into()));
    modify(h, id, |a| a.installed = None)
}

/// Apps with an update that the background job may install by itself.
pub fn auto_updatable(a: &App) -> bool {
    let Some(inst) = &a.installed else { return false };
    a.has_update()
        && !a.settings.track_only
        && !a.settings.exempt_from_background
        && !inst.manual
        && !inst.kind.needs_privileges()
}
