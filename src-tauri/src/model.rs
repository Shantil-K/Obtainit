//! Data model shared with the UI (serialised as snake_case JSON).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Github,
    Gitlab,
    /// Codeberg, Forgejo and Gitea share one API.
    Forgejo,
    /// lifter's HTML scraping method.
    Html,
    /// lifter's `api_json` method on any JSON endpoint.
    Json,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InstallKind {
    /// A single executable file (lifter's main use case).
    Binary,
    /// An archive: pull out binaries (lifter) or unpack the whole app.
    Archive,
    #[serde(rename = "appimage")]
    AppImage,
    Deb,
    Rpm,
    Pacman,
    Flatpak,
    Snap,
    /// Windows setup program (NSIS, Inno, Squirrel, ...).
    Installer,
    Msi,
    Msix,
    Unknown,
}

impl InstallKind {
    pub fn label(self) -> &'static str {
        match self {
            InstallKind::Binary => "Binary",
            InstallKind::Archive => "Archive",
            InstallKind::AppImage => "AppImage",
            InstallKind::Deb => "DEB package",
            InstallKind::Rpm => "RPM package",
            InstallKind::Pacman => "Arch package",
            InstallKind::Flatpak => "Flatpak bundle",
            InstallKind::Snap => "Snap",
            InstallKind::Installer => "Windows installer",
            InstallKind::Msi => "MSI installer",
            InstallKind::Msix => "MSIX package",
            InstallKind::Unknown => "Unknown",
        }
    }

    /// Installs that need administrator rights (a password prompt).
    pub fn needs_privileges(self) -> bool {
        matches!(
            self,
            InstallKind::Deb | InstallKind::Rpm | InstallKind::Pacman | InstallKind::Snap
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Asset {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub size: u64,
    /// sha256 hex, when the source publishes one (GitHub does).
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    pub kind: InstallKind,
    /// Platform score from lifter's asset scoring (higher = better fit).
    #[serde(default)]
    pub score: i32,
    /// Passed the app's filters and suits this machine.
    #[serde(default)]
    pub compatible: bool,
}

/// What a source returned, before filtering.
#[derive(Clone, Debug, Default)]
pub struct Release {
    pub tag: String,
    pub title: String,
    pub notes: String,
    pub url: String,
    pub published_at: Option<DateTime<Utc>>,
    pub prerelease: bool,
    pub commit: Option<String>,
    pub assets: Vec<RawAsset>,
}

#[derive(Clone, Debug, Default)]
pub struct RawAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Latest {
    pub version: String,
    #[serde(default)]
    pub commit: Option<String>,
    pub tag: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub release_url: String,
    #[serde(default)]
    pub published_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
    /// Name of the asset chosen for installing.
    #[serde(default)]
    pub selected: Option<String>,
    /// Newer releases skipped because of the minimum-age rule.
    #[serde(default)]
    pub held_back: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Installed {
    pub version: String,
    #[serde(default)]
    pub commit: Option<String>,
    pub kind: InstallKind,
    #[serde(default)]
    pub asset_name: String,
    /// Files and folders Obtainit created (removed on uninstall).
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub executable: Option<String>,
    /// Package name / Flatpak id / MSI product code / uninstall command.
    #[serde(default)]
    pub package_name: Option<String>,
    #[serde(default)]
    pub desktop_entry: Option<String>,
    pub installed_at: DateTime<Utc>,
    #[serde(default)]
    pub sha256: Option<String>,
    /// Marked as installed by hand rather than installed by Obtainit.
    #[serde(default)]
    pub manual: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct HtmlSettings {
    pub link_selector: String,
    pub link_text_regex: String,
    pub version_selector: String,
}

impl Default for HtmlSettings {
    fn default() -> Self {
        HtmlSettings {
            link_selector: "a".into(),
            link_text_regex: String::new(),
            version_selector: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct JsonSettings {
    pub version_path: String,
    pub commit_path: String,
    pub urls_path: String,
}

/// Per-app options. Most mirror Obtainium's "additional settings".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AppSettings {
    // Release selection
    pub include_prereleases: bool,
    pub fallback_to_older_releases: bool,
    pub verify_latest_tag: bool,
    pub release_title_filter: String,
    pub invert_title_filter: bool,
    pub release_notes_filter: String,
    pub invert_notes_filter: bool,
    /// `date`, `version` or `none` (API order).
    pub sort_method: String,
    pub use_latest_asset_date: bool,
    pub min_update_age_days: u32,

    // Version detection
    /// `tag`, `title` or `date`.
    pub version_source: String,
    pub version_extraction_regex: String,
    pub match_group: String,
    /// `newer` = only higher versions count as updates; `different` = any
    /// change (lifter's behaviour for tags like `nightly`, with commits).
    pub version_compare: String,

    // Asset selection
    pub asset_filter: String,
    pub invert_asset_filter: bool,
    pub auto_filter_platform: bool,
    /// `auto` or an [`InstallKind`] name to force.
    pub install_kind: String,

    // Install details (lifter's options + desktop extras)
    /// Files to pull out of an archive (regexes), lifter style.
    pub extract_targets: Vec<String>,
    /// lifter's `desired_filename`: rename for a single binary.
    pub desired_filename: String,
    /// `auto`, `binaries` (lifter targets) or `folder` (whole archive).
    pub archive_mode: String,
    /// For `.exe` files: `auto`, `portable` or `installer`.
    pub exe_mode: String,
    pub silent_install: bool,
    pub installer_args: String,
    pub create_shortcut: bool,
    pub launch_command: String,
    pub post_update_command: String,
    pub verify_checksum: bool,

    // Behaviour
    pub track_only: bool,
    pub exempt_from_background: bool,
    pub skip_notifications: bool,
    pub ignored_version: String,
    pub allow_insecure: bool,

    // Source-specific (lifter methods)
    pub html: HtmlSettings,
    pub json: JsonSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            include_prereleases: false,
            fallback_to_older_releases: true,
            verify_latest_tag: false,
            release_title_filter: String::new(),
            invert_title_filter: false,
            release_notes_filter: String::new(),
            invert_notes_filter: false,
            sort_method: "date".into(),
            use_latest_asset_date: false,
            min_update_age_days: 0,
            version_source: "tag".into(),
            version_extraction_regex: String::new(),
            match_group: String::new(),
            version_compare: "newer".into(),
            asset_filter: String::new(),
            invert_asset_filter: false,
            auto_filter_platform: true,
            install_kind: "auto".into(),
            extract_targets: vec![],
            desired_filename: String::new(),
            archive_mode: "auto".into(),
            exe_mode: "auto".into(),
            silent_install: true,
            installer_args: String::new(),
            create_shortcut: true,
            launch_command: String::new(),
            post_update_command: String::new(),
            verify_checksum: true,
            track_only: false,
            exempt_from_background: false,
            skip_notifications: false,
            ignored_version: String::new(),
            allow_insecure: false,
            html: HtmlSettings::default(),
            json: JsonSettings::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct App {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    pub source: SourceKind,
    pub url: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub settings: AppSettings,
    #[serde(default)]
    pub installed: Option<Installed>,
    #[serde(default)]
    pub latest: Option<Latest>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_checked: Option<DateTime<Utc>>,
    pub added_at: DateTime<Utc>,
    #[serde(default)]
    pub favorite: bool,
    /// Last version we notified about, so the same update is announced once.
    #[serde(default)]
    pub notified_version: Option<String>,
}

impl App {
    /// Is a newer release available than what is installed?
    pub fn has_update(&self) -> bool {
        let (Some(latest), Some(inst)) = (&self.latest, &self.installed) else {
            return false;
        };
        if latest.version.is_empty() {
            return false;
        }
        if !self.settings.ignored_version.is_empty() && self.settings.ignored_version == latest.version {
            return false;
        }
        if self.settings.version_compare == "different" {
            latest.version != inst.version
                || (latest.commit.is_some() && inst.commit.is_some() && latest.commit != inst.commit)
        } else if lifter_core::version::compare(&latest.version, &inst.version) == std::cmp::Ordering::Equal {
            // lifter: same version but a different commit is still new.
            latest.commit.is_some() && inst.commit.is_some() && latest.commit != inst.commit
        } else {
            lifter_core::version::is_newer(&latest.version, &inst.version)
        }
    }
}

/// A user-made color theme. Colors are `#rrggbb`.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct CustomTheme {
    pub id: String,
    pub name: String,
    /// Built on the dark (true) or light (false) base.
    pub dark: bool,
    pub background: String,
    pub surface: String,
    /// Empty = follow the app's accent color.
    pub accent: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    // Appearance
    /// `system`, `light` or `dark`.
    pub theme: String,
    pub pure_black: bool,
    pub seed_color: String,
    /// Accent (primary) color; empty = take it from `seed_color`.
    pub accent_color: String,
    /// Theme used in light mode: "" (Material default), "preset:<id>" or a custom theme id.
    pub light_theme: String,
    /// Theme used in dark mode, same format.
    pub dark_theme: String,
    pub custom_themes: Vec<CustomTheme>,
    /// `comfortable` or `compact`.
    pub list_density: String,
    pub sort_by: String,
    pub group_by_category: bool,

    // Updates
    /// Minutes between automatic checks; 0 = off.
    pub check_interval_minutes: u32,
    pub check_on_start: bool,
    pub background_updates: bool,
    pub notifications: bool,
    pub only_check_installed: bool,
    pub parallel_checks: u32,
    pub include_prereleases_by_default: bool,
    pub confirm_bulk_update: bool,

    // Obtainit's own updates
    pub self_update_check: bool,
    /// Install Obtainit updates by itself (and restart) when found.
    pub self_update_auto: bool,

    // System
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub autostart: bool,
    pub bin_dir: String,
    pub apps_dir: String,
    pub keep_downloads: bool,
    /// Preferred install types, best first.
    pub kind_priority: Vec<InstallKind>,

    // Accounts
    pub github_token: String,
    pub gitlab_token: String,
    pub forgejo_token: String,
}

impl Default for Settings {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let (bin_dir, apps_dir) = if cfg!(windows) {
            let base = dirs::data_local_dir().unwrap_or(home.clone()).join("Obtainit");
            (base.join("bin"), base.join("Apps"))
        } else {
            (home.join(".local").join("bin"), home.join("Applications"))
        };
        let kind_priority = if cfg!(windows) {
            vec![
                InstallKind::Installer,
                InstallKind::Msi,
                InstallKind::Archive,
                InstallKind::Binary,
                InstallKind::Msix,
            ]
        } else {
            vec![
                InstallKind::AppImage,
                InstallKind::Archive,
                InstallKind::Binary,
                InstallKind::Deb,
                InstallKind::Rpm,
                InstallKind::Pacman,
                InstallKind::Flatpak,
                InstallKind::Snap,
            ]
        };
        Settings {
            theme: "system".into(),
            pure_black: false,
            seed_color: "#6750A4".into(),
            accent_color: String::new(),
            light_theme: String::new(),
            dark_theme: String::new(),
            custom_themes: vec![],
            list_density: "comfortable".into(),
            sort_by: "status".into(),
            group_by_category: false,
            check_interval_minutes: 360,
            check_on_start: true,
            background_updates: false,
            notifications: true,
            only_check_installed: false,
            parallel_checks: 6,
            include_prereleases_by_default: false,
            confirm_bulk_update: true,
            self_update_check: true,
            self_update_auto: false,
            close_to_tray: true,
            start_minimized: false,
            autostart: false,
            bin_dir: bin_dir.to_string_lossy().into_owned(),
            apps_dir: apps_dir.to_string_lossy().into_owned(),
            keep_downloads: false,
            kind_priority,
            github_token: String::new(),
            gitlab_token: String::new(),
            forgejo_token: String::new(),
        }
    }
}
