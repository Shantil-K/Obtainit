<script lang="ts">
  let {
    label,
    desc = "",
    icon = "",
    checked = $bindable(false),
    disabled = false,
    onchange,
  }: {
    label: string;
    desc?: string;
    icon?: string;
    checked?: boolean;
    disabled?: boolean;
    onchange?: (v: boolean) => void;
  } = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="setting clickable" class:disabled onclick={toggle}>
  <md-ripple></md-ripple>
  {#if icon}<md-icon>{icon}</md-icon>{/if}
  <div class="text">
    <div class="label">{label}</div>
    {#if desc}<div class="desc">{desc}</div>{/if}
  </div>
  <md-switch
    selected={checked}
    {disabled}
    aria-label={label}
    onclick={(e: Event) => e.stopPropagation()}
    onchange={(e: Event) => {
      checked = (e.target as HTMLInputElement & { selected: boolean }).selected;
      onchange?.(checked);
    }}
  ></md-switch>
</div>

<style>
  .disabled {
    opacity: 0.5;
    pointer-events: none;
  }
</style>
