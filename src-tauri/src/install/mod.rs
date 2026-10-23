//! Installing, updating, removing and launching apps.

mod linux;
mod portable;
mod windows;

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use chrono::Utc;

use crate::model::{App, Asset, InstallKind, Installed, Settings};
use crate::system;

pub struct Ctx<'a> {
    pub app: &'a App,
    pub asset: &'a Asset,
    pub file: &'a Path,
    pub settings: &'a Settings,
    /// Local copy of the app icon, for menu entries.
    pub icon: Option<PathBuf>,
}

/// What an installer reports back; the caller adds version info.
#[derive(Default)]
pub struct Outcome {
    pub paths: Vec<String>,
    pub executable: Option<String>,
    pub package_name: Option<String>,
    pub desktop_entry: Option<String>,
}

/// File-system friendly name: "My App" -> "my-app".
pub fn slug(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '_' { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "app".into() } else { s }
}

/// The repository name (last URL segment), falling back to the app name.
pub fn base_name(app: &App) -> String {
    url::Url::parse(&app.url)
        .ok()
        .and_then(|u| u.path_segments().and_then(|s| s.filter(|x| !x.is_empty()).nth(1).map(String::from)))
        .filter(|_| matches!(app.source, crate::model::SourceKind::Github | crate::model::SourceKind::Gitlab | crate::model::SourceKind::Forgejo))
        .map(|s| slug(&s))
        .unwrap_or_else(|| slug(&app.name))
}

pub fn install(ctx: &Ctx, previous: Option<&Installed>) -> Result<Installed> {
    let kind = ctx.asset.kind;
    if !system::supported_kinds().contains(&kind) {
        bail!("{} files can't be installed on this system", kind.label());
    }
    // Switching install type: remove the old copy first (user-level only,
    // so this never asks for a password).
    if let Some(prev) = previous {
        if prev.kind != kind && !prev.manual && !prev.kind.needs_privileges() {
            if let Err(e) = uninstall(ctx.app, prev, ctx.settings) {
                log::warn!("[{}] could not remove previous {} install: {e:#}", ctx.app.name, prev.kind.label());
            }
        }
    }
    log::info!("[{}] installing {} as {}", ctx.app.name, ctx.asset.name, kind.label());
    let out = match kind {
        InstallKind::Binary => portable::binary(ctx)?,
        InstallKind::Archive => portable::archive(ctx)?,
        InstallKind::AppImage => linux::appimage(ctx)?,
        InstallKind::Deb => linux::deb(ctx)?,
        InstallKind::Rpm => linux::rpm(ctx)?,
        InstallKind::Pacman => linux::pacman(ctx)?,
        InstallKind::Flatpak => linux::flatpak(ctx)?,
        InstallKind::Snap => linux::snap(ctx)?,
        InstallKind::Installer => windows::installer(ctx)?,
        InstallKind::Msi => windows::msi(ctx)?,
        InstallKind::Msix => windows::msix(ctx)?,
        InstallKind::Unknown => bail!("don't know how to install {}", ctx.asset.name),
    };
    // An update of an already-installed Flatpak doesn't reveal its id again.
    let mut out = out;
    if let Some(prev) = previous.filter(|p| p.kind == kind) {
        if out.package_name.is_none() {
            out.package_name = prev.package_name.clone();
        }
        if out.desktop_entry.is_none() {
            out.desktop_entry = prev.desktop_entry.clone();
        }
        if out.executable.is_none() {
            out.executable = prev.executable.clone();
        }
    }
    Ok(Installed {
        version: String::new(),
        commit: None,
        kind,
        asset_name: ctx.asset.name.clone(),
        paths: out.paths,
        executable: out.executable,
        package_name: out.package_name,
        desktop_entry: out.desktop_entry,
        installed_at: Utc::now(),
        sha256: None,
        manual: false,
    })
}

fn remove_path(p: &str) {
    let path = Path::new(p);
    let res = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    };
    if let Err(e) = res {
        log::warn!("could not remove {p}: {e}");
    }
}

pub fn uninstall(app: &App, inst: &Installed, settings: &Settings) -> Result<()> {
    let _ = settings;
    if inst.manual {
        bail!("{} was marked as installed by hand; remove it the way you installed it", app.name);
    }
    log::info!("[{}] uninstalling ({})", app.name, inst.kind.label());
    match inst.kind {
        InstallKind::Binary | InstallKind::Archive | InstallKind::AppImage => {
            for p in &inst.paths {
                remove_path(p);
            }
        }
        InstallKind::Deb | InstallKind::Rpm | InstallKind::Pacman | InstallKind::Flatpak | InstallKind::Snap => {
            linux::remove_package(inst)?
        }
        InstallKind::Installer | InstallKind::Msi | InstallKind::Msix => windows::uninstall(app, inst)?,
        InstallKind::Unknown => {}
    }
    if let Some(entry) = &inst.desktop_entry {
        if entry.contains("obtainit-") || entry.ends_with(".lnk") {
            remove_path(entry);
        }
    }
    Ok(())
}

/// Can this install be started from Obtainit?
pub fn launchable(app: &App, inst: &Installed) -> bool {
    !app.settings.launch_command.trim().is_empty()
        || inst.desktop_entry.is_some()
        || matches!(inst.kind, InstallKind::AppImage | InstallKind::Flatpak | InstallKind::Snap)
        || (cfg!(windows) && inst.executable.is_some())
}

pub fn launch(app: &App, inst: &Installed) -> Result<()> {
    let cmd = app.settings.launch_command.trim();
    if !cmd.is_empty() {
        return spawn_shell(cmd, &[]);
    }
    match inst.kind {
        InstallKind::Flatpak => {
            if let Some(id) = &inst.package_name {
                system::command("flatpak").args(["run", id]).spawn()?;
                return Ok(());
            }
        }
        InstallKind::Snap => {
            if let Some(n) = &inst.package_name {
                system::command("snap").args(["run", n]).spawn()?;
                return Ok(());
            }
        }
        _ => {}
    }
    if let Some(exe) = &inst.executable {
        if Path::new(exe).exists() {
            let dir = Path::new(exe).parent().map(Path::to_path_buf);
            let mut c = std::process::Command::new(exe);
            if let Some(d) = dir {
                c.current_dir(d);
            }
            c.spawn()?;
            return Ok(());
        }
    }
    if let Some(entry) = &inst.desktop_entry {
        return linux::launch_desktop_entry(entry);
    }
    bail!("no way to start {} is known; set a launch command in its settings", app.name)
}

/// Run a user command through the shell, with extra environment.
pub fn spawn_shell(cmd: &str, env: &[(&str, String)]) -> Result<()> {
    let mut c = if cfg!(windows) {
        let mut c = system::command("cmd");
        c.args(["/C", cmd]);
        c
    } else {
        let mut c = std::process::Command::new("sh");
        c.args(["-c", cmd]);
        c
    };
    for (k, v) in env {
        c.env(k, v);
    }
    c.spawn()?;
    Ok(())
}

/// Folder holding the app's files, for "Open folder".
pub fn folder_of(inst: &Installed) -> Option<PathBuf> {
    let p = inst.paths.first().or(inst.executable.as_ref())?;
    let p = Path::new(p);
    if p.is_dir() { Some(p.to_path_buf()) } else { p.parent().map(Path::to_path_buf) }
}

/// Version the system package manager reports for this install.
pub fn package_version_of(app: &App, inst: &Installed) -> Option<String> {
    match inst.kind {
        InstallKind::Installer | InstallKind::Msi if cfg!(windows) => windows::installed_version(app, inst),
        _ => linux::package_version(inst),
    }
}

/// Add the bin folder to the user's PATH (Windows).
pub fn add_to_path(dir: &str) -> Result<()> {
    windows::add_to_user_path(dir)
}
