//! Archive handling. The target model is lifter's:
//!
//! * singular — one pattern (`target_filename_to_extract_from_archive`),
//!   optionally renamed (`desired_filename`);
//! * plural — several patterns (`target_filenames_to_extract_from_archive`),
//!   each written under its archive basename.
//!
//! Patterns are anchored regexes matched against entry basenames, and each
//! is filled by the first entry that matches, as in lifter. Additions here:
//! extraction of the whole archive into a folder (for multi-file apps),
//! `.tar.zst` / `.tar.bz2` / `.xz`, and listing entries. Files are written
//! to a temporary name and renamed into place, so a failed extraction never
//! leaves a half-written binary behind.

use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use log::*;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveKind {
    TarGz,
    TarXz,
    TarZst,
    TarBz2,
    Tar,
    Zip,
    /// Single gzip-compressed file (lifter's `.gz`).
    Gz,
    /// Single xz-compressed file.
    Xz,
}

/// lifter's extension sniffing, extended.
pub fn detect_kind(file_name: &str) -> Option<ArchiveKind> {
    let l = file_name.to_lowercase();
    // Arch packages are installed by pacman, not unpacked by us.
    if l.ends_with(".pkg.tar.zst") || l.ends_with(".pkg.tar.xz") {
        return None;
    }
    Some(if l.ends_with(".tar.gz") || l.ends_with(".tgz") {
        ArchiveKind::TarGz
    } else if l.ends_with(".tar.xz") || l.ends_with(".txz") {
        ArchiveKind::TarXz
    } else if l.ends_with(".tar.zst") || l.ends_with(".tzst") {
        ArchiveKind::TarZst
    } else if l.ends_with(".tar.bz2") || l.ends_with(".tbz2") || l.ends_with(".tbz") {
        ArchiveKind::TarBz2
    } else if l.ends_with(".tar") {
        ArchiveKind::Tar
    } else if l.ends_with(".zip") {
        ArchiveKind::Zip
    } else if l.ends_with(".gz") {
        ArchiveKind::Gz
    } else if l.ends_with(".xz") {
        ArchiveKind::Xz
    } else {
        return None;
    })
}

#[derive(Debug, Clone)]
pub struct ExtractionTarget {
    pub pattern_str: String,
    pub pattern: regex::Regex,
    /// `Some` = write under this name. `None` = keep the archive basename.
    pub rename_to: Option<String>,
}

impl ExtractionTarget {
    pub fn predicted_output_name(&self) -> &str {
        self.rename_to.as_deref().unwrap_or(&self.pattern_str)
    }
}

/// Build targets the way lifter's `build_extraction_targets` does.
/// One pattern + `desired` = singular rename; several patterns = plural.
pub fn build_targets(patterns: &[String], desired: Option<&str>) -> Result<Vec<ExtractionTarget>> {
    let patterns: Vec<&String> = patterns.iter().filter(|p| !p.trim().is_empty()).collect();
    if patterns.len() > 1 && desired.is_some_and(|d| !d.is_empty()) {
        bail!("a rename (desired file name) only applies when extracting a single file");
    }
    let singular = patterns.len() == 1;
    patterns
        .into_iter()
        .map(|p| {
            let pattern = regex::Regex::new(&format!("^(?:{})$", p.trim()))
                .with_context(|| format!("bad extraction pattern {p:?}"))?;
            // Rename only when asked; otherwise keep the name from the
            // archive (patterns may be regexes, so they make poor names).
            let rename_to = if singular {
                desired.filter(|d| !d.is_empty()).map(String::from)
            } else {
                None
            };
            Ok(ExtractionTarget {
                pattern_str: p.trim().to_string(),
                pattern,
                rename_to,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub executable: bool,
    /// The archive recorded Unix permissions, so `executable` is reliable.
    pub mode_known: bool,
}

fn open_tar(path: &Path, kind: ArchiveKind) -> Result<tar::Archive<Box<dyn Read>>> {
    let f = BufReader::new(File::open(path)?);
    let r: Box<dyn Read> = match kind {
        ArchiveKind::TarGz => Box::new(flate2::read::GzDecoder::new(f)),
        ArchiveKind::TarXz => Box::new(xz2::read::XzDecoder::new(f)),
        ArchiveKind::TarZst => Box::new(zstd::stream::read::Decoder::new(f)?),
        ArchiveKind::TarBz2 => Box::new(bzip2::read::BzDecoder::new(f)),
        ArchiveKind::Tar => Box::new(f),
        _ => bail!("{kind:?} is not a tar archive"),
    };
    Ok(tar::Archive::new(r))
}

fn basename(p: &str) -> Option<&str> {
    p.trim_end_matches('/').rsplit(['/', '\\']).next().filter(|s| !s.is_empty())
}

/// List what is inside an archive.
pub fn list_entries(path: &Path, kind: ArchiveKind) -> Result<Vec<Entry>> {
    let mut out = Vec::new();
    match kind {
        ArchiveKind::Zip => {
            let mut z = zip::ZipArchive::new(BufReader::new(File::open(path)?))?;
            for i in 0..z.len() {
                let e = z.by_index_raw(i)?;
                let name = e.name()?.to_string();
                out.push(Entry {
                    executable: e.unix_mode().is_some_and(|m| m & 0o111 != 0)
                        || name.to_lowercase().ends_with(".exe"),
                    mode_known: e.unix_mode().is_some(),
                    path: name,
                    size: e.size(),
                    is_dir: e.is_dir(),
                });
            }
        }
        ArchiveKind::Gz | ArchiveKind::Xz => {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file")
                .to_string();
            out.push(Entry {
                path: name,
                size: 0,
                is_dir: false,
                executable: true,
                mode_known: false,
            });
        }
        _ => {
            let mut a = open_tar(path, kind)?;
            for e in a.entries()? {
                let e = e?;
                let p = e.path()?.to_string_lossy().into_owned();
                let h = e.header();
                out.push(Entry {
                    is_dir: h.entry_type().is_dir(),
                    executable: h.mode().map(|m| m & 0o111 != 0).unwrap_or(false),
                    mode_known: h.mode().is_ok(),
                    size: h.size().unwrap_or(0),
                    path: p,
                });
            }
        }
    }
    Ok(out)
}

fn write_atomically(dest: &Path, reader: &mut dyn Read) -> Result<()> {
    let tmp = dest.with_file_name(format!(
        ".{}.obtainit-tmp",
        dest.file_name().and_then(|s| s.to_str()).unwrap_or("file")
    ));
    {
        let mut f = File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        if let Err(e) = std::io::copy(reader, &mut f) {
            drop(f);
            let _ = std::fs::remove_file(&tmp);
            return Err(e).context("extracting");
        }
        f.flush()?;
    }
    replace_file(&tmp, dest)
}

/// Rename `src` over `dest`. On Windows a running .exe cannot be replaced, but
/// it can be renamed out of the way first.
pub fn replace_file(src: &Path, dest: &Path) -> Result<()> {
    match std::fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(e) if cfg!(windows) && dest.exists() => {
            let old = dest.with_extension("old");
            let _ = std::fs::remove_file(&old);
            std::fs::rename(dest, &old).map_err(|_| e)?;
            std::fs::rename(src, dest)?;
            let _ = std::fs::remove_file(&old);
            Ok(())
        }
        Err(e) => Err(e).with_context(|| format!("replacing {}", dest.display())),
    }
}

/// Extract lifter-style targets into `out_dir`. Returns the files written.
/// Missing targets are an error unless at least one target was found.
pub fn extract_targets(
    path: &Path,
    kind: ArchiveKind,
    targets: &[ExtractionTarget],
    out_dir: &Path,
) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(out_dir)?;
    let mut done = vec![false; targets.len()];
    let mut written = Vec::new();
    let mut claim = |base: &str| -> Option<usize> {
        let i = targets
            .iter()
            .enumerate()
            .position(|(i, t)| !done[i] && t.pattern.is_match(base))?;
        done[i] = true;
        Some(i)
    };

    match kind {
        ArchiveKind::Gz | ArchiveKind::Xz => {
            let [t] = targets else {
                bail!("a single-file .gz/.xz download can only produce one file");
            };
            let name = t.rename_to.clone().unwrap_or_else(|| {
                path.file_stem().and_then(|s| s.to_str()).unwrap_or("file").to_string()
            });
            let dest = out_dir.join(name);
            let f = BufReader::new(File::open(path)?);
            let mut r: Box<dyn Read> = if kind == ArchiveKind::Gz {
                Box::new(flate2::read::GzDecoder::new(f))
            } else {
                Box::new(xz2::read::XzDecoder::new(f))
            };
            write_atomically(&dest, &mut r)?;
            written.push(dest);
            return Ok(written);
        }
        ArchiveKind::Zip => {
            let mut z = zip::ZipArchive::new(BufReader::new(File::open(path)?))?;
            for i in 0..z.len() {
                let name = match z.name_for_index(i) {
                    Some(Ok(n)) => n.to_string(),
                    _ => continue,
                };
                let Some(base) = basename(&name) else { continue };
                if name.ends_with('/') {
                    continue;
                }
                let Some(ti) = claim(base) else { continue };
                let out_name = targets[ti].rename_to.as_deref().unwrap_or(base);
                let dest = out_dir.join(out_name);
                debug!("zip: {name} -> {}", dest.display());
                let mut e = z.by_index(i)?;
                write_atomically(&dest, &mut e)?;
                written.push(dest);
            }
        }
        _ => {
            let mut a = open_tar(path, kind)?;
            for e in a.entries()? {
                let mut e = match e {
                    Ok(e) => e,
                    Err(err) => {
                        debug!("skipping unreadable tar entry: {err}");
                        continue;
                    }
                };
                if !e.header().entry_type().is_file() {
                    continue;
                }
                let name = e.path()?.to_string_lossy().into_owned();
                let Some(base) = basename(&name) else { continue };
                let Some(ti) = claim(base) else { continue };
                let out_name = targets[ti].rename_to.as_deref().unwrap_or(base).to_string();
                let dest = out_dir.join(&out_name);
                debug!("tar: {name} -> {}", dest.display());
                write_atomically(&dest, &mut e)?;
                written.push(dest);
            }
        }
    }

    let missing: Vec<&str> = targets
        .iter()
        .zip(&done)
        .filter(|(_, d)| !**d)
        .map(|(t, _)| t.pattern_str.as_str())
        .collect();
    if !missing.is_empty() {
        if written.is_empty() {
            bail!("nothing in the archive matched {}", missing.join(", "));
        }
        warn!("no file inside the archive matched: {}", missing.join(", "));
    }
    Ok(written)
}

/// Extract everything into `dest_dir`, replacing it. If the archive holds a
/// single top-level folder, its contents become `dest_dir`.
pub fn extract_all(path: &Path, kind: ArchiveKind, dest_dir: &Path) -> Result<()> {
    let parent = dest_dir
        .parent()
        .ok_or_else(|| anyhow!("bad destination {}", dest_dir.display()))?;
    std::fs::create_dir_all(parent)?;
    let staging = parent.join(format!(
        ".{}.obtainit-staging",
        dest_dir.file_name().and_then(|s| s.to_str()).unwrap_or("app")
    ));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)?;

    let res = (|| -> Result<()> {
        match kind {
            ArchiveKind::Zip => {
                let mut z = zip::ZipArchive::new(BufReader::new(File::open(path)?))?;
                // `extract` rejects entries escaping the folder.
                z.extract(&staging)?;
            }
            ArchiveKind::Gz | ArchiveKind::Xz => {
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
                let t = build_targets(&[".*".into()], Some(name))?;
                extract_targets(path, kind, &t, &staging)?;
            }
            _ => {
                let mut a = open_tar(path, kind)?;
                a.set_preserve_permissions(true);
                // tar's `unpack` refuses `..` and absolute paths.
                a.unpack(&staging)?;
            }
        }
        Ok(())
    })();
    if let Err(e) = res {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }

    // Unwrap a single top-level folder.
    let entries: Vec<_> = std::fs::read_dir(&staging)?.filter_map(|e| e.ok()).collect();
    let root = if entries.len() == 1 && entries[0].file_type().map(|t| t.is_dir()).unwrap_or(false) {
        entries[0].path()
    } else {
        staging.clone()
    };

    let backup = parent.join(format!(
        ".{}.obtainit-old",
        dest_dir.file_name().and_then(|s| s.to_str()).unwrap_or("app")
    ));
    let _ = std::fs::remove_dir_all(&backup);
    if dest_dir.exists() {
        std::fs::rename(dest_dir, &backup)
            .with_context(|| format!("moving old {} aside (is the app running?)", dest_dir.display()))?;
    }
    if let Err(e) = std::fs::rename(&root, dest_dir) {
        if backup.exists() {
            let _ = std::fs::rename(&backup, dest_dir);
        }
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e).context("moving extracted files into place");
    }
    let _ = std::fs::remove_dir_all(&backup);
    let _ = std::fs::remove_dir_all(&staging);
    Ok(())
}

#[cfg(unix)]
pub fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    if perms.mode() & 0o111 == 0 {
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tar_gz(entries: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let f = tempfile::Builder::new().suffix(".tar.gz").tempfile().unwrap();
        let gz = flate2::write::GzEncoder::new(f.reopen().unwrap(), flate2::Compression::fast());
        let mut b = tar::Builder::new(gz);
        for (p, data) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(data.len() as u64);
            h.set_mode(0o755);
            h.set_cksum();
            b.append_data(&mut h, p, *data).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap();
        f
    }

    #[test]
    fn singular_rename_from_tar_gz() {
        let a = tar_gz(&[("ripgrep-14/doc/rg.1", b"man"), ("ripgrep-14/rg", b"bin")]);
        let out = tempfile::tempdir().unwrap();
        let t = build_targets(&["rg".into()], Some("ripgrep")).unwrap();
        let w = extract_targets(a.path(), ArchiveKind::TarGz, &t, out.path()).unwrap();
        assert_eq!(w, vec![out.path().join("ripgrep")]);
        assert_eq!(std::fs::read(&w[0]).unwrap(), b"bin");
    }

    #[test]
    fn extract_all_unwraps_top_folder() {
        let a = tar_gz(&[("app-1.0/bin/app", b"x"), ("app-1.0/README", b"r")]);
        let out = tempfile::tempdir().unwrap();
        let dest = out.path().join("app");
        extract_all(a.path(), ArchiveKind::TarGz, &dest).unwrap();
        assert!(dest.join("bin/app").exists());
        // Again, replacing the old copy.
        extract_all(a.path(), ArchiveKind::TarGz, &dest).unwrap();
        assert!(dest.join("README").exists());
    }

    #[test]
    fn missing_target_is_error() {
        let a = tar_gz(&[("x", b"x")]);
        let out = tempfile::tempdir().unwrap();
        let t = build_targets(&["nope".into()], None).unwrap();
        assert!(extract_targets(a.path(), ArchiveKind::TarGz, &t, out.path()).is_err());
    }
}
