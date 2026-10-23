<script lang="ts">
  import type { Snippet } from "svelte";

  // A miniature app window drawn in a theme's colors.
  let {
    name,
    vars,
    selected = false,
    onselect,
    badge = "",
    actions,
  }: {
    name: string;
    vars: Record<string, string>;
    selected?: boolean;
    onselect: () => void;
    badge?: string;
    actions?: Snippet;
  } = $props();
</script>

<div class="wrap" class:selected>
  <button class="card" onclick={onselect} aria-pressed={selected} title={name}>
    <div class="mini" style="background:{vars.surface}; color:{vars.onSurface}">
      <div class="rail">
        <span class="dot" style="background:{vars.primaryContainer}"></span>
        <span class="pill" style="background:{vars.secondaryContainer}"></span>
        <span class="pill faint" style="background:{vars.outlineVariant}"></span>
      </div>
      <div class="body">
        <div class="row" style="background:{vars.surfaceContainerLow}">
          <span class="ic" style="background:{vars.primaryContainer}"></span>
          <span class="lines"><span style="background:{vars.onSurface}"></span><span style="background:{vars.onSurfaceVariant}"></span></span>
          <span class="btn" style="background:{vars.primary}"></span>
        </div>
        <div class="row" style="background:{vars.surfaceContainerLow}">
          <span class="ic" style="background:{vars.tertiaryContainer}"></span>
          <span class="lines"><span style="background:{vars.onSurface}"></span><span style="background:{vars.onSurfaceVariant}"></span></span>
          <span class="btn ghost" style="border-color:{vars.outline}"></span>
        </div>
      </div>
      {#if selected}
        <span class="check" style="background:{vars.primary}; color:{vars.onPrimary}"><md-icon>check</md-icon></span>
      {/if}
    </div>
  </button>
  <div class="foot">
    <span class="name ellipsis">{name}</span>
    {#if badge}<span class="badge">{badge}</span>{/if}
    {@render actions?.()}
  </div>
</div>

<style>
  .wrap {
    width: 160px;
    flex-shrink: 0;
  }
  .card {
    all: unset;
    display: block;
    width: 100%;
    cursor: pointer;
    border-radius: var(--shape-l);
    outline: 2px solid var(--md-sys-color-outline-variant);
    outline-offset: -1px;
    overflow: hidden;
  }
  .card:focus-visible {
    outline: 3px solid var(--md-sys-color-primary);
  }
  .selected .card {
    outline: 3px solid var(--md-sys-color-primary);
  }
  .mini {
    position: relative;
    height: 96px;
    display: flex;
    gap: 6px;
    padding: 8px;
  }
  .rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    width: 16px;
  }
  .dot {
    width: 14px;
    height: 14px;
    border-radius: 4px;
  }
  .pill {
    width: 14px;
    height: 7px;
    border-radius: 4px;
  }
  .pill.faint {
    opacity: 0.6;
  }
  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 6px;
    border-radius: 7px;
  }
  .ic {
    width: 14px;
    height: 14px;
    border-radius: 4px;
    flex-shrink: 0;
  }
  .lines {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .lines span {
    height: 3px;
    border-radius: 2px;
    width: 80%;
  }
  .lines span + span {
    width: 55%;
    opacity: 0.8;
  }
  .btn {
    width: 22px;
    height: 10px;
    border-radius: 5px;
  }
  .btn.ghost {
    border: 1px solid;
    box-sizing: border-box;
  }
  .check {
    position: absolute;
    right: 6px;
    bottom: 6px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .check md-icon {
    --md-icon-size: 16px;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 2px;
    min-height: 36px;
    padding: 2px 0 0 4px;
  }
  .name {
    flex: 1;
    font-size: 13px;
    font-weight: 500;
  }
  .badge {
    height: 20px;
    font-size: 11px;
    padding: 0 6px;
  }
  .foot :global(md-icon-button) {
    --md-icon-button-icon-size: 18px;
    width: 32px;
    height: 32px;
  }
</style>
