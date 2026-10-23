<script lang="ts">
  import type { Asset } from "../lib/types";
  import { KIND_ICON, KIND_LABEL, bytes } from "../lib/format";
  import type { Snippet } from "svelte";

  let {
    assets,
    selected = $bindable(null),
    installedName = null,
    trailing,
  }: {
    assets: Asset[];
    selected?: string | null;
    installedName?: string | null;
    trailing?: Snippet<[Asset]>;
  } = $props();

  let showAll = $state(false);
  const compatible = $derived(assets.filter((a) => a.compatible));
  const shown = $derived(showAll || compatible.length === 0 ? assets : compatible);
  const hidden = $derived(assets.length - compatible.length);
</script>

<div class="assets">
  {#if assets.length === 0}
    <p class="muted empty">This release has no downloadable files.</p>
  {/if}
  {#each shown as a (a.name)}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="asset" class:sel={selected === a.name} class:incompatible={!a.compatible} onclick={() => (selected = a.name)}>
      <md-ripple></md-ripple>
      <md-icon class="radio" class:filled={selected === a.name}>
        {selected === a.name ? "radio_button_checked" : "radio_button_unchecked"}
      </md-icon>
      <md-icon class="kind">{KIND_ICON[a.kind]}</md-icon>
      <div class="info">
        <div class="name ellipsis selectable" title={a.name}>{a.name}</div>
        <div class="meta muted">
          {KIND_LABEL[a.kind]}{a.size ? ` · ${bytes(a.size)}` : ""}{a.sha256 ? " · SHA-256" : ""}
          {#if installedName === a.name} · <b>installed</b>{/if}
          {#if !a.compatible} · not for this system{/if}
        </div>
      </div>
      {@render trailing?.(a)}
    </div>
  {/each}
  {#if hidden > 0 && compatible.length > 0}
    <md-text-button class="more" onclick={() => (showAll = !showAll)}>
      <md-icon slot="icon">{showAll ? "expand_less" : "expand_more"}</md-icon>
      {showAll ? "Hide other files" : `Show ${hidden} other file${hidden === 1 ? "" : "s"}`}
    </md-text-button>
  {/if}
</div>

<style>
  .assets {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .asset {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--shape-m);
    background: var(--md-sys-color-surface-container-low);
    cursor: pointer;
  }
  .asset.sel {
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .asset.incompatible .info {
    opacity: 0.65;
  }
  .radio {
    color: var(--md-sys-color-primary);
    --md-icon-size: 20px;
  }
  .kind {
    color: var(--md-sys-color-on-surface-variant);
  }
  .info {
    flex: 1;
    min-width: 0;
  }
  .name {
    font-size: 14px;
    font-weight: 500;
  }
  .meta {
    font-size: 12.5px;
  }
  .sel .meta {
    color: inherit;
    opacity: 0.8;
  }
  .more {
    align-self: flex-start;
  }
  .empty {
    margin: 4px;
  }
</style>
