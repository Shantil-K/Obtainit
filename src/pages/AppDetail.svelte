<script lang="ts">
  import { openPath, openUrl } from "@tauri-apps/plugin-opener";
  import { store, statusOf } from "../lib/state.svelte";
  import { api } from "../lib/api";
  import type { AppSettings, SourceKind } from "../lib/types";
  import { KIND_LABEL, SOURCE_LABEL, STATUS, ago, bytes, date } from "../lib/format";
  import TopBar from "../components/TopBar.svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import AssetList from "../components/AssetList.svelte";
  import Markdown from "../components/Markdown.svelte";
  import SettingsForm from "../components/SettingsForm.svelte";

  let { id }: { id: string } = $props();

  const app = $derived(store.get(id));
  const status = $derived(app ? statusOf(app) : "not_installed");
  const prog = $derived(store.progress[id]);
  const busy = $derived(!!prog);
  const pct = $derived(prog?.total ? prog.received / prog.total : null);

  let tab = $state<"overview" | "files" | "options">("overview");
  let menuOpen = $state(false);
  let removeOpen = $state(false);
  let alsoUninstall = $state(true);
  let markOpen = $state(false);
  let markVersion = $state("");
  let canLaunch = $state(false);
  let selected = $state<string | null>(null);

  // Editable copies for the Options tab.
  let draft = $state<AppSettings | null>(null);
  let draftName = $state("");
  let draftCats = $state("");
  let draftUrl = $state("");
  let draftSource = $state<SourceKind>("github");

  function resetDraft() {
    if (!app) return;
    draft = structuredClone($state.snapshot(app.settings)) as AppSettings;
    draftName = app.name;
    draftCats = app.categories.join(", ");
    draftUrl = app.url;
    draftSource = app.source;
  }

  $effect(() => {
    if (app && !draft) resetDraft();
  });

  $effect(() => {
    // Re-evaluate when the install changes.
    void app?.installed;
    api.canLaunch(id).then((v) => (canLaunch = v));
  });

  $effect(() => {
    if (app?.latest && selected === null) selected = app.latest.selected;
  });

  const dirty = $derived.by(() => {
    if (!app || !draft) return false;
    return (
      JSON.stringify($state.snapshot(draft)) !== JSON.stringify($state.snapshot(app.settings)) ||
      draftName !== app.name ||
      draftCats !== app.categories.join(", ") ||
      draftUrl !== app.url ||
      draftSource !== app.source
    );
  });

  async function saveOptions() {
    if (!app || !draft) return;
    const r = await store.run(
      () =>
        api.update(id, {
          name: draftName,
          categories: draftCats.split(",").map((c) => c.trim()).filter(Boolean),
          settings: $state.snapshot(draft!) as AppSettings,
          url: draftUrl !== app!.url ? draftUrl : undefined,
          source: draftSource !== app!.source ? draftSource : undefined,
        }),
      "Saved",
    );
    if (r) {
      store.upsert(r);
      resetDraft();
      selected = r.latest?.selected ?? null;
    }
  }

  async function install(asset?: string) {
    const verb = app?.installed ? "updated" : "installed";
    const r = await store.run(() => api.install(id, asset), `${app?.name} ${verb}`);
    if (r) store.upsert(r);
  }

  async function check() {
    const r = await store.run(() => api.check([id]));
    if (r?.[0]) {
      store.upsert(r[0]);
      selected = r[0].latest?.selected ?? null;
      store.toast(statusOf(r[0]) === "update" ? `Update available: ${r[0].latest?.version}` : r[0].last_error ? "Check failed" : "Up to date");
    }
  }

  async function launch() {
    await store.run(() => api.launch(id));
  }

  async function openFolder() {
    const f = await api.folder(id);
    if (f) store.run(() => openPath(f));
  }

  async function remove() {
    removeOpen = false;
    const name = app?.name;
    const ok = await store.run(() => api.remove(id, alsoUninstall && !!app?.installed && !app.installed.manual));
    if (ok !== undefined) {
      store.route = { page: "apps" };
      store.toast(`Removed ${name}`);
    }
  }

  async function uninstall() {
    const r = await store.run(() => api.uninstall(id), "Uninstalled");
    if (r) store.upsert(r);
  }

  async function markInstalled() {
    markOpen = false;
    const r = await store.run(() => api.markInstalled(id, markVersion), "Marked as installed");
    if (r) store.upsert(r);
  }

  async function ignore() {
    const v = app?.latest?.version ?? null;
    const r = await store.run(() => api.ignoreVersion(id, v), `Ignoring ${v}`);
    if (r) {
      store.upsert(r);
      resetDraft();
    }
  }

  async function prefer(name: string) {
    const r = await store.run(() => api.preferAsset(id, name), "This file will be used from now on");
    if (r) {
      store.upsert(r);
      resetDraft();
    }
  }

  const stageText: Record<string, string> = {
    checking: "Checking for updates…",
    downloading: "Downloading",
    verifying: "Verifying checksum…",
    installing: "Installing…",
    removing: "Removing…",
  };

  const mainLabel = $derived(status === "update" ? `Update to ${app?.latest?.version}` : app?.installed ? "Reinstall" : "Install");
</script>

{#if !app}
  <TopBar title="Not found" back />
{:else}
  <TopBar title={app.name} back>
    {#snippet actions()}
      <md-icon-button aria-label="Favorite" title="Favorite" onclick={() => store.run(() => api.update(id, { favorite: !app.favorite }))}>
        <md-icon class:filled={app.favorite}>{app.favorite ? "star" : "star_outline"}</md-icon>
      </md-icon-button>
      <md-icon-button aria-label="Check for updates" title="Check for updates" onclick={check} disabled={busy}>
        <md-icon>refresh</md-icon>
      </md-icon-button>
      <span style="position:relative">
        <md-icon-button id="more-anchor" aria-label="More" onclick={() => (menuOpen = !menuOpen)}>
          <md-icon>more_vert</md-icon>
        </md-icon-button>
        <md-menu anchor="more-anchor" open={menuOpen} onclosed={() => (menuOpen = false)} positioning="popover">
          <md-menu-item onclick={() => openUrl(app.url)}>
            <md-icon slot="start">open_in_new</md-icon><div slot="headline">Open source page</div>
          </md-menu-item>
          {#if app.latest?.release_url}
            <md-menu-item onclick={() => openUrl(app.latest!.release_url)}>
              <md-icon slot="start">new_releases</md-icon><div slot="headline">Open release page</div>
            </md-menu-item>
          {/if}
          {#if app.installed && !app.installed.manual}
            <md-menu-item onclick={openFolder}>
              <md-icon slot="start">folder_open</md-icon><div slot="headline">Open install folder</div>
            </md-menu-item>
          {/if}
          <md-divider></md-divider>
          <md-menu-item onclick={() => { markVersion = app.latest?.version ?? app.installed?.version ?? ""; markOpen = true; }}>
            <md-icon slot="start">done_all</md-icon><div slot="headline">Mark as installed…</div>
          </md-menu-item>
          {#if app.installed}
            <md-menu-item onclick={() => store.run(() => api.markNotInstalled(id), "Install record cleared")}>
              <md-icon slot="start">remove_done</md-icon><div slot="headline">Mark as not installed</div>
            </md-menu-item>
          {/if}
          {#if status === "update"}
            <md-menu-item onclick={ignore}>
              <md-icon slot="start">block</md-icon><div slot="headline">Ignore {app.latest?.version}</div>
            </md-menu-item>
          {/if}
          <md-divider></md-divider>
          <md-menu-item onclick={() => { alsoUninstall = true; removeOpen = true; }}>
            <md-icon slot="start">delete</md-icon><div slot="headline">Remove from Obtainit…</div>
          </md-menu-item>
        </md-menu>
      </span>
    {/snippet}
  </TopBar>

  <div class="page">
    <div class="hero">
      <AppIcon name={app.name} url={app.icon_url} size={88} />
      <div class="hero-text">
        <div class="md-typescale-headline-medium name">{app.name}</div>
        <div class="muted">
          {app.author ? app.author + " · " : ""}<button class="link" onclick={() => openUrl(app.url)}>{SOURCE_LABEL[app.source]}</button>
        </div>
        <div class="row wrap chips-row">
          <span class="badge {STATUS[status].tone}"><md-icon>{STATUS[status].icon}</md-icon>{STATUS[status].label}</span>
          {#if app.installed && app.installed.kind !== "unknown"}
            <span class="badge">{KIND_LABEL[app.installed.kind]}</span>
          {/if}
          {#each app.categories as c}<span class="badge">{c}</span>{/each}
        </div>
      </div>
    </div>

    {#if busy}
      <div class="card progress">
        <div class="row">
          <span>{stageText[prog.stage] ?? prog.stage}</span>
          <span class="spacer"></span>
          {#if prog.stage === "downloading" && prog.received}
            <span class="muted mono">{bytes(prog.received)}{prog.total ? ` / ${bytes(prog.total)}` : ""}</span>
          {/if}
        </div>
        <md-linear-progress indeterminate={pct === null} value={pct ?? 0}></md-linear-progress>
      </div>
    {/if}

    <div class="actions">
      {#if !app.settings.track_only && app.latest?.selected && (status !== "installed" || !app.installed?.manual)}
        {#if status === "update" || !app.installed}
          <md-filled-button onclick={() => install(selected ?? undefined)} disabled={busy}>
            <md-icon slot="icon">{status === "update" ? "upgrade" : "download"}</md-icon>{mainLabel}
          </md-filled-button>
        {/if}
      {/if}
      {#if app.settings.track_only && app.latest && status !== "installed"}
        <md-filled-button onclick={() => store.run(() => api.markInstalled(id), "Marked as updated")}>
          <md-icon slot="icon">done_all</md-icon>Mark {app.latest.version} as installed
        </md-filled-button>
      {/if}
      {#if canLaunch}
        <md-filled-tonal-button onclick={launch} disabled={busy}>
          <md-icon slot="icon">play_arrow</md-icon>Open
        </md-filled-tonal-button>
      {/if}
      {#if app.installed && !app.installed.manual}
        <md-outlined-button onclick={uninstall} disabled={busy}>
          <md-icon slot="icon">delete_sweep</md-icon>Uninstall
        </md-outlined-button>
      {/if}
    </div>

    {#if app.last_error}
      <div class="banner"><md-icon>error</md-icon><div class="msg selectable">{app.last_error}</div>
        <md-text-button onclick={check}>Retry</md-text-button></div>
    {/if}
    {#if app.latest && !app.latest.selected && !app.settings.track_only}
      <div class="banner tertiary">
        <md-icon>info</md-icon>
        <div class="msg">No file in {app.latest.tag} matched this system. Pick one under Files, or adjust the filters under Options.</div>
      </div>
    {/if}
    {#if app.latest?.held_back}
      <div class="banner info"><md-icon>schedule</md-icon><div class="msg">Release {app.latest.held_back} is newer but held back by the minimum age setting.</div></div>
    {/if}

    <div class="stats">
      <div class="stat card">
        <div class="k muted">Installed</div>
        <div class="v selectable">{app.installed?.version || "—"}</div>
        <div class="s muted">{app.installed ? (app.installed.manual ? "marked by hand" : ago(app.installed.installed_at)) : ""}</div>
      </div>
      <div class="stat card">
        <div class="k muted">Latest</div>
        <div class="v selectable">{app.latest?.version || "—"}</div>
        <div class="s muted">{app.latest ? date(app.latest.published_at) + (app.latest.prerelease ? " · prerelease" : "") : ""}</div>
      </div>
      <div class="stat card">
        <div class="k muted">Last checked</div>
        <div class="v">{app.last_checked ? ago(app.last_checked) : "never"}</div>
        <div class="s muted">{app.latest?.assets.filter((a) => a.compatible).length ?? 0} matching file(s)</div>
      </div>
    </div>

    <md-tabs class="tabs" activeTabIndex={["overview", "files", "options"].indexOf(tab)}
      onchange={(e: Event) => (tab = (["overview", "files", "options"] as const)[(e.target as unknown as { activeTabIndex: number }).activeTabIndex])}>
      <md-primary-tab><md-icon slot="icon">article</md-icon>Release notes</md-primary-tab>
      <md-primary-tab><md-icon slot="icon">file_present</md-icon>Files</md-primary-tab>
      <md-primary-tab><md-icon slot="icon">tune</md-icon>Options{dirty ? " •" : ""}</md-primary-tab>
    </md-tabs>

    <div class="tab-body">
      {#if tab === "overview"}
        <div class="card">
          {#if app.latest}
            <div class="md-typescale-title-medium notes-title selectable">{app.latest.title || app.latest.tag}</div>
            {#if app.latest.notes}
              <Markdown source={app.latest.notes} />
            {:else}
              <p class="muted">No release notes.</p>
            {/if}
          {:else}
            <p class="muted">No release information yet. Use refresh to check.</p>
          {/if}
        </div>
        {#if app.installed && !app.installed.manual}
          <div class="section-title"><md-icon>inventory_2</md-icon>Installation</div>
          <div class="card">
            <dl class="kv selectable">
              <dt>Type</dt><dd>{KIND_LABEL[app.installed.kind]}</dd>
              {#if app.installed.asset_name}<dt>File</dt><dd class="mono">{app.installed.asset_name}</dd>{/if}
              {#if app.installed.package_name}<dt>Package</dt><dd class="mono">{app.installed.package_name}</dd>{/if}
              {#if app.installed.executable}<dt>Program</dt><dd class="mono">{app.installed.executable}</dd>{/if}
              {#each app.installed.paths.filter((p) => p !== app.installed?.executable) as p}<dt>Files</dt><dd class="mono">{p}</dd>{/each}
              {#if app.installed.desktop_entry}<dt>Menu entry</dt><dd class="mono">{app.installed.desktop_entry}</dd>{/if}
              {#if app.installed.sha256}<dt>SHA-256</dt><dd class="mono">{app.installed.sha256}</dd>{/if}
            </dl>
          </div>
        {/if}
      {:else if tab === "files"}
        {#if app.latest}
          <AssetList assets={app.latest.assets} bind:selected installedName={app.installed?.asset_name ?? null}>
            {#snippet trailing(a)}
              {#if selected === a.name}
                <md-icon-button title="Always use this file (saves a filter)" aria-label="Always use this file"
                  onclick={(e: Event) => { e.stopPropagation(); prefer(a.name); }}>
                  <md-icon>push_pin</md-icon>
                </md-icon-button>
                <md-filled-tonal-button onclick={(e: Event) => { e.stopPropagation(); install(a.name); }} disabled={busy}>
                  Install
                </md-filled-tonal-button>
              {/if}
            {/snippet}
          </AssetList>
        {:else}
          <p class="muted">No release information yet.</p>
        {/if}
      {:else if tab === "options" && draft}
        <div class="card">
          <div class="grid2">
            <md-outlined-text-field label="Name" value={draftName}
              oninput={(e: Event) => (draftName = (e.target as HTMLInputElement).value)}></md-outlined-text-field>
            <md-outlined-text-field label="Categories" value={draftCats} supporting-text="Comma separated"
              oninput={(e: Event) => (draftCats = (e.target as HTMLInputElement).value)}></md-outlined-text-field>
            <md-outlined-text-field label="Source URL" value={draftUrl}
              oninput={(e: Event) => (draftUrl = (e.target as HTMLInputElement).value)}></md-outlined-text-field>
            <md-outlined-select label="Source type" value={draftSource}
              onchange={(e: Event) => (draftSource = (e.target as HTMLSelectElement).value as SourceKind)}>
              {#each Object.entries(SOURCE_LABEL) as [k, label]}
                <md-select-option value={k}><div slot="headline">{label}</div></md-select-option>
              {/each}
            </md-outlined-select>
          </div>
        </div>
        <div style="height:12px"></div>
        <SettingsForm bind:settings={draft} source={draftSource} />
        {#if dirty}
          <div class="save-bar">
            <span class="muted">Unsaved changes</span>
            <span class="spacer"></span>
            <md-text-button onclick={resetDraft}>Discard</md-text-button>
            <md-filled-button onclick={saveOptions}><md-icon slot="icon">save</md-icon>Save & re-check</md-filled-button>
          </div>
        {/if}
      {/if}
    </div>
  </div>

  <md-dialog open={removeOpen} onclosed={() => (removeOpen = false)}>
    <md-icon slot="icon">delete</md-icon>
    <div slot="headline">Remove {app.name}?</div>
    <div slot="content">
      <p>Obtainit will stop tracking this app.</p>
      {#if app.installed && !app.installed.manual}
        <!-- svelte-ignore a11y_label_has_associated_control -->
        <label class="row">
          <md-checkbox checked={alsoUninstall} onchange={(e: Event) => (alsoUninstall = (e.target as HTMLInputElement).checked)}></md-checkbox>
          Also uninstall it from this computer
        </label>
      {/if}
    </div>
    <div slot="actions">
      <md-text-button onclick={() => (removeOpen = false)}>Cancel</md-text-button>
      <md-filled-button onclick={remove}>Remove</md-filled-button>
    </div>
  </md-dialog>

  <md-dialog open={markOpen} onclosed={() => (markOpen = false)}>
    <md-icon slot="icon">done_all</md-icon>
    <div slot="headline">Mark as installed</div>
    <div slot="content">
      <p class="muted">Use this when you installed {app.name} yourself. Obtainit will offer updates newer than this version.</p>
      <md-outlined-text-field label="Installed version" value={markVersion}
        oninput={(e: Event) => (markVersion = (e.target as HTMLInputElement).value)}></md-outlined-text-field>
    </div>
    <div slot="actions">
      <md-text-button onclick={() => (markOpen = false)}>Cancel</md-text-button>
      <md-filled-button onclick={markInstalled}>Save</md-filled-button>
    </div>
  </md-dialog>
{/if}

<style>
  .hero {
    display: flex;
    gap: 24px;
    align-items: center;
    margin: 4px 0 20px;
  }
  .hero-text {
    min-width: 0;
  }
  .name {
    overflow-wrap: anywhere;
  }
  .chips-row {
    margin-top: 10px;
    gap: 6px;
  }
  .link {
    all: unset;
    cursor: pointer;
    color: var(--md-sys-color-primary);
  }
  .link:hover {
    text-decoration: underline;
  }
  .progress {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 16px;
  }
  md-linear-progress {
    width: 100%;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 16px;
  }
  .banner {
    margin-bottom: 12px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    margin: 8px 0 20px;
  }
  .stats .card {
    margin: 0;
    padding: 16px 18px;
  }
  .k {
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }
  .v {
    font-size: 20px;
    line-height: 30px;
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
  }
  .s {
    font-size: 12.5px;
  }
  .tabs {
    background: transparent;
    --md-primary-tab-container-color: transparent;
    margin-bottom: 16px;
  }
  .notes-title {
    margin-bottom: 12px;
  }
  .kv {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 20px;
    margin: 0;
    font-size: 14px;
  }
  .kv dt {
    color: var(--md-sys-color-on-surface-variant);
  }
  .kv dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .save-bar {
    position: sticky;
    bottom: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
    padding: 10px 12px 10px 20px;
    border-radius: var(--shape-xl);
    background: var(--md-sys-color-surface-container-highest);
    box-shadow: 0 2px 10px rgb(0 0 0 / 0.18);
  }
  label.row {
    gap: 8px;
    cursor: pointer;
  }
  @media (max-width: 640px) {
    .stats {
      grid-template-columns: 1fr;
    }
    .hero {
      gap: 16px;
    }
  }
</style>
