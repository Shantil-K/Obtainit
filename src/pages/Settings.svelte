<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { store } from "../lib/state.svelte";
  import { api } from "../lib/api";
  import { ACCENTS, SEEDS, allThemes, computeVars } from "../lib/theme";
  import ThemeCard from "../components/ThemeCard.svelte";
  import ColorField from "../components/ColorField.svelte";
  import { KIND_LABEL, ago } from "../lib/format";
  import type { CustomTheme, InstallKind, Settings } from "../lib/types";
  import TopBar from "../components/TopBar.svelte";
  import SwitchRow from "../components/SwitchRow.svelte";

  const s = $derived(store.settings!);
  const su = $derived(store.selfUpdate);
  const selfProg = $derived(store.progress["obtainit-self"]);
  const METHOD: Record<string, string> = {
    app_image: "AppImage", deb: "DEB package", rpm: "RPM package", nsis: "Windows installer",
    msi: "MSI", portable: "portable copy", dev: "development build",
  };
  const sys = $derived(store.system);
  const isWin = $derived(sys?.os === "windows");
  let pathHelp = $state("");
  let showTokens = $state(false);

  const intervals = [
    { v: 0, l: "Never (manual only)" },
    { v: 15, l: "Every 15 minutes" },
    { v: 30, l: "Every 30 minutes" },
    { v: 60, l: "Every hour" },
    { v: 180, l: "Every 3 hours" },
    { v: 360, l: "Every 6 hours" },
    { v: 720, l: "Every 12 hours" },
    { v: 1440, l: "Once a day" },
    { v: 10080, l: "Once a week" },
  ];

  // Theme slots. Older "pure black" settings count as the Black (OLED) template.
  function slotOf(dark: boolean) {
    return dark ? s.dark_theme || (s.pure_black ? "preset:black" : "") : s.light_theme;
  }

  function themesFor(dark: boolean) {
    return allThemes(s).filter((t) => t.dark === dark);
  }

  function previewVars(dark: boolean, id: string) {
    const mode: Settings["theme"] = dark ? "dark" : "light";
    const fake = { ...s, theme: mode, pure_black: false, [dark ? "dark_theme" : "light_theme"]: id } as Settings;
    return computeVars(fake).vars;
  }

  function pickTheme(dark: boolean, id: string) {
    const patch: Partial<Settings> = dark ? { dark_theme: id, pure_black: false } : { light_theme: id };
    // Switch to that mode unless following the system.
    if (s.theme !== "system") patch.theme = dark ? "dark" : "light";
    store.saveSettings(patch);
  }

  function deleteTheme(t: CustomTheme) {
    const before = $state.snapshot(s.custom_themes) as CustomTheme[];
    const patch: Partial<Settings> = { custom_themes: before.filter((x) => x.id !== t.id) };
    if (s.dark_theme === t.id) patch.dark_theme = "";
    if (s.light_theme === t.id) patch.light_theme = "";
    const slots = { dark_theme: s.dark_theme, light_theme: s.light_theme };
    store.saveSettings(patch);
    store.toast(`Deleted "${t.name}"`, false, {
      label: "Undo",
      run: () => store.saveSettings({ custom_themes: before, ...slots }),
    });
  }

  function set<K extends keyof Settings>(k: K, v: Settings[K]) {
    store.saveSettings({ [k]: v } as Partial<Settings>);
  }

  async function browse(k: "bin_dir" | "apps_dir") {
    const dir = await openDialog({ directory: true, defaultPath: s[k] });
    if (typeof dir === "string") set(k, dir);
  }

  function moveKind(i: number, d: -1 | 1) {
    const list = [...s.kind_priority];
    const j = i + d;
    if (j < 0 || j >= list.length) return;
    [list[i], list[j]] = [list[j], list[i]];
    set("kind_priority", list);
  }

  const priority = $derived<InstallKind[]>(s.kind_priority.filter((k) => sys?.supported_kinds.includes(k)));

  async function addPath() {
    const r = await store.run(() => api.addBinToPath());
    if (r) pathHelp = r;
    store.system = await api.systemInfo();
  }

  function val(e: Event) {
    return (e.target as HTMLInputElement).value;
  }
</script>

<TopBar title="Settings" />

<div class="page narrow">
  <div class="section-title"><md-icon>palette</md-icon>Appearance</div>
  <div class="card">
    <div class="setting">
      <div class="text">
        <div class="label">Mode</div>
        <div class="desc">Auto follows your system's light/dark setting.</div>
      </div>
      <div class="segmented">
        {#each [["system", "Auto", "brightness_auto"], ["light", "Light", "light_mode"], ["dark", "Dark", "dark_mode"]] as [v, l, i]}
          <button class:on={s.theme === v} onclick={() => set("theme", v as Settings["theme"])}>
            <md-icon>{s.theme === v ? "check" : i}</md-icon>{l}
          </button>
        {/each}
      </div>
    </div>

    {#each [false, true] as dark}
      <div class="setting">
        <div class="text">
          <div class="label">{dark ? "Dark mode theme" : "Light mode theme"}</div>
          <div class="desc">{dark ? "Used in Dark mode, and in Auto when your system is dark." : "Used in Light mode, and in Auto when your system is light."}</div>
        </div>
        <md-text-button onclick={() => store.go({ page: "theme", dark })}>
          <md-icon slot="icon">add</md-icon>Create
        </md-text-button>
      </div>
      <div class="gallery">
        <ThemeCard
          name="Material You"
          badge="Default"
          vars={previewVars(dark, "")}
          selected={slotOf(dark) === ""}
          onselect={() => pickTheme(dark, "")}
        />
        {#each themesFor(dark) as t (t.id)}
          <ThemeCard
            name={t.name}
            vars={previewVars(dark, t.id)}
            selected={slotOf(dark) === t.id}
            onselect={() => pickTheme(dark, t.id)}
          >
            {#snippet actions()}
              {#if t.id.startsWith("custom:")}
                <md-icon-button title="Edit" aria-label="Edit {t.name}" onclick={() => store.go({ page: "theme", id: t.id, dark })}>
                  <md-icon>edit</md-icon>
                </md-icon-button>
                <md-icon-button title="Delete" aria-label="Delete {t.name}" onclick={() => deleteTheme(t)}>
                  <md-icon>delete</md-icon>
                </md-icon-button>
              {:else}
                <md-icon-button title="Customize a copy" aria-label="Customize a copy of {t.name}" onclick={() => store.go({ page: "theme", dark, from: t.id })}>
                  <md-icon>content_copy</md-icon>
                </md-icon-button>
              {/if}
            {/snippet}
          </ThemeCard>
        {/each}
      </div>
    {/each}
  </div>

  <div class="section-title"><md-icon>colors</md-icon>Colors</div>
  <div class="card">
    <div class="setting">
      <div class="text">
        <div class="label">Palette color</div>
        <div class="desc">The Material You themes build every color from this one.</div>
      </div>
    </div>
    <div class="swatches">
      {#each SEEDS as c}
        <button class="swatch" class:on={s.seed_color.toLowerCase() === c.toLowerCase()} style="--c:{c}" aria-label="Palette color {c}"
          onclick={() => set("seed_color", c)}>
          {#if s.seed_color.toLowerCase() === c.toLowerCase()}<md-icon>check</md-icon>{/if}
        </button>
      {/each}
    </div>
    <ColorField label="Custom palette color" desc="Pick any color from the full RGB spectrum." value={s.seed_color}
      swatches={SEEDS} onchange={(v) => v && set("seed_color", v)} />

    <md-divider></md-divider>

    <div class="setting">
      <div class="text">
        <div class="label">Accent color</div>
        <div class="desc">Buttons, highlights and selections. Leave on "Match palette" to use the palette color.</div>
      </div>
    </div>
    <div class="swatches">
      <button class="swatch auto" class:on={!s.accent_color} aria-label="Match palette" title="Match palette"
        onclick={() => set("accent_color", "")}>
        <md-icon>{s.accent_color ? "auto_awesome" : "check"}</md-icon>
      </button>
      {#each ACCENTS as c}
        <button class="swatch" class:on={s.accent_color.toLowerCase() === c.toLowerCase()} style="--c:{c}" aria-label="Accent {c}"
          onclick={() => set("accent_color", c)}>
          {#if s.accent_color.toLowerCase() === c.toLowerCase()}<md-icon>check</md-icon>{/if}
        </button>
      {/each}
    </div>
    <ColorField label="Custom accent color" desc="Pick any color from the full RGB spectrum." value={s.accent_color}
      swatches={ACCENTS} allowEmpty="Match palette" onchange={(v) => set("accent_color", v)} />

    <md-divider></md-divider>
    <SwitchRow label="Compact list" checked={s.list_density === "compact"} onchange={(v) => set("list_density", v ? "compact" : "comfortable")} />
    <SwitchRow label="Group apps by category" checked={s.group_by_category} onchange={(v) => set("group_by_category", v)} />
  </div>

  <div class="section-title"><md-icon>update</md-icon>Updates</div>
  <div class="card">
    <md-outlined-select label="Check for updates" value={String(s.check_interval_minutes)}
      onchange={(e: Event) => set("check_interval_minutes", Number(val(e)))}>
      {#each intervals as i}
        <md-select-option value={String(i.v)}><div slot="headline">{i.l}</div></md-select-option>
      {/each}
    </md-outlined-select>
    <SwitchRow label="Check when Obtainit starts" checked={s.check_on_start} onchange={(v) => set("check_on_start", v)} />
    <SwitchRow label="Install updates in the background"
      desc="Updates AppImages, portable apps, Flatpaks and Windows installers on their own. System packages that need your password are only announced."
      checked={s.background_updates} onchange={(v) => set("background_updates", v)} />
    <SwitchRow label="Notifications" desc="Tell me when updates are found or installed." checked={s.notifications}
      onchange={(v) => set("notifications", v)} />
    <SwitchRow label="Only check installed and track-only apps" checked={s.only_check_installed}
      onchange={(v) => set("only_check_installed", v)} />
    <SwitchRow label="Include prereleases for new apps" checked={s.include_prereleases_by_default}
      onchange={(v) => set("include_prereleases_by_default", v)} />
    <SwitchRow label="Confirm before “Update all”" checked={s.confirm_bulk_update} onchange={(v) => set("confirm_bulk_update", v)} />
    <div class="setting">
      <div class="text">
        <div class="label">Parallel checks</div>
        <div class="desc">How many apps to check at once.</div>
      </div>
      <md-slider min="1" max="16" step="1" labeled value={s.parallel_checks}
        onchange={(e: Event) => set("parallel_checks", Number((e.target as HTMLInputElement).value))}></md-slider>
    </div>
  </div>

  <div class="section-title"><md-icon>folder</md-icon>Installing</div>
  <div class="card">
    <div class="dir-row">
      <md-outlined-text-field label="Programs folder (single binaries)" value={s.bin_dir}
        onchange={(e: Event) => set("bin_dir", val(e))}>
        <md-icon slot="leading-icon">terminal</md-icon>
      </md-outlined-text-field>
      <md-outlined-button onclick={() => browse("bin_dir")}>Browse</md-outlined-button>
    </div>
    {#if sys && !sys.bin_dir_in_path}
      <div class="banner tertiary small">
        <md-icon>warning</md-icon>
        <div class="msg">This folder isn't on your PATH, so terminal tools installed there won't be found by name.{pathHelp ? "\n\n" + pathHelp : ""}</div>
        <md-text-button onclick={addPath}>{isWin ? "Add to PATH" : "How to fix"}</md-text-button>
      </div>
    {/if}
    <div class="dir-row">
      <md-outlined-text-field label={isWin ? "Apps folder (portable apps)" : "Apps folder (AppImages & unpacked apps)"} value={s.apps_dir}
        onchange={(e: Event) => set("apps_dir", val(e))}>
        <md-icon slot="leading-icon">apps</md-icon>
      </md-outlined-text-field>
      <md-outlined-button onclick={() => browse("apps_dir")}>Browse</md-outlined-button>
    </div>
    <SwitchRow label="Keep downloaded files" checked={s.keep_downloads} onchange={(v) => set("keep_downloads", v)} />

    <div class="setting">
      <div class="text">
        <div class="label">Preferred install types</div>
        <div class="desc">When a release offers several formats, the one higher in this list wins.</div>
      </div>
    </div>
    <div class="prio">
      {#each priority as k, i (k)}
        <div class="prio-item">
          <span class="n">{i + 1}</span>
          <span class="kl">{KIND_LABEL[k]}</span>
          <md-icon-button aria-label="Move up" disabled={i === 0} onclick={() => moveKind(s.kind_priority.indexOf(k), -1)}>
            <md-icon>arrow_upward</md-icon>
          </md-icon-button>
          <md-icon-button aria-label="Move down" disabled={i === priority.length - 1} onclick={() => moveKind(s.kind_priority.indexOf(k), 1)}>
            <md-icon>arrow_downward</md-icon>
          </md-icon-button>
        </div>
      {/each}
    </div>
  </div>

  <div class="section-title"><md-icon>key</md-icon>Access tokens</div>
  <div class="card">
    <p class="muted small-p">
      Without a token GitHub allows 60 checks an hour. A token with no extra permissions raises that to 5,000.
      <button class="link" onclick={() => openUrl("https://github.com/settings/tokens/new?description=Obtainit")}>Create a GitHub token</button>
    </p>
    <md-outlined-text-field label="GitHub token" type={showTokens ? "text" : "password"} value={s.github_token}
      onchange={(e: Event) => set("github_token", val(e).trim())}>
      <md-icon-button slot="trailing-icon" aria-label="Show" onclick={() => (showTokens = !showTokens)}>
        <md-icon>{showTokens ? "visibility_off" : "visibility"}</md-icon>
      </md-icon-button>
    </md-outlined-text-field>
    <div style="height:12px"></div>
    <div class="grid2">
      <md-outlined-text-field label="GitLab token" type={showTokens ? "text" : "password"} value={s.gitlab_token}
        onchange={(e: Event) => set("gitlab_token", val(e).trim())}></md-outlined-text-field>
      <md-outlined-text-field label="Codeberg / Forgejo token" type={showTokens ? "text" : "password"} value={s.forgejo_token}
        onchange={(e: Event) => set("forgejo_token", val(e).trim())}></md-outlined-text-field>
    </div>
  </div>

  <div class="section-title"><md-icon>desktop_windows</md-icon>System</div>
  <div class="card">
    <SwitchRow label="Keep running in the tray when closed" desc="Needed for background checks."
      checked={s.close_to_tray} onchange={(v) => set("close_to_tray", v)} />
    <SwitchRow label="Start with {isWin ? 'Windows' : 'your session'}" checked={s.autostart} onchange={(v) => set("autostart", v)} />
    <SwitchRow label="Start hidden in the tray" desc="When started automatically." checked={s.start_minimized}
      disabled={!s.autostart} onchange={(v) => set("start_minimized", v)} />
  </div>

  <div class="section-title"><md-icon>system_update</md-icon>Obtainit updates</div>
  <div class="card">
    <div class="setting">
      <div class="text">
        <div class="label">
          {#if su?.available}
            Version {su.latest} is available
          {:else}
            Obtainit {store.version}
          {/if}
        </div>
        <div class="desc">
          {#if su?.error}
            Couldn't check: {su.error}
          {:else if su?.available && !su.can_install}
            {su.method === "dev" ? "This is a development build, so it can't replace itself." : "This copy wasn't installed with an installer, so download the new version yourself."}
          {:else if su?.checked_at}
            Installed as {METHOD[su.method]} · checked {ago(su.checked_at)}
          {:else}
            Updates come from github.com/Shantil-K/Obtainit as ready-built files.
          {/if}
        </div>
      </div>
      {#if store.selfUpdating}
        <md-circular-progress indeterminate style="--md-circular-progress-size:32px"></md-circular-progress>
      {:else if su?.available && su.can_install}
        <md-filled-button onclick={() => store.updateSelf()}><md-icon slot="icon">upgrade</md-icon>Update & restart</md-filled-button>
      {:else if su?.available}
        <md-filled-tonal-button onclick={() => openUrl(su!.release_url)}><md-icon slot="icon">open_in_new</md-icon>Download</md-filled-tonal-button>
      {:else}
        <md-outlined-button onclick={() => store.checkSelf()}>Check now</md-outlined-button>
      {/if}
    </div>
    {#if selfProg}
      <md-linear-progress indeterminate={!selfProg.total} value={selfProg.total ? selfProg.received / selfProg.total : 0}></md-linear-progress>
    {/if}
    <SwitchRow label="Check for Obtainit updates" desc="Uses the same schedule as your apps, and also checks at startup."
      checked={s.self_update_check} onchange={(v) => set("self_update_check", v)} />
    <SwitchRow label="Install Obtainit updates automatically"
      desc="Downloads, checks the SHA-256, installs and restarts. DEB/RPM installs ask for your password, so they're only announced."
      checked={s.self_update_auto} disabled={!s.self_update_check} onchange={(v) => set("self_update_auto", v)} />
  </div>

  <div class="section-title"><md-icon>info</md-icon>About</div>
  <div class="card about">
    <div class="row">
      <img src="/icon.png" alt="" width="48" height="48" />
      <div>
        <div class="md-typescale-title-medium">Obtainit {store.version}</div>
        <div class="muted small-p">{sys?.distro} · {sys?.arch}</div>
      </div>
    </div>
    <p class="small-p">
      An Obtainium-style updater for desktop apps. The download, scraping and extraction engine comes from
      <button class="link" onclick={() => openUrl("https://github.com/cjrh/lifter")}>lifter</button>; the ideas and options
      from <button class="link" onclick={() => openUrl("https://github.com/ImranR98/Obtainium")}>Obtainium</button>; the
      Material 3 look from <button class="link" onclick={() => openUrl("https://github.com/plebone/autonomix")}>Autonomix</button>.
      Licensed under the AGPL-3.0.
    </p>
    <div class="muted small-p">
      Can install here:
      {sys?.supported_kinds.map((k) => KIND_LABEL[k]).join(", ")}
      {#if sys && !isWin && !sys.has_pkexec}· pkexec is missing, so system packages can't be installed{/if}
    </div>
  </div>
</div>

<style>
  .narrow {
    max-width: 760px;
  }
  .card {
    padding: 8px 20px 12px;
  }
  .card > md-outlined-select,
  .card > md-outlined-text-field {
    margin-top: 12px;
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 0 4px 12px;
  }
  .swatch {
    all: unset;
    position: relative;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: var(--c);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    outline-offset: 3px;
  }
  .swatch.on {
    outline: 3px solid var(--md-sys-color-primary);
  }
  .swatch:focus-visible {
    outline: 3px solid var(--md-sys-color-outline);
  }
  .swatch.auto {
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-primary);
    border: 2px dashed var(--md-sys-color-outline);
    box-sizing: border-box;
  }
  .gallery {
    display: flex;
    gap: 14px;
    overflow-x: auto;
    padding: 4px 4px 12px;
  }
  .card md-divider {
    margin: 8px 0;
  }
  .dir-row {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-top: 12px;
  }
  .dir-row md-outlined-text-field {
    flex: 1;
  }
  .banner.small {
    margin-top: 10px;
    font-size: 13px;
  }
  .prio {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }
  .prio-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 2px 4px 2px 14px;
    border-radius: var(--shape-m);
    background: var(--md-sys-color-surface-container);
  }
  .prio-item .n {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
  .prio-item .kl {
    flex: 1;
    font-size: 14px;
  }
  md-slider {
    width: 220px;
  }
  .small-p {
    font-size: 13.5px;
    line-height: 20px;
  }
  .about {
    padding: 16px 20px;
  }
  .about img {
    border-radius: 12px;
  }
  .link {
    all: unset;
    cursor: pointer;
    color: var(--md-sys-color-primary);
  }
  .link:hover {
    text-decoration: underline;
  }
</style>
