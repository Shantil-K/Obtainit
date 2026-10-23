<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { store } from "../lib/state.svelte";
  import { api } from "../lib/api";
  import type { LogLine } from "../lib/types";
  import TopBar from "../components/TopBar.svelte";

  let lines = $state<LogLine[]>([]);
  let level = $state<"all" | "warn">("all");
  let follow = $state(true);
  let box: HTMLDivElement | undefined = $state();
  let timer: ReturnType<typeof setInterval>;

  async function refresh() {
    lines = await api.logs();
    if (follow && box) requestAnimationFrame(() => box && (box.scrollTop = box.scrollHeight));
  }

  onMount(() => {
    refresh();
    timer = setInterval(refresh, 2000);
  });
  onDestroy(() => clearInterval(timer));

  const shown = $derived(level === "warn" ? lines.filter((l) => l.level === "WARN" || l.level === "ERROR") : lines.filter((l) => l.level !== "DEBUG" && l.level !== "TRACE"));

  async function copy() {
    await navigator.clipboard.writeText(shown.map((l) => `${l.time} ${l.level} ${l.message}`).join("\n"));
    store.toast("Copied");
  }
</script>

<TopBar title="Logs">
  {#snippet actions()}
    <md-icon-button title="Copy" aria-label="Copy" onclick={copy}><md-icon>content_copy</md-icon></md-icon-button>
    <md-icon-button title="Open log file" aria-label="Open log file" onclick={async () => store.run(async () => openPath(await api.logPath()))}>
      <md-icon>description</md-icon>
    </md-icon-button>
    <md-icon-button title="Clear" aria-label="Clear" onclick={async () => { await api.clearLogs(); refresh(); }}>
      <md-icon>delete_sweep</md-icon>
    </md-icon-button>
  {/snippet}
</TopBar>

<div class="page wide">
  <md-chip-set>
    <md-filter-chip label="Everything" selected={level === "all"} onclick={() => (level = "all")}></md-filter-chip>
    <md-filter-chip label="Warnings & errors" selected={level === "warn"} onclick={() => (level = "warn")}></md-filter-chip>
    <md-filter-chip label="Follow" selected={follow} onclick={() => (follow = !follow)}>
      <md-icon slot="icon">vertical_align_bottom</md-icon>
    </md-filter-chip>
  </md-chip-set>
  <div class="log mono selectable" bind:this={box}>
    {#each shown as l}
      <div class="line {l.level.toLowerCase()}"><span class="t">{l.time.slice(11)}</span><span class="lv">{l.level}</span><span class="m">{l.message}</span></div>
    {:else}
      <div class="muted">Nothing logged yet.</div>
    {/each}
  </div>
</div>

<style>
  .wide {
    max-width: none;
  }
  .log {
    margin-top: 12px;
    height: calc(100vh - 190px);
    overflow: auto;
    padding: 12px 16px;
    border-radius: var(--shape-l);
    background: var(--md-sys-color-surface-container-lowest, var(--md-sys-color-surface-container-low));
    border: 1px solid var(--md-sys-color-outline-variant);
    line-height: 1.6;
  }
  .line {
    display: flex;
    gap: 12px;
  }
  .t {
    color: var(--md-sys-color-on-surface-variant);
    flex-shrink: 0;
  }
  .lv {
    width: 44px;
    flex-shrink: 0;
    font-weight: 600;
    color: var(--md-sys-color-secondary);
  }
  .m {
    white-space: pre-wrap;
    word-break: break-word;
  }
  .warn .lv {
    color: var(--md-sys-color-tertiary);
  }
  .error .lv,
  .error .m {
    color: var(--md-sys-color-error);
  }
</style>
