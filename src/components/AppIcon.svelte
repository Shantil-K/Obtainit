<script lang="ts">
  import { initials } from "../lib/format";

  let { name, url = null, size = 48 }: { name: string; url?: string | null; size?: number } = $props();
  let broken = $state(false);

  // A stable tonal color per name for the letter avatar.
  const hue = $derived([...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7));
</script>

<div class="icon" style="--size:{size}px; --hue:{hue}">
  {#if url && !broken}
    <img src={url} alt="" loading="lazy" onerror={() => (broken = true)} />
  {:else}
    <span>{initials(name)}</span>
  {/if}
</div>

<style>
  .icon {
    width: var(--size);
    height: var(--size);
    flex-shrink: 0;
    border-radius: calc(var(--size) * 0.3);
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in oklch, var(--md-sys-color-primary-container) 70%, hsl(var(--hue) 60% 60%));
    color: var(--md-sys-color-on-primary-container);
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  span {
    font-weight: 600;
    font-size: calc(var(--size) * 0.36);
    letter-spacing: 0.5px;
  }
</style>
