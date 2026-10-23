//! What this machine can install, and process helpers.

use std::process::{Command, Output};
use std::sync::OnceLock;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::model::InstallKind;

#[derive(Serialize, Clone, Debug)]
pub struct SystemInfo {
    pub os: &'static str,
    pub arch: &'static str,
    pub distro: String,
    pub supported_kinds: Vec<InstallKind>,
    pub has_pkexec: bool,
    pub bin_dir_in_path: bool,
}

pub fn has(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

fn distro() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string()))
        })
        .unwrap_or_else(|| {
            if cfg!(windows) {
                "Windows".into()
            } else {
                std::env::consts::OS.into()
            }
        })
}

pub fn supported_kinds() -> &'static Vec<InstallKind> {
    static K: OnceLock<Vec<InstallKind>> = OnceLock::new();
    K.get_or_init(|| {
        use InstallKind::*;
        let mut v = vec![Binary, Archive];
        if cfg!(windows) {
            v.extend([Installer, Msi, Msix]);
        } else if cfg!(target_os = "linux") {
            v.push(AppImage);
            if has("dpkg") {
                v.push(Deb);
            }
            if has("rpm") && !has("dpkg") || has("dnf") || has("zypper") {
                v.push(Rpm);
            }
            if has("pacman") {
                v.push(Pacman);
            }
            if has("flatpak") {
                v.push(Flatpak);
            }
            if has("snap") {
                v.push(Snap);
            }
        }
        v
    })
}

pub fn info(bin_dir: &str) -> SystemInfo {
    let in_path = std::env::var_os("PATH")
        .map(|p| {
            std::env::split_paths(&p).any(|d| {
                d == std::path::Path::new(bin_dir)
                    || d.canonicalize().ok() == std::path::Path::new(bin_dir).canonicalize().ok()
                        && d.exists()
            })
        })
        .unwrap_or(false);
    SystemInfo {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        distro: distro(),
        supported_kinds: supported_kinds().clone(),
        has_pkexec: has("pkexec"),
        bin_dir_in_path: in_path,
    }
}

/// A `Command` that does not flash a console window on Windows.
pub fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

fn describe(program: &str, args: &[&str]) -> String {
    format!("{program} {}", args.join(" "))
}

/// Run and require success; the error carries stderr.
pub fn run(program: &str, args: &[&str]) -> Result<Output> {
    log::info!("running: {}", describe(program, args));
    let out = command(program)
        .args(args)
        .output()
        .with_context(|| format!("could not start {program}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let msg = if err.trim().is_empty() {
            String::from_utf8_lossy(&out.stdout).to_string()
        } else {
            err.to_string()
        };
        bail!(
            "{} failed ({}): {}",
            describe(program, args),
            out.status,
            lifter_core::http::one_line(&msg)
        );
    }
    Ok(out)
}

pub fn run_stdout(program: &str, args: &[&str]) -> Option<String> {
    command(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Run with administrator rights via polkit (`pkexec`), like Autonomix.
pub fn run_privileged(program: &str, args: &[&str]) -> Result<Output> {
    #[cfg(unix)]
    {
        if run_stdout("id", &["-u"]).as_deref() == Some("0") {
            return run(program, args);
        }
        if !has("pkexec") {
            bail!("pkexec (polkit) is needed to install system packages but was not found");
        }
        let full = which::which(program)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| program.to_string());
        let mut a = vec![full.as_str()];
        a.extend_from_slice(args);
        match run("pkexec", &a) {
            Err(e) if e.to_string().contains("exit status: 126") => {
                bail!("authentication was cancelled")
            }
            other => other,
        }
    }
    #[cfg(not(unix))]
    {
        run(program, args)
    }
}

/// Run a PowerShell snippet (Windows).
pub fn powershell(script: &str) -> Result<String> {
    let out = run(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script],
    )?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Quote a string for a single-quoted PowerShell literal.
pub fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
