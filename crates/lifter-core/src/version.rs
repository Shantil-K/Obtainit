//! Version comparison. lifter compared versions as plain strings (with a
//! TODO to compare numerically); this does it properly:
//! `v1.10.0 > v1.9.2`, `2.0.0 > 2.0.0-rc.1`, `2024.10.03 > 2024.9.30`.

use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, Clone)]
enum Tok {
    Num(u64),
    Text(String),
}

/// Strip decoration that is never part of the version itself.
pub fn normalize(v: &str) -> String {
    let mut s = v.trim();
    for p in ["release-", "release_", "version-", "version", "rel-", "ver"] {
        if s.len() > p.len() && s.get(..p.len()).is_some_and(|h| h.eq_ignore_ascii_case(p)) {
            s = &s[p.len()..];
        }
    }
    let s = s.trim_start_matches(['v', 'V', '-', '_', ' ']);
    s.to_string()
}

fn tokens(v: &str) -> Vec<Tok> {
    let n = normalize(v);
    // Drop build metadata (`+build.5`).
    let n = n.split('+').next().unwrap_or("");
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut cur_num: Option<bool> = None;
    let flush = |cur: &mut String, num: Option<bool>, out: &mut Vec<Tok>| {
        if cur.is_empty() {
            return;
        }
        if num == Some(true) {
            out.push(Tok::Num(cur.parse().unwrap_or(u64::MAX)));
        } else {
            out.push(Tok::Text(cur.to_lowercase()));
        }
        cur.clear();
    };
    for c in n.chars() {
        if c.is_ascii_digit() {
            if cur_num == Some(false) {
                flush(&mut cur, cur_num, &mut out);
            }
            cur_num = Some(true);
            cur.push(c);
        } else if c.is_alphabetic() {
            if cur_num == Some(true) {
                flush(&mut cur, cur_num, &mut out);
            }
            cur_num = Some(false);
            cur.push(c);
        } else {
            flush(&mut cur, cur_num, &mut out);
            cur_num = None;
        }
    }
    flush(&mut cur, cur_num, &mut out);
    out
}

/// Rank for pre-release words. Anything below 0 sorts before the release.
fn pre_rank(t: &str) -> Option<i32> {
    Some(match t {
        "dev" | "snapshot" | "nightly" | "canary" => -5,
        "a" | "alpha" => -4,
        "b" | "beta" => -3,
        "pre" | "preview" => -2,
        "rc" | "cr" => -1,
        _ => return None,
    })
}

/// Compare two version strings.
pub fn compare(a: &str, b: &str) -> Ordering {
    let (ta, tb) = (tokens(a), tokens(b));
    let mut i = 0;
    loop {
        match (ta.get(i), tb.get(i)) {
            (None, None) => return Ordering::Equal,
            (Some(x), None) => return tail_order(x),
            (None, Some(y)) => return tail_order(y).reverse(),
            (Some(Tok::Num(x)), Some(Tok::Num(y))) => {
                if x != y {
                    return x.cmp(y);
                }
            }
            (Some(Tok::Text(x)), Some(Tok::Text(y))) => {
                if x != y {
                    return match (pre_rank(x), pre_rank(y)) {
                        (Some(p), Some(q)) => p.cmp(&q),
                        _ => x.cmp(y),
                    };
                }
            }
            // A number beats a pre-release word (1.0.1 > 1.0-rc), but a
            // plain word like "lts" just sorts after numbers.
            (Some(Tok::Num(_)), Some(Tok::Text(y))) => {
                return if pre_rank(y).is_some() { Ordering::Greater } else { Ordering::Less };
            }
            (Some(Tok::Text(x)), Some(Tok::Num(_))) => {
                return if pre_rank(x).is_some() { Ordering::Less } else { Ordering::Greater };
            }
        }
        i += 1;
    }
}

/// Ordering of a version that has an extra token `t` beyond the other.
fn tail_order(t: &Tok) -> Ordering {
    match t {
        Tok::Num(0) => Ordering::Equal,
        Tok::Num(_) => Ordering::Greater,
        Tok::Text(s) if pre_rank(s).is_some() => Ordering::Less,
        Tok::Text(_) => Ordering::Greater,
    }
}

/// True when `candidate` should replace `installed`.
pub fn is_newer(candidate: &str, installed: &str) -> bool {
    compare(candidate, installed) == Ordering::Greater
}

/// Does the version string look like a pre-release?
pub fn looks_prerelease(v: &str) -> bool {
    tokens(v)
        .iter()
        .any(|t| matches!(t, Tok::Text(s) if pre_rank(s).is_some()))
}

/// Apply a "version extraction" regex (Obtainium's option). `group` may be a
/// number, a named group, or a template such as `$1.$2`.
pub fn extract(v: &str, regex_src: &str, group: &str) -> Option<String> {
    if regex_src.trim().is_empty() {
        return Some(v.to_string());
    }
    let re = regex::Regex::new(regex_src).ok()?;
    let caps = re.captures(v)?;
    let g = group.trim();
    if g.is_empty() {
        return Some(caps.get(0)?.as_str().to_string());
    }
    if let Ok(n) = g.parse::<usize>() {
        return caps.get(n).map(|m| m.as_str().to_string());
    }
    if g.contains('$') {
        let mut out = String::new();
        caps.expand(g, &mut out);
        return Some(out);
    }
    caps.name(g).map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ordering::*;

    #[test]
    fn ordering() {
        assert_eq!(compare("v1.10.0", "1.9.2"), Greater);
        assert_eq!(compare("2.0.0", "2.0.0-rc.1"), Greater);
        assert_eq!(compare("2.0.0-beta.2", "2.0.0-alpha.9"), Greater);
        assert_eq!(compare("1.0", "1.0.0"), Equal);
        assert_eq!(compare("v0.60.3", "0.60.3"), Equal);
        assert_eq!(compare("release-2024.10.03", "2024.9.30"), Greater);
        assert_eq!(compare("1.2.3+build9", "1.2.3"), Equal);
        assert_eq!(compare("1.0.1", "1.0-rc2"), Greater);
    }

    #[test]
    fn extraction() {
        assert_eq!(extract("App v1.2.3 (stable)", r"(\d+\.\d+\.\d+)", "1").as_deref(), Some("1.2.3"));
        assert_eq!(extract("build-12-34", r"(\d+)-(\d+)", "$1.$2").as_deref(), Some("12.34"));
    }

    #[test]
    fn prerelease_detect() {
        assert!(looks_prerelease("v2.0.0-rc1"));
        assert!(!looks_prerelease("v2.0.0"));
    }
}
