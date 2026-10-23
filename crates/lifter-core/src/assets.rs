//! Picking the right release asset for a machine. Based on lifter's
//! `asset_score` / `is_auxiliary_asset` / `anchor_text_for_asset` from
//! `lifter add github`, with two additions: assets built for a different CPU
//! are penalised, and installer formats for other operating systems
//! (`.exe` on Linux, `.deb` on Windows, ...) count against the asset.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Platform {
    pub os: &'static str,
    pub arch: &'static str,
}

impl Platform {
    pub fn current() -> Self {
        Platform {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        }
    }
}

fn arch_aliases(arch: &str) -> &'static [&'static str] {
    match arch {
        "x86_64" => &["x86_64", "amd64", "x64", "x86-64", "win64", "linux64"],
        "aarch64" => &["aarch64", "arm64", "armv8"],
        "arm" => &["armv7", "armhf", "arm-", "arm_", "-arm.", "_arm."],
        "x86" => &["i686", "i386", "x86", "win32", "386"],
        "riscv64" => &["riscv64"],
        "powerpc64" => &["ppc64le", "ppc64"],
        "s390x" => &["s390x"],
        _ => &[],
    }
}

const ALL_ARCHES: &[&str] = &["x86_64", "aarch64", "arm", "x86", "riscv64", "powerpc64", "s390x"];

/// Does the name say it is for a CPU other than `arch`?
fn names_other_arch(lower: &str, arch: &str) -> bool {
    let mine = arch_aliases(arch);
    ALL_ARCHES.iter().filter(|a| **a != arch).any(|other| {
        arch_aliases(other).iter().any(|alias| {
            // Ignore aliases that are part of one of our own aliases
            // ("x86" inside "x86_64", "arm64" contains "arm").
            lower.contains(alias)
                && !mine.iter().any(|m| m.contains(alias) && lower.contains(m))
        })
    })
}

/// lifter's list of files that are never the thing you want to run.
pub fn is_auxiliary_asset(name: &str) -> bool {
    let lower = name.to_lowercase();
    [
        "checksum", "checksums", "sha256", "sha512", "sha1", "md5", ".sig", ".asc", ".pem",
        ".txt", ".json", ".spdx", "sbom", ".sbom", ".intoto", ".jsonl", ".blockmap", ".yml",
        ".yaml", ".zsync", ".minisig", ".cert", ".crt", ".sum", ".b3", ".provenance",
        "-symbols", ".pdb", ".dsym", "debuginfo", "-debug.", "source code",
    ]
    .iter()
    .any(|m| lower.contains(m))
        || lower.ends_with(".sha")
        || lower == "latest.yml"
}

const LINUX_ONLY: &[&str] = &[".deb", ".rpm", ".appimage", ".flatpak", ".snap", ".pkg.tar.zst", ".pkg.tar.xz"];
const WINDOWS_ONLY: &[&str] = &[".exe", ".msi", ".msix", ".msixbundle", ".appx", ".appxbundle"];
const MAC_ONLY: &[&str] = &[".dmg", ".pkg", ".app.zip", ".app.tar.gz"];

fn ends_with_any(lower: &str, exts: &[&str]) -> bool {
    exts.iter().any(|e| lower.ends_with(e))
}

/// Score an asset name for `platform`. Higher is better; <= 0 means "not for
/// this machine". Mirrors lifter's weights: arch +20, OS +20, musl +6 / gnu
/// +3, wrong OS -30, known extension +2.
pub fn asset_score(name: &str, platform: Platform) -> i32 {
    let lower = name.to_lowercase();
    if is_auxiliary_asset(&lower) {
        return -100;
    }
    let mut score = 0;
    if arch_aliases(platform.arch).iter().any(|a| lower.contains(a)) {
        score += 20;
    } else if names_other_arch(&lower, platform.arch) {
        // A build for another CPU can't run here, whatever else matches.
        score -= 60;
    } else if ["universal", "noarch", "_all.", "-all.", "any"].iter().any(|a| lower.contains(a)) {
        score += 8;
    }

    let mac_named = lower.contains("darwin")
        || lower.contains("apple")
        || lower.contains("macos")
        || lower.contains("osx")
        || ends_with_any(&lower, MAC_ONLY) && !lower.ends_with(".pkg.tar.zst");
    let win_named = lower.contains("windows") || lower.contains("win64") || lower.contains("win32")
        || lower.contains("-win.") || lower.contains("_win.") || lower.contains("-win-");
    match platform.os {
        "linux" => {
            if lower.contains("linux") {
                score += 20;
            }
            if lower.contains("musl") {
                score += 6;
            } else if lower.contains("gnu") {
                score += 3;
            }
            if ends_with_any(&lower, LINUX_ONLY) {
                score += 15;
            }
            if win_named || mac_named || ends_with_any(&lower, WINDOWS_ONLY) {
                score -= 30;
            }
            if lower.contains("android") || lower.ends_with(".apk") || lower.contains("freebsd") {
                score -= 30;
            }
        }
        "windows" => {
            if win_named || lower.contains("pc-windows") {
                score += 20;
            }
            if ends_with_any(&lower, WINDOWS_ONLY) {
                score += 15;
            }
            if lower.contains("msvc") {
                score += 5;
            }
            if lower.contains("linux") || mac_named || ends_with_any(&lower, LINUX_ONLY) {
                score -= 30;
            }
            if lower.contains("android") || lower.ends_with(".apk") {
                score -= 30;
            }
        }
        "macos" => {
            if mac_named {
                score += 20;
            }
            if lower.contains("linux") || win_named || ends_with_any(&lower, WINDOWS_ONLY) || ends_with_any(&lower, LINUX_ONLY) {
                score -= 30;
            }
        }
        os => {
            if lower.contains(os) {
                score += 20;
            }
        }
    }

    if [
        ".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.zst", ".tar.bz2", ".zip", ".gz", ".exe", ".com",
        ".appimage", ".msi", ".deb", ".rpm", ".flatpak", ".pkg.tar.zst",
    ]
    .iter()
    .any(|s| lower.ends_with(s))
    {
        score += 2;
    }
    score
}

/// Highest scoring assets (ties kept), in input order. Empty when nothing
/// scores above zero.
pub fn best_assets<'a>(names: &'a [String], platform: Platform) -> Vec<&'a String> {
    let mut best = 0;
    let mut out = Vec::new();
    for n in names {
        let s = asset_score(n, platform);
        if s > best {
            best = s;
            out.clear();
            out.push(n);
        } else if s == best && s > 0 {
            out.push(n);
        }
    }
    out
}

/// lifter's `simple_semver`: the first `N.N.N` in a tag.
pub fn simple_semver(tag: &str) -> Option<&str> {
    let bytes = tag.as_bytes();
    for start in 0..bytes.len() {
        if !bytes[start].is_ascii_digit() || (start > 0 && bytes[start - 1].is_ascii_digit()) {
            continue;
        }
        let mut dots = 0;
        let mut end = start;
        while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
            if bytes[end] == b'.' {
                dots += 1;
            }
            end += 1;
        }
        let cand = tag[start..end].trim_end_matches('.');
        if dots >= 2 && cand.split('.').count() == 3 && cand.split('.').all(|p| !p.is_empty()) {
            return Some(cand);
        }
    }
    None
}

pub fn escape_regex_literal(value: &str) -> String {
    regex::escape(value)
}

/// lifter's `anchor_text_for_asset`: turn a concrete asset name into a regex
/// that keeps matching when the version number changes.
pub fn anchor_regex_for_asset(asset_name: &str, tag: &str) -> String {
    let Some(version) = simple_semver(tag) else {
        return escape_regex_literal(asset_name);
    };
    let Some(start) = asset_name.find(version) else {
        return escape_regex_literal(asset_name);
    };
    let include_v = asset_name[..start]
        .chars()
        .next_back()
        .is_some_and(|c| c == 'v' || c == 'V');
    let rs = if include_v { start - 1 } else { start };
    format!(
        "{}{}{}",
        escape_regex_literal(&asset_name[..rs]),
        if include_v { r"v?(\d+\.\d+\.\d+)" } else { r"(\d+\.\d+\.\d+)" },
        escape_regex_literal(&asset_name[start + version.len()..])
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX: Platform = Platform { os: "linux", arch: "x86_64" };
    const WIN: Platform = Platform { os: "windows", arch: "x86_64" };

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn picks_linux_musl_for_ripgrep() {
        let n = names(&[
            "ripgrep-14.1.0-x86_64-unknown-linux-musl.tar.gz",
            "ripgrep-14.1.0-x86_64-unknown-linux-musl.tar.gz.sha256",
            "ripgrep-14.1.0-aarch64-unknown-linux-gnu.tar.gz",
            "ripgrep-14.1.0-x86_64-pc-windows-msvc.zip",
            "ripgrep-14.1.0-x86_64-apple-darwin.tar.gz",
        ]);
        assert_eq!(best_assets(&n, LINUX), vec![&n[0]]);
        assert_eq!(best_assets(&n, WIN), vec![&n[3]]);
    }

    #[test]
    fn other_cpu_never_compatible() {
        for n in [
            "ripgrep-15.2.0-aarch64-unknown-linux-musl.tar.gz",
            "ripgrep-15.2.0-armv7-unknown-linux-musleabihf.tar.gz",
            "nvim-linux-arm64.appimage",
        ] {
            assert!(asset_score(n, LINUX) <= 0, "{n} scored {}", asset_score(n, LINUX));
        }
    }

    #[test]
    fn prefers_platform_installers() {
        let n = names(&["App-1.2.0.AppImage", "App-Setup-1.2.0.exe", "App-1.2.0-arm64.AppImage"]);
        assert_eq!(best_assets(&n, LINUX), vec![&n[0]]);
        assert_eq!(best_assets(&n, WIN), vec![&n[1]]);
    }

    #[test]
    fn anchor_regex_like_lifter() {
        assert_eq!(
            anchor_regex_for_asset("fzf-0.60.3-linux_amd64.tar.gz", "v0.60.3"),
            r"fzf\-(\d+\.\d+\.\d+)\-linux_amd64\.tar\.gz"
        );
        assert_eq!(
            anchor_regex_for_asset("starship-x86_64-unknown-linux-musl.tar.gz", "v1.9.1"),
            r"starship\-x86_64\-unknown\-linux\-musl\.tar\.gz"
        );
    }

    #[test]
    fn auxiliary() {
        assert!(is_auxiliary_asset("checksums.txt"));
        assert!(is_auxiliary_asset("app.AppImage.zsync"));
        assert!(!is_auxiliary_asset("app.AppImage"));
    }
}
