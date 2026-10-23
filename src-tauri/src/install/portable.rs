//! Portable installs, lifter style: single binaries copied into the bin
//! folder, binaries pulled out of archives, or a whole archive unpacked into
//! its own folder (for multi-file apps).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lifter_core::archive::{self, ArchiveKind, Entry};

use super::{base_name, linux, slug, windows, Ctx, Outcome};

fn exe_suffix(name: &str) -> &'static str {
    if cfg!(windows) && !name.to_lowercase().ends_with(".exe") {
        ".exe"
    } else {
        ""
    }
}

/// A single executable file: copy to the bin folder.
pub fn binary(ctx: &Ctx) -> Result<Outcome> {
    let bin = PathBuf::from(&ctx.settings.bin_dir);
    std::fs::create_dir_all(&bin).with_context(|| format!("creating {}", bin.display()))?;
    let s = &ctx.app.settings;
    let name = if !s.desired_filename.trim().is_empty() {
        s.desired_filename.trim().to_string()
    } else {
        // lifter names the file after its config section; we use the repo
        // name, e.g. "ripgrep".
        let b = base_name(ctx.app);
        let ext = if ctx.asset.name.to_lowercase().ends_with(".com") { ".com" } else { "" };
        format!("{b}{ext}")
    };
    let name = format!("{name}{}", exe_suffix(&name));
    let dest = bin.join(&name);
    let tmp = bin.join(format!(".{name}.obtainit-tmp"));
    std::fs::copy(ctx.file, &tmp).with_context(|| format!("copying to {}", tmp.display()))?;
    archive::replace_file(&tmp, &dest)?;
    archive::set_executable(&dest)?;
    let d = dest.to_string_lossy().into_owned();
    Ok(Outcome {
        paths: vec![d.clone()],
        executable: Some(d),
        ..Default::default()
    })
}

const NOT_PROGRAMS: &[&str] = &[
    "license", "licence", "readme", "copying", "changelog", "changes", "notice", "authors",
    "contributors", "makefile", "install", "credits", "version", "news", "todo", "history",
];

/// Is this archive entry likely an executable program?
fn looks_executable(e: &Entry) -> bool {
    if e.is_dir {
        return false;
    }
    let path = e.path.replace('\\', "/").to_lowercase();
    let base = path.rsplit('/').next().unwrap_or(&path);
    if base.starts_with('.') || base.is_empty() {
        return false;
    }
    if ["/completions/", "/complete/", "/autocomplete/", "/man/", "/doc/", "/docs/", "/share/"]
        .iter()
        .any(|d| format!("/{path}").contains(d))
    {
        return false;
    }
    if base.contains("unins") || base.contains("uninstall") {
        return false;
    }
    if cfg!(windows) {
        return base.ends_with(".exe");
    }
    if [".so", ".dll", ".dylib", ".a", ".exe", ".bat", ".cmd", ".ps1"]
        .iter()
        .any(|x| base.ends_with(x))
        || base.contains(".so.")
    {
        return false;
    }
    let stem = base.split('.').next().unwrap_or(base);
    if NOT_PROGRAMS.iter().any(|n| stem.starts_with(n)) || stem == "unlicense" {
        return false;
    }
    if e.mode_known {
        e.executable
    } else {
        !base.contains('.')
    }
}

fn stem_of(path: &str) -> String {
    let base = path.replace('\\', "/");
    let base = base.rsplit('/').next().unwrap_or(&base);
    base.strip_suffix(".exe").unwrap_or(base).to_lowercase()
}

/// Archive install. Decides between lifter's "pull out the binaries" and
/// unpacking the whole app into a folder.
pub fn archive(ctx: &Ctx) -> Result<Outcome> {
    let kind = archive::detect_kind(&ctx.asset.name)
        .with_context(|| format!("{} is not a supported archive", ctx.asset.name))?;
    let s = &ctx.app.settings;
    let targets: Vec<String> = s.extract_targets.iter().filter(|t| !t.trim().is_empty()).cloned().collect();

    let mode = match s.archive_mode.as_str() {
        "binaries" => "binaries",
        "folder" => "folder",
        _ if !targets.is_empty() => "binaries",
        _ if matches!(kind, ArchiveKind::Gz | ArchiveKind::Xz) => "binaries",
        _ => "auto",
    };

    if mode == "binaries" {
        let patterns = if !targets.is_empty() {
            targets
        } else if matches!(kind, ArchiveKind::Gz | ArchiveKind::Xz) {
            vec![".*".into()]
        } else {
            let picked = pick_binaries(ctx, kind)?;
            if picked.is_empty() {
                anyhow::bail!("couldn't find a program inside {}; set the files to extract in this app's settings", ctx.asset.name);
            }
            picked
        };
        let mut desired = s.desired_filename.trim().to_string();
        if desired.is_empty() && matches!(kind, ArchiveKind::Gz | ArchiveKind::Xz) {
            desired = format!("{}{}", base_name(ctx.app), if cfg!(windows) { ".exe" } else { "" });
        }
        return extract_binaries(ctx, kind, &patterns, &desired);
    }

    if mode == "auto" {
        let picked = pick_binaries(ctx, kind)?;
        if picked.len() == 1 {
            return extract_binaries(ctx, kind, &picked, s.desired_filename.trim());
        }
    }
    folder(ctx, kind)
}

/// Docs, licences, man pages and shell completions: files that ship next to
/// a CLI tool but aren't needed to run it.
fn is_extra(e: &Entry) -> bool {
    let path = e.path.replace('\\', "/").to_lowercase();
    let base = path.rsplit('/').next().unwrap_or(&path);
    let stem = base.split('.').next().unwrap_or(base);
    NOT_PROGRAMS.iter().any(|n| stem.starts_with(n))
        || ["unlicense", "faq", "guide", "code_of_conduct", "contributing", "security"].contains(&stem)
        || [".md", ".txt", ".rst", ".html", ".pdf", ".bash", ".fish", ".zsh", ".ps1", ".elv", ".nu", ".1", ".5", ".7", ".gz", ".png", ".svg"]
            .iter()
            .any(|x| base.ends_with(x))
        || base.starts_with('_')
        || ["/complete", "/completion", "/autocomplete", "/man/", "/doc/", "/docs/", "/contrib/"]
            .iter()
            .any(|d| format!("/{path}").contains(d))
}

/// Auto-pick binaries (lifter mode): a lone program in the archive, or the
/// one named like the repository, as long as nothing else in the archive
/// looks like data the program needs. Otherwise return nothing, and the
/// caller unpacks the whole folder.
fn pick_binaries(ctx: &Ctx, kind: ArchiveKind) -> Result<Vec<String>> {
    let entries = archive::list_entries(ctx.file, kind)?;
    let programs: Vec<&Entry> = entries.iter().filter(|e| looks_executable(e)).collect();
    let payload = entries
        .iter()
        .filter(|e| !e.is_dir && !looks_executable(e) && !is_extra(e))
        .count();
    let base = base_name(ctx.app);
    let name = slug(&ctx.app.name);
    let squash = |s: &str| s.replace(['-', '_'], "");
    let named: Vec<&&Entry> = programs
        .iter()
        .filter(|e| {
            let st = stem_of(&e.path);
            st == base || st == name || squash(&st) == squash(&base)
        })
        .collect();
    let lone = |e: &Entry| regex::escape(e.path.replace('\\', "/").rsplit('/').next().unwrap_or(&e.path));
    if payload <= 2 {
        if programs.len() == 1 {
            return Ok(vec![lone(programs[0])]);
        }
        if named.len() == 1 && programs.len() <= 4 {
            return Ok(vec![lone(named[0])]);
        }
    }
    Ok(vec![])
}

fn extract_binaries(ctx: &Ctx, kind: ArchiveKind, patterns: &[String], desired: &str) -> Result<Outcome> {
    let bin = PathBuf::from(&ctx.settings.bin_dir);
    let desired = if desired.is_empty() { None } else { Some(desired) };
    let targets = archive::build_targets(patterns, desired)?;
    let written = archive::extract_targets(ctx.file, kind, &targets, &bin)?;
    for w in &written {
        archive::set_executable(w)?;
    }
    let paths: Vec<String> = written.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    Ok(Outcome {
        executable: paths.first().cloned(),
        paths,
        ..Default::default()
    })
}

/// Pick the main program in an unpacked folder.
pub fn find_main_executable(dir: &Path, hints: &[String]) -> Option<PathBuf> {
    let mut best: Option<(i64, PathBuf)> = None;
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(md) = e.metadata() else { continue };
            if md.is_dir() {
                if depth < 3 {
                    stack.push((p, depth + 1));
                }
                continue;
            }
            #[cfg(unix)]
            let exec = {
                use std::os::unix::fs::PermissionsExt;
                md.permissions().mode() & 0o111 != 0
            };
            #[cfg(not(unix))]
            let exec = false;
            let rel = p.strip_prefix(dir).unwrap_or(&p).to_string_lossy().into_owned();
            let entry = Entry { path: rel, size: md.len(), is_dir: false, executable: exec, mode_known: cfg!(unix) };
            if !looks_executable(&entry) {
                continue;
            }
            let lower = stem_of(&entry.path);
            if ["crashpad", "chrome-sandbox", "chrome_crashpad_handler", "update", "squirrel", "elevate", "helper", "notification_helper"]
                .iter()
                .any(|bad| lower.contains(bad))
            {
                continue;
            }
            let mut score = -(depth as i64) * 50 + (md.len() / 1_000_000) as i64;
            if hints.iter().any(|h| *h == lower) {
                score += 1000;
            } else if hints.iter().any(|h| lower.contains(h.as_str()) || h.contains(lower.as_str())) {
                score += 300;
            }
            if best.as_ref().is_none_or(|(b, _)| score > *b) {
                best = Some((score, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Unpack the whole archive into `<apps dir>/<slug>` and add a menu entry.
fn folder(ctx: &Ctx, kind: ArchiveKind) -> Result<Outcome> {
    let dest = PathBuf::from(&ctx.settings.apps_dir).join(slug(&ctx.app.name));
    archive::extract_all(ctx.file, kind, &dest)?;
    let hints = vec![base_name(ctx.app), slug(&ctx.app.name), slug(&ctx.app.name).replace('-', "")];
    let exe = find_main_executable(&dest, &hints);
    if let Some(e) = &exe {
        archive::set_executable(e)?;
    }
    let mut out = Outcome {
        paths: vec![dest.to_string_lossy().into_owned()],
        executable: exe.as_ref().map(|p| p.to_string_lossy().into_owned()),
        ..Default::default()
    };
    if ctx.app.settings.create_shortcut {
        if let Some(e) = &exe {
            out.desktop_entry = if cfg!(windows) {
                windows::create_shortcut(ctx, e).ok()
            } else {
                linux::write_desktop_entry(ctx, e).ok()
            };
        }
    }
    Ok(out)
}
