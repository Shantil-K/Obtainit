import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { applyTheme } from "./theme";
import type { App, CustomTheme, Progress, SelfUpdate, Settings, Status, SystemInfo } from "./types";

export type Route =
  | { page: "apps" }
  | { page: "add"; url?: string }
  | { page: "app"; id: string }
  | { page: "import" }
  | { page: "settings" }
  | { page: "theme"; id?: string; dark?: boolean; from?: string }
  | { page: "logs" };

interface Snack {
  id: number;
  text: string;
  action?: { label: string; run: () => void };
  error?: boolean;
}

class Store {
  apps = $state<App[]>([]);
  settings = $state<Settings | null>(null);
  system = $state<SystemInfo | null>(null);
  version = $state("");
  progress = $state<Record<string, Progress>>({});
  route = $state<Route>({ page: "apps" });
  history: Route[] = [];
  snack = $state<Snack | null>(null);
  checkingAll = $state(false);
  ready = $state(false);
  selfUpdate = $state<SelfUpdate | null>(null);
  selfUpdating = $state(false);

  updatesCount = $derived(this.apps.filter((a) => statusOf(a) === "update").length);

  async load() {
    const b = await api.bootstrap();
    this.apps = b.apps;
    this.settings = b.settings;
    this.system = b.system;
    this.version = b.version;
    for (const id of b.busy) this.progress[id] = { id, stage: "installing", received: 0, total: null, message: null };
    this.applyTheme();
    this.ready = true;
    api.selfStatus().then((s) => (this.selfUpdate = s)).catch(() => {});
  }

  async reloadApps() {
    const b = await api.bootstrap();
    this.apps = b.apps;
  }

  /** Theme being edited, previewed on the whole app until saved/cancelled. */
  previewTheme = $state<CustomTheme | null>(null);

  applyTheme() {
    const s = this.settings;
    if (s) applyTheme($state.snapshot(s) as Settings, (this.previewTheme ? $state.snapshot(this.previewTheme) : undefined) as CustomTheme | undefined);
  }

  async saveSettings(patch: Partial<Settings>) {
    if (!this.settings) return;
    const next = { ...$state.snapshot(this.settings), ...patch } as Settings;
    this.settings = next;
    this.applyTheme();
    try {
      this.settings = await api.saveSettings(next);
    } catch (e) {
      this.toast(String(e), true);
    }
  }

  upsert(app: App) {
    const i = this.apps.findIndex((a) => a.id === app.id);
    if (i >= 0) this.apps[i] = app;
    else this.apps.push(app);
  }

  get(id: string) {
    return this.apps.find((a) => a.id === id);
  }

  go(r: Route) {
    this.history.push($state.snapshot(this.route) as Route);
    if (this.history.length > 30) this.history.shift();
    this.route = r;
    document.querySelector(".content")?.scrollTo({ top: 0 });
  }

  back() {
    this.route = this.history.pop() ?? { page: "apps" };
  }

  toast(text: string, error = false, action?: Snack["action"]) {
    const id = Date.now();
    this.snack = { id, text, error, action };
    setTimeout(() => {
      if (this.snack?.id === id) this.snack = null;
    }, error ? 8000 : 4500);
  }

  /** Run an action, showing errors as a snackbar. */
  async run<T>(fn: () => Promise<T>, ok?: string): Promise<T | undefined> {
    try {
      const r = await fn();
      if (ok) this.toast(ok);
      return r;
    } catch (e) {
      this.toast(String(e), true);
      return undefined;
    }
  }

  async checkSelf() {
    const r = await this.run(() => api.selfCheck());
    if (r) {
      this.selfUpdate = r;
      if (!r.available && !r.error) this.toast(`Obtainit ${r.current} is the latest version`);
    }
  }

  async updateSelf() {
    this.selfUpdating = true;
    const ok = await this.run(() => api.selfApply());
    // On success the app restarts; we only get here on failure.
    if (ok === undefined) this.selfUpdating = false;
  }

  async checkAll() {
    if (this.checkingAll) return;
    this.checkingAll = true;
    try {
      await api.check();
      const n = this.updatesCount;
      this.toast(n ? `${n} update${n === 1 ? "" : "s"} available` : "Everything is up to date");
    } catch (e) {
      this.toast(String(e), true);
    } finally {
      this.checkingAll = false;
    }
  }
}

export const store = new Store();

export function statusOf(a: App): Status {
  if (a.installed && hasUpdate(a)) return "update";
  if (a.last_error && !a.latest) return "error";
  if (a.installed) return "installed";
  if (a.settings.track_only) return "track";
  return "not_installed";
}

/** Same rules as App::has_update in Rust (for instant UI feedback). */
export function hasUpdate(a: App): boolean {
  const l = a.latest;
  const i = a.installed;
  if (!l || !i || !l.version) return false;
  if (a.settings.ignored_version && a.settings.ignored_version === l.version) return false;
  const commitChanged = !!l.commit && !!i.commit && l.commit !== i.commit;
  if (a.settings.version_compare === "different") return l.version !== i.version || commitChanged;
  const c = compareVersions(l.version, i.version);
  return c > 0 || (c === 0 && commitChanged);
}

const PRE: Record<string, number> = { dev: -5, snapshot: -5, nightly: -5, canary: -5, a: -4, alpha: -4, b: -3, beta: -3, pre: -2, preview: -2, rc: -1, cr: -1 };

function tokens(v: string): (number | string)[] {
  let s = v.trim().replace(/^(release[-_]|version[-_]?|rel-|ver)/i, "").replace(/^[vV\-_ ]+/, "");
  s = s.split("+")[0];
  return (s.match(/\d+|[a-zA-Z]+/g) ?? []).map((t) => (/^\d+$/.test(t) ? Number(t) : t.toLowerCase()));
}

function tail(t: number | string): number {
  if (typeof t === "number") return t === 0 ? 0 : 1;
  return t in PRE ? -1 : 1;
}

export function compareVersions(a: string, b: string): number {
  const ta = tokens(a), tb = tokens(b);
  for (let i = 0; ; i++) {
    const x = ta[i], y = tb[i];
    if (x === undefined && y === undefined) return 0;
    if (y === undefined) return tail(x);
    if (x === undefined) return -tail(y);
    if (typeof x === "number" && typeof y === "number") {
      if (x !== y) return x > y ? 1 : -1;
    } else if (typeof x === "string" && typeof y === "string") {
      if (x !== y) {
        if (x in PRE && y in PRE) return PRE[x] > PRE[y] ? 1 : -1;
        return x > y ? 1 : -1;
      }
    } else if (typeof x === "number") {
      return (y as string) in PRE ? 1 : -1;
    } else {
      return (x as string) in PRE ? -1 : 1;
    }
  }
}

export async function startListeners() {
  await listen<App>("app-changed", (e) => store.upsert(e.payload));
  await listen<string>("app-removed", (e) => {
    store.apps = store.apps.filter((a) => a.id !== e.payload);
  });
  await listen("apps-reloaded", () => store.reloadApps());
  await listen<SelfUpdate>("self-update", (e) => (store.selfUpdate = e.payload));
  await listen<Settings>("settings-changed", (e) => {
    store.settings = e.payload;
    store.applyTheme();
  });
  await listen<Progress>("progress", (e) => {
    const p = e.payload;
    if (p.stage === "done" || p.stage === "error") {
      delete store.progress[p.id];
    } else {
      store.progress[p.id] = p;
    }
  });
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => store.applyTheme());
}
