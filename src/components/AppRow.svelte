<script lang="ts">
  import type { App } from "../lib/types";
  import { store, statusOf } from "../lib/state.svelte";
  import { STATUS, ago, bytes } from "../lib/format";
  import { api } from "../lib/api";
  import AppIcon from "./AppIcon.svelte";

  let { app, compact = false }: { app: App; compact?: boolean } = $props();

  const status = $derived(statusOf(app));
  const prog = $derived(store.progress[app.id]);
  const st = $derived(STATUS[status]);
  const pct = $derived(prog?.total ? prog.received / prog.total : null);

  const versionLine = $derived.by(() => {
    const inst = app.installed?.version;
    const lat = app.latest?.version;
    if (status === "update") return `${inst} → ${lat}`;
    if (inst) return inst;
    if (lat) return `Latest ${lat}`;
    return app.last_checked ? "No release found" : "Not checked yet";
  });

  const stageText: Record<string, string> = {
    checking: "Checking…",
    downloading: "Downloading",
    verifying: "Verifying checksum…",
    installing: "Installing…",
    removing: "Removing…",
  };

  function act(e: Event) {
    e.stopPropagation();
    if (status === "update" || status === "not_installed") {
      store.run(() => api.install(app.id), `${app.name} ${status === "update" ? "updated" : "installed"}`);
    } else if (status === "track") {
      store.run(() => api.markInstalled(app.id), "Marked as updated");
    } else if (status === "error") {
      store.run(() => api.check([app.id]));
    }
  }

  const canAct = $derived(
    !prog &&
      (status === "error" ||
        (status === "update" && !app.settings.track_only && !!app.latest?.selected) ||
        (status === "not_installed" && !!app.latest?.selected) ||
        (status === "track" && !!app.latest)),
  );
  const actLabel = $derived(
    status === "update"
      ? "Update"
      : status === "not_installed"
        ? "Install"
        : status === "track"
          ? "Mark updated"
          : "Retry",
  );
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="row-card"
  class:compact
  class:has-update={status === "update"}
  role="button"
  tabindex="0"
  onclick={() => store.go({ page: "app", id: app.id })}
  onkeydown={(e) => e.key === "Enter" && store.go({ page: "app", id: app.id })}
>
  <md-ripple></md-ripple>
  <md-focus-ring></md-focus-ring>
  <AppIcon name={app.name} url={app.icon_url} size={compact ? 36 : 48} />
  <div class="main">
    <div class="name-line">
      <span class="name ellipsis">{app.name}</span>
      {#if app.favorite}<md-icon class="fav filled">star</md-icon>{/if}
      {#if app.author && !compact}<span class="author ellipsis muted">{app.author}</span>{/if}
    </div>
    {#if prog}
      <div class="prog-line">
        <span class="muted">
          {stageText[prog.stage] ?? prog.stage}
          {#if prog.stage === "downloading" && prog.received}
            {bytes(prog.received)}{prog.total ? ` of ${bytes(prog.total)}` : ""}
          {/if}
        </span>
        <md-linear-progress indeterminate={pct === null} value={pct ?? 0}></md-linear-progress>
      </div>
    {:else}
      <div class="sub muted ellipsis">
        <span class="ver">{versionLine}</span>
        {#if app.latest?.published_at && !compact}
          <span class="dot">·</span>{ago(app.latest.published_at)}
        {/if}
        {#each app.categories.slice(0, 2) as c}
          <span class="dot">·</span>{c}
        {/each}
      </div>
    {/if}
  </div>
  {#if !prog}
    <span class="badge {st.tone} status">
      <md-icon>{st.icon}</md-icon>
      <span class="badge-text">{st.label}</span>
    </span>
  {/if}
  {#if canAct}
    {#if status === "update"}
      <md-filled-button class="act" onclick={act}>{actLabel}</md-filled-button>
    {:else}
      <md-outlined-button class="act" onclick={act}>{actLabel}</md-outlined-button>
    {/if}
  {/if}
</div>

<style>
  .row-card {
    position: relative;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    min-height: 76px;
    border-radius: var(--shape-l);
    background: var(--md-sys-color-surface-container-low);
    cursor: pointer;
    outline: none;
    --md-focus-ring-shape: var(--shape-l);
  }
  .row-card.compact {
    min-height: 56px;
    padding: 8px 12px;
    gap: 12px;
  }
  .row-card.has-update {
    background: color-mix(in srgb, var(--md-sys-color-primary-container) 45%, var(--md-sys-color-surface-container-low));
  }
  .main {
    flex: 1;
    min-width: 0;
  }
  .name-line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .name {
    font-size: 16px;
    font-weight: 500;
    line-height: 24px;
  }
  .compact .name {
    font-size: 15px;
  }
  .author {
    font-size: 13px;
  }
  .fav {
    --md-icon-size: 16px;
    color: var(--md-sys-color-tertiary);
    align-self: center;
  }
  .sub {
    font-size: 13.5px;
    line-height: 20px;
  }
  .ver {
    font-variant-numeric: tabular-nums;
  }
  .dot {
    margin: 0 6px;
  }
  .prog-line {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    padding-top: 2px;
  }
  md-linear-progress {
    width: 100%;
    --md-linear-progress-track-shape: 4px;
    --md-linear-progress-track-height: 4px;
    --md-linear-progress-active-indicator-height: 4px;
  }
  .act {
    flex-shrink: 0;
  }
  @media (max-width: 760px) {
    .badge-text {
      display: none;
    }
    .status {
      padding: 0 6px;
    }
  }
  @media (max-width: 480px) {
    .status {
      display: none;
    }
  }
</style>
