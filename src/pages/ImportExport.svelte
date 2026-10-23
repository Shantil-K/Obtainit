<script lang="ts">
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { store } from "../lib/state.svelte";
  import { api } from "../lib/api";
  import type { ImportReport } from "../lib/types";
  import TopBar from "../components/TopBar.svelte";

  let report = $state<ImportReport | null>(null);
  let text = $state("");
  let working = $state(false);

  async function importFile() {
    const path = await openDialog({
      multiple: false,
      filters: [
        { name: "Backups and configs", extensions: ["json", "config", "ini", "txt", "conf"] },
        { name: "All files", extensions: ["*"] },
      ],
    });
    if (typeof path !== "string") return;
    working = true;
    const r = await store.run(() => api.importFile(path));
    working = false;
    if (r) done(r);
  }

  async function importText() {
    if (!text.trim()) return;
    working = true;
    const r = await store.run(() => api.importText(text));
    working = false;
    if (r) {
      done(r);
      text = "";
    }
  }

  function done(r: ImportReport) {
    report = r;
    store.reloadApps();
    store.toast(`Imported ${r.added.length} app${r.added.length === 1 ? "" : "s"} from ${r.format}`);
  }

  async function exportAs(format: "obtainit" | "lifter") {
    const stamp = new Date().toISOString().slice(0, 10);
    const path = await saveDialog({
      defaultPath: format === "lifter" ? "lifter.config" : `obtainit-export-${stamp}.json`,
      filters: format === "lifter" ? [{ name: "lifter config", extensions: ["config"] }] : [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    await store.run(() => api.exportFile(path, format), `Exported ${store.apps.length} apps`);
  }
</script>

<TopBar title="Import & export" />

<div class="page narrow">
  <div class="section-title"><md-icon>download</md-icon>Import</div>
  <div class="card">
    <div class="opt">
      <div class="art"><md-icon>upload_file</md-icon></div>
      <div class="text">
        <div class="label">From a file</div>
        <div class="desc muted">
          Obtainit backups, <b>Obtainium</b> exports (GitHub, GitLab and Codeberg apps), <b>lifter.config</b> files, or a text
          file with one URL per line. The format is detected automatically.
        </div>
      </div>
      <md-filled-tonal-button onclick={importFile} disabled={working}>Choose file</md-filled-tonal-button>
    </div>
  </div>
  <div class="card">
    <md-outlined-text-field type="textarea" rows="5" label="Paste URLs, an Obtainium export or a lifter.config"
      placeholder={"https://github.com/BurntSushi/ripgrep\nsharkdp/bat\nhttps://codeberg.org/owner/repo"}
      value={text} oninput={(e: Event) => (text = (e.target as HTMLTextAreaElement).value)}></md-outlined-text-field>
    <div class="row end">
      <md-filled-button onclick={importText} disabled={working || !text.trim()}>
        <md-icon slot="icon">playlist_add</md-icon>Import
      </md-filled-button>
    </div>
  </div>

  {#if report}
    <div class="card filled">
      <div class="md-typescale-title-small">
        {report.format}: added {report.added.length}, skipped {report.skipped.length}
      </div>
      {#if report.added.length}
        <p class="muted small">{report.added.map((a) => a.name).join(", ")}</p>
      {/if}
      {#if report.skipped.length}
        <ul class="small skipped">
          {#each report.skipped as s}<li>{s}</li>{/each}
        </ul>
      {/if}
    </div>
  {/if}

  <div class="section-title"><md-icon>upload</md-icon>Export</div>
  <div class="card">
    <div class="opt">
      <div class="art"><md-icon>data_object</md-icon></div>
      <div class="text">
        <div class="label">Obtainit backup</div>
        <div class="desc muted">All apps with their options and install records. Access tokens are left out.</div>
      </div>
      <md-outlined-button onclick={() => exportAs("obtainit")} disabled={!store.apps.length}>Export</md-outlined-button>
    </div>
    <md-divider></md-divider>
    <div class="opt">
      <div class="art"><md-icon>terminal</md-icon></div>
      <div class="text">
        <div class="label">lifter.config</div>
        <div class="desc muted">Run the same downloads headless with the <code>lifter</code> command-line tool, e.g. on a server or in cron.</div>
      </div>
      <md-outlined-button onclick={() => exportAs("lifter")} disabled={!store.apps.length}>Export</md-outlined-button>
    </div>
  </div>
</div>

<style>
  .narrow {
    max-width: 760px;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 0;
  }
  .opt .text {
    flex: 1;
  }
  .label {
    font-size: 15px;
    font-weight: 500;
  }
  .desc {
    font-size: 13.5px;
    line-height: 20px;
    margin-top: 2px;
  }
  .art {
    width: 48px;
    height: 48px;
    flex-shrink: 0;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  md-divider {
    margin: 8px 0;
  }
  .end {
    justify-content: flex-end;
    margin-top: 12px;
  }
  .small {
    font-size: 13px;
  }
  .skipped {
    margin: 8px 0 0;
    padding-left: 18px;
    color: var(--md-sys-color-on-surface-variant);
  }
  code {
    font-size: 12px;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--md-sys-color-surface-container-highest);
  }
</style>
