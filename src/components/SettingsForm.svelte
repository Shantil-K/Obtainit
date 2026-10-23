<script lang="ts">
  import type { AppSettings, SourceKind } from "../lib/types";
  import { store } from "../lib/state.svelte";
  import { KIND_LABEL } from "../lib/format";
  import SwitchRow from "./SwitchRow.svelte";

  let { settings = $bindable(), source }: { settings: AppSettings; source: SourceKind } = $props();

  const isWin = $derived(store.system?.os === "windows");
  const kinds = $derived(store.system?.supported_kinds ?? []);

  let open = $state<Record<string, boolean>>({ source: true });

  function val(e: Event) {
    return (e.target as HTMLInputElement).value;
  }

  const sections = $derived([
    ...(source === "html" || source === "json" ? [{ key: "source", icon: "travel_explore", title: source === "html" ? "Page scraping (lifter HTML method)" : "JSON query (lifter api_json method)" }] : []),
    { key: "releases", icon: "new_releases", title: "Releases" },
    { key: "version", icon: "tag", title: "Version detection" },
    { key: "files", icon: "file_present", title: "Files & installing" },
    { key: "behaviour", icon: "tune", title: "Behaviour" },
  ]);
</script>

<div class="form">
  {#each sections as sec}
    <div class="group" class:open={open[sec.key]}>
      <button class="head" onclick={() => (open[sec.key] = !open[sec.key])} aria-expanded={!!open[sec.key]}>
        <md-ripple></md-ripple>
        <md-icon>{sec.icon}</md-icon>
        <span class="title">{sec.title}</span>
        <md-icon class="chev">expand_more</md-icon>
      </button>
      {#if open[sec.key]}
        <div class="body">
          {#if sec.key === "source" && source === "html"}
            <p class="muted hint">
              Obtainit fetches the page, looks at every link matched by the selector, and keeps those whose text (or
              file name) fully matches the regex. A capture group like <code>(\d+\.\d+\.\d+)</code> becomes the version.
            </p>
            <md-outlined-text-field label="Link selector (CSS)" value={settings.html.link_selector} placeholder="a"
              oninput={(e: Event) => (settings.html.link_selector = val(e))}></md-outlined-text-field>
            <md-outlined-text-field label="Link text regex" value={settings.html.link_text_regex}
              placeholder={"tool-(\\d+\\.\\d+\\.\\d+)-linux\\.tar\\.gz"}
              oninput={(e: Event) => (settings.html.link_text_regex = val(e))}></md-outlined-text-field>
            <md-outlined-text-field label="Version selector (CSS, optional)" value={settings.html.version_selector}
              supporting-text="Element whose text is the version. Leave empty to use the regex group."
              oninput={(e: Event) => (settings.html.version_selector = val(e))}></md-outlined-text-field>
          {:else if sec.key === "source" && source === "json"}
            <p class="muted hint">JSONPath expressions, e.g. <code>$.tag_name</code> and <code>$.assets.*.browser_download_url</code>.</p>
            <md-outlined-text-field label="Version path" value={settings.json.version_path} placeholder="$.tag_name"
              oninput={(e: Event) => (settings.json.version_path = val(e))}></md-outlined-text-field>
            <md-outlined-text-field label="Download URLs path" value={settings.json.urls_path}
              placeholder="$.assets.*.browser_download_url"
              oninput={(e: Event) => (settings.json.urls_path = val(e))}></md-outlined-text-field>
            <md-outlined-text-field label="Commit path (optional)" value={settings.json.commit_path}
              supporting-text="For tags like 'nightly' that never change: a new commit counts as an update."
              oninput={(e: Event) => (settings.json.commit_path = val(e))}></md-outlined-text-field>
          {:else if sec.key === "releases"}
            <SwitchRow label="Include prereleases" bind:checked={settings.include_prereleases} />
            <SwitchRow label="Fall back to older releases"
              desc="If the newest release has no file for this system, use the newest one that does."
              bind:checked={settings.fallback_to_older_releases} />
            {#if source === "github"}
              <SwitchRow label="Verify the 'latest' tag" desc="Use the release GitHub marks as latest."
                bind:checked={settings.verify_latest_tag} />
            {/if}
            <div class="pair">
              <md-outlined-text-field label="Filter release titles by regex" value={settings.release_title_filter}
                oninput={(e: Event) => (settings.release_title_filter = val(e))}></md-outlined-text-field>
              <md-checkbox touch-target="wrapper" checked={settings.invert_title_filter} aria-label="Invert"
                onchange={(e: Event) => (settings.invert_title_filter = (e.target as HTMLInputElement).checked)}></md-checkbox>
              <span class="inv">Invert</span>
            </div>
            <div class="pair">
              <md-outlined-text-field label="Filter release notes by regex" value={settings.release_notes_filter}
                oninput={(e: Event) => (settings.release_notes_filter = val(e))}></md-outlined-text-field>
              <md-checkbox touch-target="wrapper" checked={settings.invert_notes_filter} aria-label="Invert"
                onchange={(e: Event) => (settings.invert_notes_filter = (e.target as HTMLInputElement).checked)}></md-checkbox>
              <span class="inv">Invert</span>
            </div>
            <div class="grid2">
              <md-outlined-select label="Sort releases by" value={settings.sort_method}
                onchange={(e: Event) => (settings.sort_method = val(e))}>
                <md-select-option value="date"><div slot="headline">Release date</div></md-select-option>
                <md-select-option value="version"><div slot="headline">Version number</div></md-select-option>
                <md-select-option value="none"><div slot="headline">As listed by the source</div></md-select-option>
              </md-outlined-select>
              <md-outlined-text-field label="Minimum release age (days)" type="number" min="0"
                value={String(settings.min_update_age_days)}
                supporting-text="Wait this long before offering a new release."
                oninput={(e: Event) => (settings.min_update_age_days = Math.max(0, Number(val(e)) || 0))}></md-outlined-text-field>
            </div>
            <SwitchRow label="Use latest file upload as release date" bind:checked={settings.use_latest_asset_date} />
          {:else if sec.key === "version"}
            <div class="grid2">
              <md-outlined-select label="Version comes from" value={settings.version_source}
                onchange={(e: Event) => (settings.version_source = val(e))}>
                <md-select-option value="tag"><div slot="headline">Release tag</div></md-select-option>
                <md-select-option value="title"><div slot="headline">Release title</div></md-select-option>
                <md-select-option value="date"><div slot="headline">Release date</div></md-select-option>
              </md-outlined-select>
              <md-outlined-select label="An update is" value={settings.version_compare}
                onchange={(e: Event) => (settings.version_compare = val(e))}>
                <md-select-option value="newer"><div slot="headline">A higher version number</div></md-select-option>
                <md-select-option value="different"><div slot="headline">Any different version</div></md-select-option>
              </md-outlined-select>
            </div>
            <div class="grid2">
              <md-outlined-text-field label="Version extraction regex" value={settings.version_extraction_regex}
                placeholder={"(\\d+\\.\\d+\\.\\d+)"}
                oninput={(e: Event) => (settings.version_extraction_regex = val(e))}></md-outlined-text-field>
              <md-outlined-text-field label="Match group" value={settings.match_group} placeholder="1, name, or $1.$2"
                oninput={(e: Event) => (settings.match_group = val(e))}></md-outlined-text-field>
            </div>
          {:else if sec.key === "files"}
            <div class="pair">
              <md-outlined-text-field label="File name filter (regex)" value={settings.asset_filter}
                supporting-text="Only files matching this are considered."
                oninput={(e: Event) => (settings.asset_filter = val(e))}></md-outlined-text-field>
              <md-checkbox touch-target="wrapper" checked={settings.invert_asset_filter} aria-label="Invert"
                onchange={(e: Event) => (settings.invert_asset_filter = (e.target as HTMLInputElement).checked)}></md-checkbox>
              <span class="inv">Invert</span>
            </div>
            <SwitchRow label="Pick files for this OS and CPU automatically"
              desc="lifter's scoring: prefers {store.system?.os}/{store.system?.arch} builds and skips other platforms."
              bind:checked={settings.auto_filter_platform} />
            <div class="grid2">
              <md-outlined-select label="Install as" value={settings.install_kind}
                onchange={(e: Event) => (settings.install_kind = val(e))}>
                <md-select-option value="auto"><div slot="headline">Automatic</div></md-select-option>
                {#each kinds as k}
                  <md-select-option value={k}><div slot="headline">{KIND_LABEL[k]}</div></md-select-option>
                {/each}
              </md-outlined-select>
              <md-outlined-select label="Archives" value={settings.archive_mode}
                onchange={(e: Event) => (settings.archive_mode = val(e))}>
                <md-select-option value="auto"><div slot="headline">Automatic</div></md-select-option>
                <md-select-option value="binaries"><div slot="headline">Extract programs to bin folder</div></md-select-option>
                <md-select-option value="folder"><div slot="headline">Unpack whole app into a folder</div></md-select-option>
              </md-outlined-select>
            </div>
            <div class="grid2">
              <md-outlined-text-field label="Files to extract from archives"
                value={settings.extract_targets.join(", ")} placeholder="rg, rg.1"
                supporting-text="Comma-separated names or regexes (lifter's target_filename_to_extract)."
                oninput={(e: Event) =>
                  (settings.extract_targets = val(e).split(",").map((s) => s.trim()).filter(Boolean))}></md-outlined-text-field>
              <md-outlined-text-field label="Save program as" value={settings.desired_filename}
                placeholder="(keep original name)" supporting-text="Rename a single extracted/downloaded file."
                oninput={(e: Event) => (settings.desired_filename = val(e))}></md-outlined-text-field>
            </div>
            {#if isWin}
              <div class="grid2">
                <md-outlined-select label=".exe files are" value={settings.exe_mode}
                  onchange={(e: Event) => (settings.exe_mode = val(e))}>
                  <md-select-option value="auto"><div slot="headline">Detect automatically</div></md-select-option>
                  <md-select-option value="installer"><div slot="headline">Setup programs</div></md-select-option>
                  <md-select-option value="portable"><div slot="headline">Portable programs</div></md-select-option>
                </md-outlined-select>
                <md-outlined-text-field label="Installer arguments" value={settings.installer_args}
                  placeholder="/S" supporting-text="Overrides the detected silent-install switches."
                  oninput={(e: Event) => (settings.installer_args = val(e))}></md-outlined-text-field>
              </div>
              <SwitchRow label="Install silently" desc="Detects NSIS, Inno Setup, Squirrel and WiX installers."
                bind:checked={settings.silent_install} />
            {/if}
            <SwitchRow label={isWin ? "Add to Start menu" : "Add to app menu"}
              desc="For AppImages and unpacked apps." bind:checked={settings.create_shortcut} />
            <SwitchRow label="Verify checksums" desc="Check SHA-256 digests when the source publishes them."
              bind:checked={settings.verify_checksum} />
          {:else if sec.key === "behaviour"}
            <SwitchRow label="Track only" desc="Only notify about new versions; mark them updated yourself."
              bind:checked={settings.track_only} />
            <SwitchRow label="Exempt from background updates" bind:checked={settings.exempt_from_background} />
            <SwitchRow label="Skip update notifications" bind:checked={settings.skip_notifications} />
            <SwitchRow label="Allow insecure HTTP" bind:checked={settings.allow_insecure} />
            <md-outlined-text-field label="Launch command (optional)" value={settings.launch_command}
              supporting-text="Used by the Open button instead of the detected program."
              oninput={(e: Event) => (settings.launch_command = val(e))}></md-outlined-text-field>
            <md-outlined-text-field label="Run after install/update (optional)" value={settings.post_update_command}
              supporting-text="Shell command. Gets OBTAINIT_APP, OBTAINIT_VERSION, OBTAINIT_PATHS and OBTAINIT_EXECUTABLE."
              oninput={(e: Event) => (settings.post_update_command = val(e))}></md-outlined-text-field>
            {#if settings.ignored_version}
              <div class="setting">
                <md-icon>block</md-icon>
                <div class="text">
                  <div class="label">Ignoring version {settings.ignored_version}</div>
                </div>
                <md-text-button onclick={() => (settings.ignored_version = "")}>Stop ignoring</md-text-button>
              </div>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .group {
    border-radius: var(--shape-l);
    background: var(--md-sys-color-surface-container-low);
    overflow: hidden;
  }
  .head {
    all: unset;
    position: relative;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 20px;
    cursor: pointer;
  }
  .head:focus-visible {
    outline: 2px solid var(--md-sys-color-primary);
    outline-offset: -2px;
    border-radius: var(--shape-l);
  }
  .head > md-icon {
    color: var(--md-sys-color-on-surface-variant);
  }
  .title {
    flex: 1;
    font-size: 15px;
    font-weight: 500;
  }
  .chev {
    transition: transform 200ms;
  }
  .open .chev {
    transform: rotate(180deg);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 0 20px 20px;
  }
  .pair {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .pair md-outlined-text-field {
    flex: 1;
  }
  .inv {
    font-size: 13px;
    color: var(--md-sys-color-on-surface-variant);
    padding-right: 4px;
  }
  .hint {
    font-size: 13px;
    margin: 0;
  }
  code {
    font-size: 12px;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--md-sys-color-surface-container-highest);
  }
</style>
