// Mirrors src-tauri/src/model.rs (snake_case JSON).

export type SourceKind = "github" | "gitlab" | "forgejo" | "html" | "json";

export type InstallKind =
  | "binary"
  | "archive"
  | "appimage"
  | "deb"
  | "rpm"
  | "pacman"
  | "flatpak"
  | "snap"
  | "installer"
  | "msi"
  | "msix"
  | "unknown";

export interface Asset {
  name: string;
  url: string;
  size: number;
  sha256: string | null;
  updated_at: string | null;
  kind: InstallKind;
  score: number;
  compatible: boolean;
}

export interface Latest {
  version: string;
  commit: string | null;
  tag: string;
  title: string;
  notes: string;
  release_url: string;
  published_at: string | null;
  prerelease: boolean;
  assets: Asset[];
  selected: string | null;
  held_back: string | null;
}

export interface Installed {
  version: string;
  commit: string | null;
  kind: InstallKind;
  asset_name: string;
  paths: string[];
  executable: string | null;
  package_name: string | null;
  desktop_entry: string | null;
  installed_at: string;
  sha256: string | null;
  manual: boolean;
}

export interface AppSettings {
  include_prereleases: boolean;
  fallback_to_older_releases: boolean;
  verify_latest_tag: boolean;
  release_title_filter: string;
  invert_title_filter: boolean;
  release_notes_filter: string;
  invert_notes_filter: boolean;
  sort_method: string;
  use_latest_asset_date: boolean;
  min_update_age_days: number;
  version_source: string;
  version_extraction_regex: string;
  match_group: string;
  version_compare: string;
  asset_filter: string;
  invert_asset_filter: boolean;
  auto_filter_platform: boolean;
  install_kind: string;
  extract_targets: string[];
  desired_filename: string;
  archive_mode: string;
  exe_mode: string;
  silent_install: boolean;
  installer_args: string;
  create_shortcut: boolean;
  launch_command: string;
  post_update_command: string;
  verify_checksum: boolean;
  track_only: boolean;
  exempt_from_background: boolean;
  skip_notifications: boolean;
  ignored_version: string;
  allow_insecure: boolean;
  html: { link_selector: string; link_text_regex: string; version_selector: string };
  json: { version_path: string; commit_path: string; urls_path: string };
}

export interface App {
  id: string;
  name: string;
  author: string;
  source: SourceKind;
  url: string;
  icon_url: string | null;
  description: string;
  categories: string[];
  settings: AppSettings;
  installed: Installed | null;
  latest: Latest | null;
  last_error: string | null;
  last_checked: string | null;
  added_at: string;
  favorite: boolean;
  notified_version: string | null;
}

export interface Settings {
  theme: "system" | "light" | "dark";
  pure_black: boolean;
  seed_color: string;
  accent_color: string;
  light_theme: string;
  dark_theme: string;
  custom_themes: CustomTheme[];
  list_density: "comfortable" | "compact";
  sort_by: string;
  group_by_category: boolean;
  check_interval_minutes: number;
  check_on_start: boolean;
  background_updates: boolean;
  notifications: boolean;
  only_check_installed: boolean;
  parallel_checks: number;
  include_prereleases_by_default: boolean;
  confirm_bulk_update: boolean;
  self_update_check: boolean;
  self_update_auto: boolean;
  close_to_tray: boolean;
  start_minimized: boolean;
  autostart: boolean;
  bin_dir: string;
  apps_dir: string;
  keep_downloads: boolean;
  kind_priority: InstallKind[];
  github_token: string;
  gitlab_token: string;
  forgejo_token: string;
}

export interface SystemInfo {
  os: string;
  arch: string;
  distro: string;
  supported_kinds: InstallKind[];
  has_pkexec: boolean;
  bin_dir_in_path: boolean;
}

export interface Meta {
  name: string;
  author: string;
  description: string;
  icon_url: string | null;
  url: string;
}

export interface Preview {
  source: SourceKind;
  url: string;
  meta: Meta;
  latest: Latest | null;
  error: string | null;
}

export interface Progress {
  id: string;
  stage: "checking" | "downloading" | "verifying" | "installing" | "removing" | "done" | "error";
  received: number;
  total: number | null;
  message: string | null;
}

export interface ImportReport {
  format: string;
  added: App[];
  skipped: string[];
}

export interface LogLine {
  time: string;
  level: string;
  message: string;
}

export type Status = "update" | "installed" | "not_installed" | "track" | "error";

export interface CustomTheme {
  id: string;
  name: string;
  dark: boolean;
  background: string;
  surface: string;
  /** Empty = follow the app's accent color. */
  accent: string;
  text: string;
}

export interface SelfUpdate {
  current: string;
  latest: string | null;
  available: boolean;
  method: "app_image" | "deb" | "rpm" | "nsis" | "msi" | "portable" | "dev";
  can_install: boolean;
  asset: Asset | null;
  notes: string;
  release_url: string;
  error: string | null;
  checked_at: string | null;
}
