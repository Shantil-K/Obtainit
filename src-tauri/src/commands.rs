//! Commands called from the UI. Network and install work runs on blocking
//! threads so the window stays responsive.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::import_export::{self, Report};
use crate::model::{App, AppSettings, Installed, InstallKind, Latest, Settings, SourceKind};
use crate::ops::{self, state};
use crate::{install, logs, resolve, sources, system};

type Res<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(err)
}

#[derive(Serialize)]
pub struct Bootstrap {
    apps: Vec<App>,
    settings: Settings,
    system: system::SystemInfo,
    version: &'static str,
    busy: Vec<String>,
}

#[tauri::command]
pub fn bootstrap(h: AppHandle) -> Bootstrap {
    let st = state(&h);
    let busy: Vec<String> = st.busy.lock().unwrap().iter().cloned().collect();
    let s = st.store.lock().unwrap();
    Bootstrap {
        apps: s.apps.clone(),
        settings: s.settings.clone(),
        system: system::info(&s.settings.bin_dir),
        version: env!("CARGO_PKG_VERSION"),
        busy,
    }
}

#[derive(Serialize)]
pub struct Preview {
    source: SourceKind,
    url: String,
    meta: sources::Meta,
    latest: Option<Latest>,
    error: Option<String>,
}

/// Look up a URL before adding it: detect the source, fetch details and
/// resolve the release that would be installed with these settings.
#[tauri::command]
pub async fn preview_app(h: AppHandle, url: String, source: Option<SourceKind>, settings: AppSettings) -> Res<Preview> {
    let global = state(&h).store.lock().unwrap().settings.clone();
    blocking(move || {
        let url = sources::normalize_url(&url);
        let kind = source.unwrap_or_else(|| sources::detect_kind(&url));
        let meta = sources::metadata(kind, &url, &global)?;
        let mut app = import_export::new_app(&meta.url, kind, &meta.name, settings);
        app.icon_url = meta.icon_url.clone();
        let (latest, error) = match resolve::resolve(&app, &global) {
            Ok(l) => (Some(l), None),
            Err(e) => (None, Some(format!("{e:#}"))),
        };
        Ok(Preview { source: kind, url: meta.url.clone(), meta, latest, error })
    })
    .await
}

#[derive(Deserialize)]
pub struct NewApp {
    url: String,
    source: SourceKind,
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    settings: AppSettings,
    /// Already installed outside Obtainit at this version.
    #[serde(default)]
    installed_version: Option<String>,
}

#[tauri::command]
pub async fn add_app(h: AppHandle, app: NewApp) -> Res<App> {
    let mut a = import_export::new_app(&sources::normalize_url(&app.url), app.source, app.name.trim(), app.settings);
    a.author = app.author;
    a.description = app.description;
    a.icon_url = app.icon_url;
    a.categories = app.categories;
    if let Some(v) = app.installed_version.filter(|v| !v.trim().is_empty()) {
        a.installed = Some(manual_install(v.trim()));
    }
    {
        let st = state(&h);
        let mut s = st.store.lock().unwrap();
        if s.apps.iter().any(|x| x.url.eq_ignore_ascii_case(&a.url) && x.name == a.name) {
            return Err(format!("{} is already in your list", a.name));
        }
        s.apps.push(a.clone());
        s.save_apps();
    }
    log::info!("added {} ({})", a.name, a.url);
    let id = a.id.clone();
    let h2 = h.clone();
    blocking(move || ops::check(&h2, &id)).await
}

fn manual_install(version: &str) -> Installed {
    Installed {
        version: version.to_string(),
        commit: None,
        kind: InstallKind::Unknown,
        asset_name: String::new(),
        paths: vec![],
        executable: None,
        package_name: None,
        desktop_entry: None,
        installed_at: chrono::Utc::now(),
        sha256: None,
        manual: true,
    }
}

#[derive(Deserialize)]
pub struct AppPatch {
    name: Option<String>,
    categories: Option<Vec<String>>,
    settings: Option<AppSettings>,
    favorite: Option<bool>,
    source: Option<SourceKind>,
    url: Option<String>,
}

#[tauri::command]
pub async fn update_app(h: AppHandle, id: String, patch: AppPatch) -> Res<App> {
    let recheck = patch.settings.is_some() || patch.source.is_some() || patch.url.is_some();
    let app = ops::modify(&h, &id, |a| {
        if let Some(n) = patch.name.filter(|n| !n.trim().is_empty()) {
            a.name = n.trim().to_string();
        }
        if let Some(c) = patch.categories {
            a.categories = c;
        }
        if let Some(s) = patch.settings {
            a.settings = s;
        }
        if let Some(f) = patch.favorite {
            a.favorite = f;
        }
        if let Some(s) = patch.source {
            a.source = s;
        }
        if let Some(u) = patch.url {
            a.url = sources::normalize_url(&u);
        }
    })
    .map_err(err)?;
    if !recheck {
        return Ok(app);
    }
    blocking(move || ops::check(&h, &id)).await
}

#[tauri::command]
pub async fn delete_app(h: AppHandle, id: String, uninstall: bool) -> Res<()> {
    if uninstall {
        let h2 = h.clone();
        let id2 = id.clone();
        let installed = ops::snapshot(&h, &id).map_err(err)?.0.installed.is_some_and(|i| !i.manual);
        if installed {
            blocking(move || ops::uninstall(&h2, &id2).map(|_| ())).await?;
        }
    }
    let st = state(&h);
    let mut s = st.store.lock().unwrap();
    s.apps.retain(|a| a.id != id);
    s.save_apps();
    let _ = h.emit("app-removed", &id);
    Ok(())
}

#[tauri::command]
pub async fn check_apps(h: AppHandle, ids: Option<Vec<String>>) -> Res<Vec<App>> {
    let ids = ids.unwrap_or_else(|| {
        let st = state(&h);
        let s = st.store.lock().unwrap();
        s.apps
            .iter()
            .filter(|a| !s.settings.only_check_installed || a.installed.is_some() || a.settings.track_only)
            .map(|a| a.id.clone())
            .collect()
    });
    blocking(move || Ok(ops::check_many(&h, ids))).await
}

#[tauri::command]
pub async fn install_app(h: AppHandle, id: String, asset: Option<String>) -> Res<App> {
    blocking(move || ops::install(&h, &id, asset)).await
}

/// Install every available update (except track-only apps).
#[tauri::command]
pub async fn update_all(h: AppHandle) -> Res<Vec<String>> {
    blocking(move || {
        let ids: Vec<(String, String)> = {
            let st = state(&h);
            let s = st.store.lock().unwrap();
            s.apps
                .iter()
                .filter(|a| a.has_update() && !a.settings.track_only)
                .map(|a| (a.id.clone(), a.name.clone()))
                .collect()
        };
        let mut failed = vec![];
        for (id, name) in ids {
            if let Err(e) = ops::install(&h, &id, None) {
                failed.push(format!("{name}: {e:#}"));
            }
        }
        Ok(failed)
    })
    .await
}

#[tauri::command]
pub async fn uninstall_app(h: AppHandle, id: String) -> Res<App> {
    blocking(move || ops::uninstall(&h, &id)).await
}

/// Record a version as installed (Obtainium's "mark as updated").
#[tauri::command]
pub fn mark_installed(h: AppHandle, id: String, version: Option<String>) -> Res<App> {
    ops::modify(&h, &id, |a| {
        let v = version
            .filter(|v| !v.trim().is_empty())
            .or_else(|| a.latest.as_ref().map(|l| l.version.clone()))
            .unwrap_or_default();
        let commit = a.latest.as_ref().and_then(|l| l.commit.clone());
        match a.installed.as_mut() {
            Some(i) => {
                i.version = v;
                i.commit = commit;
            }
            None => {
                let mut m = manual_install(&v);
                m.commit = commit;
                a.installed = Some(m);
            }
        }
    })
    .map_err(err)
}

/// Forget the install record without touching any files.
#[tauri::command]
pub fn mark_not_installed(h: AppHandle, id: String) -> Res<App> {
    ops::modify(&h, &id, |a| a.installed = None).map_err(err)
}

#[tauri::command]
pub fn ignore_version(h: AppHandle, id: String, version: Option<String>) -> Res<App> {
    ops::modify(&h, &id, |a| a.settings.ignored_version = version.unwrap_or_default()).map_err(err)
}

/// Pin the asset choice: lifter's generator turns the file name into a regex
/// that survives version bumps.
#[tauri::command]
pub async fn prefer_asset(h: AppHandle, id: String, asset: String) -> Res<App> {
    let app = ops::modify(&h, &id, |a| {
        let tag = a.latest.as_ref().map(|l| l.tag.clone()).unwrap_or_default();
        a.settings.asset_filter = format!("^{}$", lifter_core::assets::anchor_regex_for_asset(&asset, &tag));
        a.settings.invert_asset_filter = false;
    })
    .map_err(err)?;
    let id = app.id.clone();
    blocking(move || ops::check(&h, &id)).await
}

#[tauri::command]
pub fn launch_app(h: AppHandle, id: String) -> Res<()> {
    let (app, _) = ops::snapshot(&h, &id).map_err(err)?;
    let inst = app.installed.as_ref().ok_or("not installed")?;
    install::launch(&app, inst).map_err(err)
}

#[tauri::command]
pub fn can_launch(h: AppHandle, id: String) -> bool {
    ops::snapshot(&h, &id)
        .ok()
        .and_then(|(a, _)| a.installed.as_ref().map(|i| install::launchable(&a, i)))
        .unwrap_or(false)
}

#[tauri::command]
pub fn app_folder(h: AppHandle, id: String) -> Option<String> {
    let (app, _) = ops::snapshot(&h, &id).ok()?;
    install::folder_of(app.installed.as_ref()?).map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn save_settings(h: AppHandle, settings: Settings) -> Res<Settings> {
    use tauri_plugin_autostart::ManagerExt;
    let st = state(&h);
    let mut s = st.store.lock().unwrap();
    if settings.autostart != s.settings.autostart {
        let al = h.autolaunch();
        let r = if settings.autostart { al.enable() } else { al.disable() };
        if let Err(e) = r {
            log::warn!("autostart: {e}");
        }
    }
    s.settings = settings;
    s.save_settings();
    let out = s.settings.clone();
    drop(s);
    let _ = h.emit("settings-changed", &out);
    Ok(out)
}

#[tauri::command]
pub fn system_info(h: AppHandle) -> system::SystemInfo {
    let bin = state(&h).store.lock().unwrap().settings.bin_dir.clone();
    system::info(&bin)
}

#[tauri::command]
pub fn add_bin_to_path(h: AppHandle) -> Res<String> {
    let bin = state(&h).store.lock().unwrap().settings.bin_dir.clone();
    if cfg!(windows) {
        install::add_to_path(&bin).map_err(err)?;
        Ok("Added. Programs started from now on will find it.".into())
    } else {
        Ok(format!("Add this line to ~/.profile (or your shell's rc file), then log in again:\n\nexport PATH=\"{bin}:$PATH\""))
    }
}

#[tauri::command]
pub async fn import_file(h: AppHandle, path: String) -> Res<Report> {
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    import_text(h, text).await
}

#[tauri::command]
pub async fn import_text(h: AppHandle, text: String) -> Res<Report> {
    let defaults = state(&h).store.lock().unwrap().settings.clone();
    let mut report = import_export::parse(&text, &defaults).map_err(err)?;
    let ids: Vec<String> = {
        let st = state(&h);
        let mut s = st.store.lock().unwrap();
        let mut kept = vec![];
        for a in report.added.drain(..) {
            if s.apps.iter().any(|x| x.id == a.id || (x.url.eq_ignore_ascii_case(&a.url) && x.name == a.name)) {
                report.skipped.push(format!("{}: already in your list", a.name));
            } else {
                s.apps.push(a.clone());
                kept.push(a);
            }
        }
        s.save_apps();
        report.added = kept;
        report.added.iter().map(|a| a.id.clone()).collect()
    };
    log::info!("imported {} apps from {}", ids.len(), report.format);
    let _ = h.emit("apps-reloaded", ());
    // Fetch details for the new apps in the background.
    let h2 = h.clone();
    tauri::async_runtime::spawn_blocking(move || {
        ops::check_many(&h2, ids);
    });
    Ok(report)
}

#[tauri::command]
pub fn export_file(h: AppHandle, path: String, format: String) -> Res<()> {
    let st = state(&h);
    let s = st.store.lock().unwrap();
    let text = if format == "lifter" {
        import_export::export_lifter(&s.apps)
    } else {
        import_export::export_obtainit(&s.apps, &s.settings)
    };
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    log::info!("exported {} apps to {path}", s.apps.len());
    Ok(())
}

#[tauri::command]
pub fn get_logs() -> Vec<logs::Line> {
    logs::lines()
}

#[tauri::command]
pub fn clear_logs() {
    logs::clear();
}

#[tauri::command]
pub fn log_path() -> String {
    crate::store::data_dir().join("obtainit.log").to_string_lossy().into_owned()
}

#[tauri::command]
pub fn self_update_status() -> Option<crate::self_update::Status> {
    crate::self_update::cached()
}

#[tauri::command]
pub async fn self_update_check(h: AppHandle) -> Res<crate::self_update::Status> {
    blocking(move || Ok(crate::self_update::check(&h))).await
}

#[tauri::command]
pub async fn self_update_apply(h: AppHandle) -> Res<()> {
    blocking(move || crate::self_update::apply(&h)).await
}
