<script lang="ts">
  // A labelled color swatch that opens the spectrum picker in a dialog.
  import ColorPicker from "./ColorPicker.svelte";

  let {
    label,
    desc = "",
    value = $bindable(""),
    swatches = [],
    allowEmpty = "",
    onchange,
  }: {
    label: string;
    desc?: string;
    value?: string;
    swatches?: string[];
    /** When set, offers a "use default" choice with this label. */
    allowEmpty?: string;
    onchange?: (v: string) => void;
  } = $props();

  let open = $state(false);
  let draft = $state("#6750A4");

  function show() {
    draft = /^#[0-9a-f]{6}$/i.test(value) ? value : swatches[0] ?? "#6750A4";
    open = true;
  }

  function done(v: string) {
    value = v;
    onchange?.(v);
    open = false;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="setting clickable" onclick={show}>
  <md-ripple></md-ripple>
  <div class="text">
    <div class="label">{label}</div>
    {#if desc}<div class="desc">{desc}</div>{/if}
  </div>
  <span class="hex mono">{value ? value.toUpperCase() : allowEmpty}</span>
  <span class="chip" class:empty={!value} style="background:{value || 'transparent'}">
    {#if !value}<md-icon>auto_awesome</md-icon>{/if}
  </span>
</div>

<md-dialog open={open} onclosed={() => (open = false)}>
  <div slot="headline">{label}</div>
  <div slot="content" class="content">
    {#if open}<ColorPicker bind:value={draft} {swatches} />{/if}
  </div>
  <div slot="actions">
    {#if allowEmpty}
      <md-text-button onclick={() => done("")}>{allowEmpty}</md-text-button>
      <span style="flex:1"></span>
    {/if}
    <md-text-button onclick={() => (open = false)}>Cancel</md-text-button>
    <md-filled-button onclick={() => done(draft.toUpperCase())}>Select</md-filled-button>
  </div>
</md-dialog>

<style>
  .hex {
    color: var(--md-sys-color-on-surface-variant);
  }
  .chip {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: 50%;
    border: 1px solid var(--md-sys-color-outline-variant);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .chip.empty {
    border-style: dashed;
    color: var(--md-sys-color-primary);
  }
  .chip md-icon {
    --md-icon-size: 18px;
  }
  .content {
    width: min(360px, 80vw);
  }
  md-dialog {
    max-width: 420px;
  }
  [slot="actions"] {
    display: flex;
    width: 100%;
  }
</style>
