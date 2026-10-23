//! Linux installs: AppImage, system packages (via pkexec, like Autonomix),
//! Flatpak bundles, Snaps, and .desktop menu entries.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use lifter_core::archive;

use super::{slug, Ctx, Outcome};
use crate::model::{InstallKind, Installed};
use crate::system::{self, has, run, run_privileged, run_stdout};

fn applications_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_default().join("applications")
}

fn desktop_escape(s: &str) -> String {
    s.replace('\n', " ")
}

/// Write `~/.local/share/applications/obtainit-<slug>.desktop`.
pub fn write_desktop_entry(ctx: &Ctx, exe: &Path) -> Result<String> {
    let dir = applications_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("obtainit-{}.desktop", slug(&ctx.app.name)));
    let icon = ctx
        .icon
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "application-x-executable".into());
    let body = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec=\"{}\" %U\nIcon={}\nTerminal=false\nCategories=Utility;\nX-Obtainit-Id={}\n",
        desktop_escape(&ctx.app.name),
        desktop_escape(&ctx.app.description),
        exe.to_string_lossy().replace('"', "\\\""),
        icon,
        ctx.app.id
    );
    std::fs::write(&path, body)?;
    let _ = system::command("update-desktop-database").arg(&dir).output();
    Ok(path.to_string_lossy().into_owned())
}

pub fn appimage(ctx: &Ctx) -> Result<Outcome> {
    let dir = PathBuf::from(&ctx.settings.apps_dir);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let dest = dir.join(format!("{}.AppImage", slug(&ctx.app.name)));
    let tmp = dir.join(format!(".{}.obtainit-tmp", slug(&ctx.app.name)));
    std::fs::copy(ctx.file, &tmp)?;
    archive::replace_file(&tmp, &dest)?;
    archive::set_executable(&dest)?;
    let mut out = Outcome {
        paths: vec![dest.to_string_lossy().into_owned()],
        executable: Some(dest.to_string_lossy().into_owned()),
        ..Default::default()
    };
    if ctx.app.settings.create_shortcut {
        out.desktop_entry = write_desktop_entry(ctx, &dest).ok();
    }
    Ok(out)
}

fn abs(p: &Path) -> String {
    p.canonicalize().unwrap_or(p.to_path_buf()).to_string_lossy().into_owned()
}

/// Find a .desktop file among a package's files.
fn desktop_from_list(list: Option<String>) -> Option<String> {
    list?
        .lines()
        .map(|l| l.split_whitespace().last().unwrap_or(l).trim())
        .find(|l| l.starts_with("/usr/share/applications/") && l.ends_with(".desktop"))
        .map(String::from)
}

pub fn deb(ctx: &Ctx) -> Result<Outcome> {
    let file = abs(ctx.file);
    let name = run_stdout("dpkg-deb", &["-f", &file, "Package"]);
    if has("apt-get") {
        // apt resolves dependencies for a local .deb.
        run_privileged("apt-get", &["install", "-y", "--allow-downgrades", &file])?;
    } else {
        run_privileged("dpkg", &["-i", &file])?;
    }
    let desktop = name.as_ref().and_then(|n| desktop_from_list(run_stdout("dpkg", &["-L", n])));
    Ok(Outcome { package_name: name, desktop_entry: desktop, ..Default::default() })
}

pub fn rpm(ctx: &Ctx) -> Result<Outcome> {
    let file = abs(ctx.file);
    let name = run_stdout("rpm", &["-qp", "--queryformat", "%{NAME}", &file]);
    if has("dnf") {
        run_privileged("dnf", &["install", "-y", &file])?;
    } else if has("zypper") {
        run_privileged("zypper", &["--non-interactive", "install", "--allow-unsigned-rpm", &file])?;
    } else {
        run_privileged("rpm", &["-Uvh", "--replacepkgs", &file])?;
    }
    let desktop = name.as_ref().and_then(|n| desktop_from_list(run_stdout("rpm", &["-ql", n])));
    Ok(Outcome { package_name: name, desktop_entry: desktop, ..Default::default() })
}

pub fn pacman(ctx: &Ctx) -> Result<Outcome> {
    let file = abs(ctx.file);
    let name = run_stdout("pacman", &["-Qp", &file]).and_then(|s| s.split_whitespace().next().map(String::from));
    run_privileged("pacman", &["-U", "--noconfirm", &file])?;
    let desktop = name.as_ref().and_then(|n| desktop_from_list(run_stdout("pacman", &["-Ql", n])));
    Ok(Outcome { package_name: name, desktop_entry: desktop, ..Default::default() })
}

fn flatpak_apps() -> Vec<String> {
    run_stdout("flatpak", &["list", "--app", "--columns=application"])
        .map(|s| s.lines().map(|l| l.trim().to_string()).collect())
        .unwrap_or_default()
}

pub fn flatpak(ctx: &Ctx) -> Result<Outcome> {
    let file = abs(ctx.file);
    let before = flatpak_apps();
    let is_ref = file.to_lowercase().ends_with(".flatpakref");
    let mut args = vec!["install", "--user", "-y", "--noninteractive"];
    if is_ref {
        args.push("--from");
    } else {
        args.push("--bundle");
    }
    args.push(&file);
    run("flatpak", &args)?;
    // The app id is whatever appeared (or, for an update, the one already
    // recorded).
    let id = flatpak_apps().into_iter().find(|a| !before.contains(a));
    Ok(Outcome { package_name: id, ..Default::default() })
}

pub fn snap(ctx: &Ctx) -> Result<Outcome> {
    let file = abs(ctx.file);
    run_privileged("snap", &["install", "--dangerous", &file])?;
    let name = ctx.asset.name.split(['_', '.']).next().map(String::from);
    Ok(Outcome { package_name: name, ..Default::default() })
}

pub fn remove_package(inst: &Installed) -> Result<()> {
    let Some(name) = inst.package_name.as_deref() else {
        bail!("the package name wasn't recorded, so it can't be removed automatically");
    };
    match inst.kind {
        InstallKind::Deb => {
            if has("apt-get") {
                run_privileged("apt-get", &["remove", "-y", name])?;
            } else {
                run_privileged("dpkg", &["-r", name])?;
            }
        }
        InstallKind::Rpm => {
            if has("dnf") {
                run_privileged("dnf", &["remove", "-y", name])?;
            } else if has("zypper") {
                run_privileged("zypper", &["--non-interactive", "remove", name])?;
            } else {
                run_privileged("rpm", &["-e", name])?;
            }
        }
        InstallKind::Pacman => {
            run_privileged("pacman", &["-R", "--noconfirm", name])?;
        }
        InstallKind::Flatpak => {
            run("flatpak", &["uninstall", "--user", "-y", "--noninteractive", name])?;
        }
        InstallKind::Snap => {
            run_privileged("snap", &["remove", name])?;
        }
        _ => {}
    }
    Ok(())
}

/// Version of an installed system package, to spot updates made outside
/// Obtainit.
pub fn package_version(inst: &Installed) -> Option<String> {
    let name = inst.package_name.as_deref()?;
    match inst.kind {
        InstallKind::Deb => run_stdout("dpkg-query", &["-W", "-f=${Version}", name]),
        InstallKind::Rpm => run_stdout("rpm", &["-q", "--queryformat", "%{VERSION}", name]),
        InstallKind::Pacman => run_stdout("pacman", &["-Q", name])
            .and_then(|s| s.split_whitespace().nth(1).map(String::from)),
        _ => None,
    }
}

/// Start an app from its .desktop file.
pub fn launch_desktop_entry(entry: &str) -> Result<()> {
    let id = Path::new(entry).file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
    if has("gtk-launch") && system::command("gtk-launch").arg(&id).spawn().is_ok() {
        return Ok(());
    }
    if has("gio") && system::command("gio").args(["launch", entry]).spawn().is_ok() {
        return Ok(());
    }
    // Fall back to the Exec line, minus field codes like %U.
    let text = std::fs::read_to_string(entry)?;
    let exec = text
        .lines()
        .find_map(|l| l.strip_prefix("Exec="))
        .context("no Exec line in desktop entry")?;
    let cmd: String = exec
        .split_whitespace()
        .filter(|p| !(p.starts_with('%') && p.len() == 2))
        .collect::<Vec<_>>()
        .join(" ");
    super::spawn_shell(&cmd, &[])
}
