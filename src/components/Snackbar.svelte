<script lang="ts">
  import { store } from "../lib/state.svelte";
</script>

{#if store.snack}
  {#key store.snack.id}
    <div class="snackbar" class:error={store.snack.error} role="status">
      <span class="text selectable">{store.snack.text}</span>
      {#if store.snack.action}
        <md-text-button
          class="action"
          onclick={() => {
            store.snack?.action?.run();
            store.snack = null;
          }}>{store.snack.action.label}</md-text-button
        >
      {/if}
      <md-icon-button aria-label="Dismiss" onclick={() => (store.snack = null)}>
        <md-icon>close</md-icon>
      </md-icon-button>
    </div>
  {/key}
{/if}

<style>
  .snackbar {
    position: fixed;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    z-index: 100;
    min-width: 288px;
    max-width: min(640px, calc(100vw - 32px));
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 4px 4px 16px;
    min-height: 48px;
    border-radius: var(--shape-xs);
    background: var(--md-sys-color-inverse-surface, #322f35);
    color: var(--md-sys-color-inverse-on-surface, #f5eff7);
    box-shadow: 0 3px 8px rgb(0 0 0 / 0.25);
    font-size: 14px;
    line-height: 20px;
    animation: rise 220ms cubic-bezier(0.2, 0, 0, 1);
  }
  .snackbar.error {
    border-left: 4px solid var(--md-sys-color-error);
  }
  .text {
    flex: 1;
    padding: 10px 0;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 30vh;
    overflow: auto;
  }
  .action {
    --md-text-button-label-text-color: var(--md-sys-color-inverse-primary, #d0bcff);
  }
  md-icon-button {
    --md-icon-button-icon-color: var(--md-sys-color-inverse-on-surface, #f5eff7);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 12px);
    }
  }
  @media (max-width: 600px) {
    .snackbar {
      bottom: 92px;
    }
  }
</style>
