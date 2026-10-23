import { invoke } from "@tauri-apps/api/core";
import type {
  App,
  AppSettings,
  ImportReport,
  LogLine,
  Preview,
  SelfUpdate,
  Settings,
  SourceKind,
  SystemInfo,
} from "./types";

export interface Bootstrap {
  apps: App[];
  settings: Settings;
  system: SystemInfo;
  version: string;
  busy: string[];
}

export interface NewApp {
  url: string;
  source: SourceKind;
  name: string;
  author?: string;
  description?: string;
  icon_url?: string | null;
  categories?: string[];
  settings: AppSettings;
  installed_version?: string | null;
}

export interface AppPatch {
  name?: string;
  categories?: string[];
  settings?: AppSettings;
  favorite?: boolean;
  source?: SourceKind;
  url?: string;
}

export const api = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  preview: (url: string, source: SourceKind | null, settings: AppSettings) =>
    invoke<Preview>("preview_app", { url, source, settings }),
  add: (app: NewApp) => invoke<App>("add_app", { app }),
  update: (id: string, patch: AppPatch) => invoke<App>("update_app", { id, patch }),
  remove: (id: string, uninstall: boolean) => invoke<void>("delete_app", { id, uninstall }),
  check: (ids?: string[]) => invoke<App[]>("check_apps", { ids: ids ?? null }),
  install: (id: string, asset?: string) => invoke<App>("install_app", { id, asset: asset ?? null }),
  updateAll: () => invoke<string[]>("update_all"),
  uninstall: (id: string) => invoke<App>("uninstall_app", { id }),
  markInstalled: (id: string, version?: string) =>
    invoke<App>("mark_installed", { id, version: version ?? null }),
  markNotInstalled: (id: string) => invoke<App>("mark_not_installed", { id }),
  ignoreVersion: (id: string, version: string | null) => invoke<App>("ignore_version", { id, version }),
  preferAsset: (id: string, asset: string) => invoke<App>("prefer_asset", { id, asset }),
  launch: (id: string) => invoke<void>("launch_app", { id }),
  canLaunch: (id: string) => invoke<boolean>("can_launch", { id }),
  folder: (id: string) => invoke<string | null>("app_folder", { id }),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  systemInfo: () => invoke<SystemInfo>("system_info"),
  addBinToPath: () => invoke<string>("add_bin_to_path"),
  importFile: (path: string) => invoke<ImportReport>("import_file", { path }),
  importText: (text: string) => invoke<ImportReport>("import_text", { text }),
  exportFile: (path: string, format: "obtainit" | "lifter") => invoke<void>("export_file", { path, format }),
  logs: () => invoke<LogLine[]>("get_logs"),
  clearLogs: () => invoke<void>("clear_logs"),
  logPath: () => invoke<string>("log_path"),
  selfStatus: () => invoke<SelfUpdate | null>("self_update_status"),
  selfCheck: () => invoke<SelfUpdate>("self_update_check"),
  selfApply: () => invoke<void>("self_update_apply"),
};
