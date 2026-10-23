//! Reading and writing lifter's `lifter.config` INI format, so existing
//! lifter setups can be imported and Obtainit apps exported back out.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use serde::Serialize;
use strfmt::strfmt;

/// One `[section]` with its template already applied.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Section {
    pub name: String,
    /// `api_json` or `html` (lifter's default).
    pub method: String,
    pub page_url: String,
    pub anchor_tag: String,
    pub anchor_text: String,
    pub version_tag: String,
    pub commit_tag: String,
    pub version: String,
    pub commit: String,
    /// `project` (owner/repo) when the section used one.
    pub project: String,
    pub template: String,
    pub targets: Vec<String>,
    pub desired_filename: String,
}

const TEMPLATE_KEYS: &[&str] = &["page_url", "anchor_tag", "version_tag", "commit_tag", "method"];

/// Parse lifter.config text. Template fields are substituted with
/// `{field}` values from the section, as lifter does.
pub fn parse(text: &str) -> Result<Vec<Section>> {
    let ini = tini::Ini::from_string(text).map_err(|e| anyhow!("invalid lifter.config: {e}"))?;
    let mut templates: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut names = Vec::new();
    for (name, _) in ini.iter() {
        if let Some(t) = name.strip_prefix("template:") {
            let m: HashMap<String, String> = ini
                .section_iter(name)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            templates.insert(t.to_string(), m);
        } else {
            names.push(name.clone());
        }
    }

    let mut out = Vec::new();
    for name in names {
        let vals: HashMap<String, String> = ini
            .section_iter(&name)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut merged: HashMap<String, String> = HashMap::new();
        if let Some(t) = vals.get("template") {
            let tf = templates
                .get(t)
                .ok_or_else(|| anyhow!("[{name}] uses unknown template {t:?}"))?;
            for k in TEMPLATE_KEYS {
                if let Some(v) = tf.get(*k) {
                    merged.insert(k.to_string(), strfmt(v, &vals).unwrap_or_else(|_| v.clone()));
                }
            }
        }
        for (k, v) in &vals {
            merged.insert(k.clone(), strfmt(v, &vals).unwrap_or_else(|_| v.clone()));
        }
        let get = |k: &str| merged.get(k).cloned().unwrap_or_default();
        let targets = if let Some(p) = merged.get("target_filenames_to_extract_from_archive") {
            serde_json::from_str::<Vec<String>>(p).unwrap_or_else(|_| vec![p.clone()])
        } else if let Some(s) = merged.get("target_filename_to_extract_from_archive") {
            vec![s.clone()]
        } else {
            vec![]
        };
        out.push(Section {
            method: if get("method").is_empty() { "html".into() } else { get("method") },
            page_url: get("page_url"),
            anchor_tag: get("anchor_tag"),
            anchor_text: get("anchor_text"),
            version_tag: get("version_tag"),
            commit_tag: get("commit_tag"),
            version: get("version"),
            commit: get("commit"),
            project: get("project"),
            template: get("template"),
            desired_filename: get("desired_filename"),
            targets,
            name,
        });
    }
    Ok(out)
}

pub const GITHUB_API_TEMPLATE: &str = "[template:github_api_latest]\nmethod = api_json\npage_url = https://api.github.com/repos/{project}/releases/latest\nversion_tag = $.tag_name\nanchor_tag = $.assets.*.browser_download_url\n";

/// Render sections back to lifter.config text.
pub fn render(sections: &[Section]) -> String {
    let mut s = String::from("# Exported by Obtainit. Usable with lifter (github.com/cjrh/lifter).\n\n");
    if sections.iter().any(|x| x.template == "github_api_latest") {
        s.push_str(GITHUB_API_TEMPLATE);
        s.push('\n');
    }
    for x in sections {
        s.push_str(&format!("[{}]\n", x.name));
        let mut kv = |k: &str, v: &str| {
            if !v.is_empty() {
                s.push_str(&format!("{k} = {v}\n"));
            }
        };
        if x.template == "github_api_latest" {
            kv("template", &x.template);
            kv("project", &x.project);
        } else {
            kv("method", if x.method == "html" { "" } else { &x.method });
            kv("page_url", &x.page_url);
            kv("anchor_tag", &x.anchor_tag);
            kv("version_tag", &x.version_tag);
            kv("commit_tag", &x.commit_tag);
        }
        kv("anchor_text", &x.anchor_text);
        match x.targets.as_slice() {
            [] => {}
            [one] => kv("target_filename_to_extract_from_archive", one),
            many => kv(
                "target_filenames_to_extract_from_archive",
                &serde_json::to_string(many).unwrap_or_default(),
            ),
        }
        kv("desired_filename", &x.desired_filename);
        kv("version", &x.version);
        kv("commit", &x.commit);
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lifter_sample() {
        let text = format!(
            "{GITHUB_API_TEMPLATE}\n[ripgrep]\ntemplate = github_api_latest\nproject = BurntSushi/ripgrep\nanchor_text = ripgrep-(\\d+\\.\\d+\\.\\d+)-x86_64-unknown-linux-musl.tar.gz\ntarget_filename_to_extract_from_archive = rg\nversion = 14.1.1\n"
        );
        let s = parse(&text).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].page_url, "https://api.github.com/repos/BurntSushi/ripgrep/releases/latest");
        assert_eq!(s[0].method, "api_json");
        assert_eq!(s[0].targets, vec!["rg"]);
        let back = parse(&render(&s)).unwrap();
        assert_eq!(back[0].project, "BurntSushi/ripgrep");
    }
}
