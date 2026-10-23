<script lang="ts">
  import { marked } from "marked";
  import DOMPurify from "dompurify";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let { source }: { source: string } = $props();

  const html = $derived(
    DOMPurify.sanitize(marked.parse(source || "", { async: false, gfm: true, breaks: false }) as string, {
      FORBID_TAGS: ["style", "iframe", "form", "input"],
    }),
  );

  // Links open in the system browser, not inside the app window.
  function onClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a");
    if (a?.href) {
      e.preventDefault();
      openUrl(a.href);
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="md selectable" onclick={onClick}>{@html html}</div>

<style>
  .md {
    font-size: 14px;
    line-height: 1.6;
    overflow-wrap: anywhere;
    color: var(--md-sys-color-on-surface);
  }
  .md :global(h1),
  .md :global(h2),
  .md :global(h3) {
    font-size: 16px;
    margin: 18px 0 6px;
  }
  .md :global(:first-child) {
    margin-top: 0;
  }
  .md :global(img) {
    max-width: 100%;
    border-radius: 8px;
  }
  .md :global(code) {
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12.5px;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--md-sys-color-surface-container-highest);
  }
  .md :global(pre) {
    padding: 12px;
    border-radius: 12px;
    overflow: auto;
    background: var(--md-sys-color-surface-container-highest);
  }
  .md :global(pre code) {
    padding: 0;
    background: none;
  }
  .md :global(ul),
  .md :global(ol) {
    padding-left: 22px;
  }
  .md :global(blockquote) {
    margin: 8px 0;
    padding-left: 12px;
    border-left: 3px solid var(--md-sys-color-outline-variant);
    color: var(--md-sys-color-on-surface-variant);
  }
  .md :global(table) {
    border-collapse: collapse;
    display: block;
    overflow: auto;
  }
  .md :global(td),
  .md :global(th) {
    border: 1px solid var(--md-sys-color-outline-variant);
    padding: 4px 8px;
  }
</style>
