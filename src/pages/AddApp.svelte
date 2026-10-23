<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "../lib/state.svelte";
  import { api } from "../lib/api";
  import type { AppSettings, Preview, SourceKind } from "../lib/types";
  import { SOURCE_LABEL, date } from "../lib/format";
  import TopBar from "../components/TopBar.svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import SettingsForm from "../components/SettingsForm.svelte";
  import AssetList from "../components/AssetList.svelte";
  import SwitchRow from "../components/SwitchRow.svelte";

  let { initialUrl = "" }: { initialUrl?: string } = $props();

  function defaults(): AppSettings {
    return {
      include_prereleases: store.settings?.include_prereleases_by_default ?? false,
      fallback_to_older_releases: true,
      verify_latest_tag: false,
      release_title_filter: "",
      invert_title_filter: false,
      release_notes_filter: "",
      invert_notes_filter: false,
      sort_method: "date",
      use_latest_asset_date: false,
      min_update_age_days: 0,
      version_source: "tag",
      version_extraction_regex: "",
      match_group: "",
      version_compare: "newer",
      asset_filter: "",
      invert_asset_filter: false,
      auto_filter_platform: true,
      install_kind: "auto",
      extract_targets: [],
      desired_filename: "",
      archive_mode: "auto",
      exe_mode: "auto",
      silent_install: true,
      installer_args: "",
      create_shortcut: true,
      launch_command: "",
      post_update_command: "",
      verify_checksum: true,
      track_only: false,
      exempt_from_background: false,
      skip_notifications: false,
      ignored_version: "",
      allow_insecure: false,
      html: { link_selector: "a", link_text_regex: "", version_selector: "" },
      json: { version_path: "$.tag_name", commit_path: "", urls_path: "$.assets.*.browser_download_url" },
    };
  }

  // svelte-ignore state_referenced_locally
  let url = $state(initialUrl);
  let source = $state<SourceKind | "auto">("auto");
  let settings = $state<AppSettings>(defaults());
  let preview = $state<Preview | null>(null);
  let loading = $state(false);
  let saving = $state(false);
  let name = $state("");
  let categories = $state("");
  let alreadyInstalled = $state(false);
  let installedVersion = $state("");
  let chosenAsset = $state<string | null>(null);

  const effectiveSource = $derived<SourceKind>(preview?.source ?? (source === "auto" ? "github" : source));

  async function lookup() {
    if (!url.trim()) return;
    loading = true;
    try {
      preview = await api.preview(url, source === "auto" ? null : source, $state.snapshot(settings) as AppSettings);
      if (!name.trim()) name = preview.meta.name;
      chosenAsset = preview.latest?.selected ?? null;
      if (!installedVersion && preview.latest) installedVersion = preview.latest.version;
    } catch (e) {
      preview = null;
      store.toast(String(e), true);
    } finally {
      loading = false;
    }
  }

  async function add(install: boolean) {
    if (!preview) return;
    saving = true;
    const app = await store.run(() =>
      api.add({
        url: preview!.url,
        source: preview!.source,
        name: name.trim() || preview!.meta.name,
        author: preview!.meta.author,
        description: preview!.meta.description,
        icon_url: preview!.meta.icon_url,
        categories: categories.split(",").map((c) => c.trim()).filter(Boolean),
        settings: $state.snapshot(settings) as AppSettings,
        installed_version: alreadyInstalled ? installedVersion : null,
      }),
    );
    saving = false;
    if (!app) return;
    store.upsert(app);
    store.route = { page: "app", id: app.id };
    if (install) {
      const asset = chosenAsset && chosenAsset !== app.latest?.selected ? chosenAsset : undefined;
      store.run(() => api.install(app.id, asset), `${app.name} installed`);
    } else {
      store.toast(`Added ${app.name}`);
    }
  }

  const examples = ["BurntSushi/ripgrep", "obsidianmd/obsidian-releases", "neovim/neovim", "localsend/localsend", "codeberg.org/forgejo/forgejo"];

  onMount(() => {
    if (initialUrl) lookup();
  });
</script>

<TopBar title="Add app" back />

<div class="page">
  <div class="card lookup">
    <div class="url-row">
      <md-outlined-text-field
        label="Repository or page URL"
        placeholder="https://github.com/owner/repo  or  owner/repo"
        value={url}
        oninput={(e: Event) => (url = (e.target as HTMLInputElement).value)}
        onkeydown={(e: KeyboardEvent) => e.key === "Enter" && lookup()}
      >
        <md-icon slot="leading-icon">link</md-icon>
      </md-outlined-text-field>
      <md-outlined-select
        class="src"
        label="Source"
        value={source}
        onchange={(e: Event) => {
          source = (e.target as HTMLSelectElement).value as SourceKind | "auto";
          preview = null;
        }}
      >
        <md-select-option value="auto"><div slot="headline">Detect</div></md-select-option>
        {#each Object.entries(SOURCE_LABEL) as [k, label]}
          <md-select-option value={k}><div slot="headline">{label}</div></md-select-option>
        {/each}
      </md-outlined-select>
      <md-filled-button onclick={lookup} disabled={loading || !url.trim()}>
        {#if loading}
          <md-circular-progress slot="icon" indeterminate style="--md-circular-progress-size:18px"></md-circular-progress>
        {:else}
          <md-icon slot="icon">search</md-icon>
        {/if}
        Look up
      </md-filled-button>
    </div>
    {#if !preview}
      <div class="row wrap examples">
        <span class="muted">Try</span>
        <md-chip-set>
          {#each examples as ex}
            <md-suggestion-chip label={ex} onclick={() => { url = ex; lookup(); }}></md-suggestion-chip>
          {/each}
        </md-chip-set>
      </div>
    {/if}
  </div>

  {#if preview}
    <div class="card hero">
      <AppIcon name={preview.meta.name} url={preview.meta.icon_url} size={64} />
      <div class="hero-text">
        <div class="md-typescale-title-large">{preview.meta.name}</div>
        <div class="muted">{preview.meta.author} · {SOURCE_LABEL[preview.source]}</div>
        {#if preview.meta.description}<p class="desc">{preview.meta.description}</p>{/if}
      </div>
      {#if preview.latest}
        <div class="latest">
          <div class="md-typescale-label-medium muted">Latest</div>
          <div class="md-typescale-title-medium selectable">{preview.latest.version}</div>
          <div class="muted small">{date(preview.latest.published_at)}{preview.latest.prerelease ? " · prerelease" : ""}</div>
        </div>
      {/if}
    </div>

    {#if preview.error}
      <div class="banner"><md-icon>error</md-icon><div class="msg">{preview.error}</div></div>
    {/if}

    {#if preview.latest}
      {#if preview.latest.held_back}
        <div class="banner info"><md-icon>schedule</md-icon><div class="msg">Newer release {preview.latest.held_back} is held back by the minimum age setting.</div></div>
      {/if}
      <div class="section-title"><md-icon>file_present</md-icon>Files in {preview.latest.tag}</div>
      <AssetList assets={preview.latest.assets} bind:selected={chosenAsset} />
      {#if !preview.latest.selected}
        <div class="banner tertiary" style="margin-top:12px">
          <md-icon>info</md-icon>
          <div class="msg">No file matched this system automatically. Pick one above, adjust the filters below, or add it as track-only.</div>
        </div>
      {/if}
    {/if}

    <div class="section-title"><md-icon>badge</md-icon>Details</div>
    <div class="card">
      <div class="grid2">
        <md-outlined-text-field label="Name" value={name} oninput={(e: Event) => (name = (e.target as HTMLInputElement).value)}></md-outlined-text-field>
        <md-outlined-text-field label="Categories" placeholder="Dev tools, Media" value={categories}
          supporting-text="Comma separated"
          oninput={(e: Event) => (categories = (e.target as HTMLInputElement).value)}></md-outlined-text-field>
      </div>
      <SwitchRow label="Already installed" desc="Track a copy you installed yourself instead of installing it now."
        bind:checked={alreadyInstalled} />
      {#if alreadyInstalled}
        <md-outlined-text-field label="Installed version" value={installedVersion}
          oninput={(e: Event) => (installedVersion = (e.target as HTMLInputElement).value)}></md-outlined-text-field>
      {/if}
    </div>

    <div class="section-title">
      <md-icon>tune</md-icon>Options
      <span class="spacer"></span>
      <md-text-button onclick={lookup} disabled={loading}>
        <md-icon slot="icon">refresh</md-icon>Apply to preview
      </md-text-button>
    </div>
    <SettingsForm bind:settings source={effectiveSource} />

    <div class="footer">
      <md-outlined-button onclick={() => add(false)} disabled={saving}>
        <md-icon slot="icon">bookmark_add</md-icon>Add
      </md-outlined-button>
      {#if !alreadyInstalled && !settings.track_only && (chosenAsset || preview.latest?.selected)}
        <md-filled-button onclick={() => add(true)} disabled={saving}>
          <md-icon slot="icon">download</md-icon>Add & install
        </md-filled-button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .url-row {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .url-row md-outlined-text-field {
    flex: 1;
  }
  .src {
    width: 200px;
    flex-shrink: 0;
  }
  .examples {
    margin-top: 14px;
    font-size: 14px;
  }
  .hero {
    display: flex;
    gap: 20px;
    align-items: flex-start;
    margin-top: 16px;
  }
  .hero-text {
    flex: 1;
    min-width: 0;
  }
  .desc {
    margin: 8px 0 0;
    font-size: 14px;
    line-height: 20px;
  }
  .latest {
    text-align: right;
  }
  .small {
    font-size: 12px;
  }
  .banner {
    margin-top: 12px;
  }
  .footer {
    position: sticky;
    bottom: 0;
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 0;
    margin-top: 16px;
    background: linear-gradient(transparent, var(--md-sys-color-surface) 30%);
  }
  @media (max-width: 720px) {
    .url-row {
      flex-wrap: wrap;
    }
    .src {
      flex: 1;
    }
    .hero {
      flex-wrap: wrap;
    }
    .latest {
      text-align: left;
    }
  }
</style>
