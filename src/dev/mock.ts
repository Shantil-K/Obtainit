// Fake backend for previewing the UI in a plain browser:
//   VITE_MOCK=1 npx vite
// Never included in real builds.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { App, AppSettings, Asset, Settings } from "../lib/types";

const base: AppSettings = {
  include_prereleases: false, fallback_to_older_releases: true, verify_latest_tag: false,
  release_title_filter: "", invert_title_filter: false, release_notes_filter: "", invert_notes_filter: false,
  sort_method: "date", use_latest_asset_date: false, min_update_age_days: 0, version_source: "tag",
  version_extraction_regex: "", match_group: "", version_compare: "newer", asset_filter: "",
  invert_asset_filter: false, auto_filter_platform: true, install_kind: "auto", extract_targets: [],
  desired_filename: "", archive_mode: "auto", exe_mode: "auto", silent_install: true, installer_args: "",
  create_shortcut: true, launch_command: "", post_update_command: "", verify_checksum: true, track_only: false,
  exempt_from_background: false, skip_notifications: false, ignored_version: "", allow_insecure: false,
  html: { link_selector: "a", link_text_regex: "", version_selector: "" },
  json: { version_path: "", commit_path: "", urls_path: "" },
};

const asset = (name: string, kind: Asset["kind"], size: number, compatible: boolean, score = 40): Asset => ({
  name, url: "https://example.com/" + name, size, sha256: compatible ? "ab12" : null, updated_at: null, kind, score, compatible,
});

const day = (n: number) => new Date(Date.now() - n * 86400e3).toISOString();

const notes = `## What's new\n\n- Faster searching in large repositories\n- New \`--hyperlink-format\` flag\n- Fixed a crash with \`.gitignore\` files containing \`**\`\n\n### Contributors\n\nThanks to everyone who helped!`;

function app(p: Partial<App> & { name: string }): App {
  return {
    id: p.name.toLowerCase(), author: "", source: "github", url: `https://github.com/x/${p.name}`, icon_url: null,
    description: "", categories: [], settings: base, installed: null, latest: null, last_error: null,
    last_checked: day(0.01), added_at: day(10), favorite: false, notified_version: null, ...p,
  };
}

const latest = (version: string, assets: Asset[], ago = 3) => ({
  version, commit: null, tag: "v" + version, title: "v" + version, notes, release_url: "https://github.com",
  published_at: day(ago), prerelease: false, assets, selected: assets.find((a) => a.compatible)?.name ?? null, held_back: null,
});

const inst = (version: string, kind: Asset["kind"], exe: string) => ({
  version, commit: null, kind, asset_name: "", paths: [exe], executable: exe, package_name: null,
  desktop_entry: null, installed_at: day(20), sha256: null, manual: false,
});

const apps: App[] = [
  app({
    name: "ripgrep", author: "BurntSushi", categories: ["Dev tools"],
    icon_url: "https://github.com/BurntSushi.png?size=96",
    installed: inst("14.0.3", "archive", "/home/me/.local/bin/rg"),
    latest: latest("14.1.1", [
      asset("ripgrep-14.1.1-x86_64-unknown-linux-musl.tar.gz", "archive", 2_620_000, true, 71),
      asset("ripgrep_14.1.1-1_amd64.deb", "deb", 1_550_000, true, 59),
      asset("ripgrep-14.1.1-aarch64-unknown-linux-gnu.tar.gz", "archive", 2_400_000, false, -3),
      asset("ripgrep-14.1.1-x86_64-pc-windows-msvc.zip", "archive", 2_100_000, false, -8),
    ]),
  }),
  app({
    name: "Obsidian", author: "obsidianmd", categories: ["Notes"], favorite: true,
    installed: inst("1.7.7", "appimage", "/home/me/Applications/obsidian.AppImage"),
    latest: latest("1.7.7", [asset("Obsidian-1.7.7.AppImage", "appimage", 118_000_000, true)], 12),
  }),
  app({
    name: "LocalSend", author: "localsend", categories: ["Network"],
    installed: inst("1.15.4", "deb", "/usr/bin/localsend"),
    latest: latest("1.16.1", [asset("LocalSend-1.16.1-linux-x86-64.deb", "deb", 14_000_000, true)], 1),
  }),
  app({ name: "neovim", author: "neovim", categories: ["Dev tools"],
    latest: latest("0.10.2", [asset("nvim-linux64.tar.gz", "archive", 11_000_000, true), asset("nvim.appimage", "appimage", 10_500_000, true)], 30) }),
  app({ name: "Forgejo", author: "forgejo", source: "forgejo", url: "https://codeberg.org/forgejo/forgejo",
    settings: { ...base, track_only: true }, latest: latest("9.0.1", [], 5) }),
  app({ name: "7-Zip", author: "7-zip.org", source: "html", url: "https://www.7-zip.org/download.html",
    last_error: "fetching https://www.7-zip.org/download.html: status 503: Service Unavailable" }),
];

const settings: Settings = {
  theme: "system", pure_black: false, seed_color: "#6750A4", accent_color: "", light_theme: "", dark_theme: "", custom_themes: [], list_density: "comfortable", sort_by: "status",
  group_by_category: false, check_interval_minutes: 360, check_on_start: true, background_updates: false,
  notifications: true, only_check_installed: false, parallel_checks: 6, include_prereleases_by_default: false,
  confirm_bulk_update: true, self_update_check: true, self_update_auto: false, close_to_tray: true, start_minimized: false, autostart: false,
  bin_dir: "/home/me/.local/bin", apps_dir: "/home/me/Applications", keep_downloads: false,
  kind_priority: ["appimage", "archive", "binary", "deb", "rpm", "pacman", "flatpak", "snap"],
  github_token: "", gitlab_token: "", forgejo_token: "",
};

const system = {
  os: "linux", arch: "x86_64", distro: "Ubuntu 24.04.3 LTS",
  supported_kinds: ["binary", "archive", "appimage", "deb", "flatpak", "snap"], has_pkexec: true, bin_dir_in_path: true,
};

mockWindows("main");
mockIPC(
  (cmd, args) => {
    const a = args as Record<string, unknown>;
    switch (cmd) {
      case "bootstrap":
        return { apps, settings, system, version: "0.1.0", busy: [] };
      case "can_launch":
        return a.id !== "ripgrep";
      case "self_update_status":
        return {
          current: "0.1.0", latest: "0.2.0", available: true, method: "app_image", can_install: true, asset: null,
          notes: "", release_url: "https://github.com/Shantil-K/Obtainit/releases", error: null, checked_at: day(0.01),
        };
      case "save_settings":
        return a.settings;
      case "get_logs":
        return [
          { time: "2026-10-23 19:40:01", level: "INFO", message: "background check of 6 apps" },
          { time: "2026-10-23 19:40:02", level: "WARN", message: "[7-Zip] check failed: status 503" },
        ];
      case "preview_app":
        return {
          source: "github", url: "https://github.com/sharkdp/bat",
          meta: { name: "bat", author: "sharkdp", description: "A cat(1) clone with wings.", icon_url: "https://github.com/sharkdp.png?size=96", url: "https://github.com/sharkdp/bat" },
          latest: latest("0.24.0", [
            asset("bat-v0.24.0-x86_64-unknown-linux-musl.tar.gz", "archive", 2_900_000, true),
            asset("bat_0.24.0_amd64.deb", "deb", 2_100_000, true),
            asset("bat-v0.24.0-x86_64-pc-windows-msvc.zip", "archive", 2_700_000, false),
          ]),
          error: null,
        };
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);

// ?page=app&id=ripgrep, ?page=settings, ?theme=dark, ?seed=%23386A20
const params = new URLSearchParams(location.search);
setTimeout(async () => {
  const { store } = await import("../lib/state.svelte");
  const page = params.get("page");
  if (page === "app") store.route = { page: "app", id: params.get("id") ?? "ripgrep" };
  else if (page === "add") store.route = { page: "add", url: params.get("url") ?? "" };
  else if (page) store.route = { page } as never;
  const theme = params.get("theme");
  const seed = params.get("seed");
  if (theme || seed) {
    await store.saveSettings({
      ...(theme ? { theme: theme as Settings["theme"] } : {}),
      ...(seed ? { seed_color: seed } : {}),
    });
  }
}, 400);
