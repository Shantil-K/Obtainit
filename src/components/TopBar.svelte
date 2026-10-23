<script lang="ts">
  import type { Snippet } from "svelte";
  import { store } from "../lib/state.svelte";

  let {
    title,
    subtitle = "",
    back = false,
    actions,
  }: { title: string; subtitle?: string; back?: boolean; actions?: Snippet } = $props();
</script>

<header class="bar">
  {#if back}
    <md-icon-button aria-label="Back" title="Back (Esc)" onclick={() => store.back()}>
      <md-icon>arrow_back</md-icon>
    </md-icon-button>
  {/if}
  <div class="titles">
    <h1 class="md-typescale-headline-small ellipsis">{title}</h1>
    {#if subtitle}<div class="sub muted ellipsis">{subtitle}</div>{/if}
  </div>
  <div class="actions">{@render actions?.()}</div>
</header>

<style>
  .bar {
    position: sticky;
    top: 0;
    z-index: 4;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 24px 12px;
    min-height: 76px;
    background: var(--md-sys-color-surface);
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  h1 {
    margin: 0;
  }
  .sub {
    font-size: 13px;
    margin-top: 2px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  @media (max-width: 600px) {
    .bar {
      padding: 12px 12px 8px 12px;
    }
  }
</style>
