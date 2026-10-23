<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { store } from "../lib/state.svelte";
  import { ACCENTS, PRESETS, allThemes, contrast, customVars, effectiveAccent, isHex } from "../lib/theme";
  import type { CustomTheme, Settings } from "../lib/types";
  import TopBar from "../components/TopBar.svelte";
  import ColorField from "../components/ColorField.svelte";
  import SwitchRow from "../components/SwitchRow.svelte";

  let { id = undefined, dark = false, from = undefined }: { id?: string; dark?: boolean; from?: string } = $props();

  const s = $derived(store.settings!);

  // svelte-ignore state_referenced_locally
  const existing = id ? s.custom_themes.find((t) => t.id === id) : undefined;
  // svelte-ignore state_referenced_locally
  const source = from ? allThemes(s).find((t) => t.id === from) : undefined;

  function starter(isDark: boolean): CustomTheme {
    return isDark
      ? { id: "", name: "", dark: true, background: "#121212", surface: "#1E1E22", accent: "", text: "#ECEAF0" }
      : { id: "", name: "", dark: false, background: "#FFFFFF", surface: "#F3F1F6", accent: "", text: "#1C1B1F" };
  }

  // svelte-ignore state_referenced_locally
  let draft = $state<CustomTheme>(
    existing
      ? { ...existing }
      : source
        ? { ...source, id: "", name: `${source.name} (copy)` }
        : { ...starter(dark), name: dark ? "My dark theme" : "My light theme" },
  );
  // svelte-ignore state_referenced_locally
  let followAccent = $state(!draft.accent);
  let useNow = $state(true);

  // Preview on the whole app while editing.
  $effect(() => {
    const t = { ...$state.snapshot(draft), accent: followAccent ? "" : draft.accent };
    untrack(() => {
      store.previewTheme = t;
      store.applyTheme();
    });
  });
  onDestroy(() => {
    store.previewTheme = null;
    store.applyTheme();
  });

  function switchBase(toDark: boolean) {
    if (toDark === draft.dark) return;
    const st = starter(toDark);
    draft = { ...draft, dark: toDark, background: st.background, surface: st.surface, text: st.text };
  }

  const vars = $derived(customVars({ ...draft, accent: followAccent ? "" : draft.accent }, effectiveAccent(s)));
  const textContrast = $derived(isHex(draft.background) && isHex(draft.text) ? contrast(draft.background, draft.text) : 21);
  const accentContrast = $derived(contrast(vars.primary, vars.background));

  async function save() {
    const t: CustomTheme = {
      ...$state.snapshot(draft),
      accent: followAccent ? "" : draft.accent,
      id: existing?.id ?? `custom:${Date.now().toString(36)}`,
      name: draft.name.trim() || (draft.dark ? "Custom dark" : "Custom light"),
    };
    const list = existing ? s.custom_themes.map((x) => (x.id === t.id ? t : x)) : [...s.custom_themes, t];
    const patch: Partial<Settings> = { custom_themes: list };
    if (useNow) {
      if (t.dark) patch.dark_theme = t.id;
      else patch.light_theme = t.id;
      // Show it right away if the current mode doesn't match.
      if (s.theme !== "system") patch.theme = t.dark ? "dark" : "light";
    }
    store.previewTheme = null;
    await store.saveSettings(patch);
    store.toast(`Saved "${t.name}"`);
    store.back();
  }

  const templates = PRESETS;
</script>

<TopBar title={existing ? "Edit theme" : "New theme"} back>
  {#snippet actions()}
    <md-filled-button onclick={save}><md-icon slot="icon">save</md-icon>Save</md-filled-button>
  {/snippet}
</TopBar>

<div class="page narrow">
  <div class="banner info">
    <md-icon>visibility</md-icon>
    <div class="msg">The whole app shows your theme while you edit. Leave without saving to discard it.</div>
  </div>

  <div class="section-title"><md-icon>badge</md-icon>Basics</div>
  <div class="card">
    <md-outlined-text-field label="Theme name" value={draft.name}
      oninput={(e: Event) => (draft.name = (e.target as HTMLInputElement).value)}></md-outlined-text-field>
    <div class="setting">
      <div class="text">
        <div class="label">Mode</div>
        <div class="desc">Which slot this theme fills: your light mode or your dark mode.</div>
      </div>
      <div class="segmented">
        <button class:on={!draft.dark} onclick={() => switchBase(false)}><md-icon>light_mode</md-icon>Light</button>
        <button class:on={draft.dark} onclick={() => switchBase(true)}><md-icon>dark_mode</md-icon>Dark</button>
      </div>
    </div>
    {#if !existing}
      <md-outlined-select label="Start from a template" value=""
        onchange={(e: Event) => {
          const t = templates.find((x) => x.id === (e.target as HTMLSelectElement).value);
          if (t) {
            draft = { ...t, id: "", name: draft.name };
            followAccent = !t.accent;
          }
        }}>
        <md-select-option value=""><div slot="headline">—</div></md-select-option>
        {#each templates as t}
          <md-select-option value={t.id}><div slot="headline">{t.name} ({t.dark ? "dark" : "light"})</div></md-select-option>
        {/each}
      </md-outlined-select>
    {/if}
  </div>

  <div class="section-title"><md-icon>palette</md-icon>Colors</div>
  <div class="card">
    <ColorField label="Background" desc="Behind everything." bind:value={draft.background}
      swatches={draft.dark ? ["#000000", "#121212", "#1A1B26", "#1E1E2E", "#282A36", "#2E3440", "#002B36", "#0F1420"] : ["#FFFFFF", "#FAFAFA", "#FBF8F3", "#FDF6E3", "#ECEFF4", "#FAF4ED", "#F4F7FB", "#FFF8F0"]} />
    <ColorField label="Surface" desc="Cards and list rows." bind:value={draft.surface}
      swatches={draft.dark ? ["#101012", "#1E1E22", "#24283B", "#313244", "#343746", "#3B4252", "#073642", "#182033"] : ["#F3F1F6", "#F2EDE4", "#EEE8D5", "#E5E9F0", "#F2E9E1", "#EAF0F8", "#FFFFFF", "#F5F5F5"]} />
    <ColorField label="Text" desc="Main text and icons." bind:value={draft.text}
      swatches={draft.dark ? ["#FFFFFF", "#ECEAF0", "#E6E9F2", "#CDD6F4", "#F8F8F2", "#ECEFF4", "#EEE8D5", "#C0CAF5"] : ["#000000", "#1C1B1F", "#1F1B16", "#2E3440", "#073642", "#575279", "#202124", "#3B3B3B"]} />
    {#if textContrast < 4.5}
      <div class="banner tertiary small">
        <md-icon>warning</md-icon>
        <div class="msg">Text contrast is {textContrast.toFixed(1)}:1. Aim for at least 4.5:1 so text stays easy to read.</div>
      </div>
    {/if}
    <SwitchRow label="Use the app's accent color" desc="Follow the accent set in Settings instead of a fixed one."
      bind:checked={followAccent} />
    {#if !followAccent}
      <ColorField label="Accent" desc="Buttons, highlights, selection." bind:value={draft.accent} swatches={ACCENTS} />
      {#if accentContrast < 3}
        <div class="banner tertiary small">
          <md-icon>warning</md-icon>
          <div class="msg">The accent is hard to see on this background ({accentContrast.toFixed(1)}:1).</div>
        </div>
      {/if}
    {/if}
  </div>

  <div class="section-title"><md-icon>preview</md-icon>Preview</div>
  <div class="card preview">
    <div class="row wrap">
      <md-filled-button>Update</md-filled-button>
      <md-filled-tonal-button>Open</md-filled-tonal-button>
      <md-outlined-button>Uninstall</md-outlined-button>
      <md-text-button>Cancel</md-text-button>
    </div>
    <div class="row wrap">
      <md-filter-chip label="Selected" selected></md-filter-chip>
      <md-filter-chip label="Not selected"></md-filter-chip>
      <span class="badge primary"><md-icon>upgrade</md-icon>Update available</span>
      <span class="badge secondary"><md-icon>check_circle</md-icon>Up to date</span>
      <span class="badge error"><md-icon>error</md-icon>Check failed</span>
    </div>
    <div class="row">
      <md-switch selected></md-switch>
      <md-checkbox checked></md-checkbox>
      <md-linear-progress value="0.6" style="flex:1"></md-linear-progress>
    </div>
    <p class="muted">Secondary text looks like this.</p>
  </div>

  <div class="card">
    <SwitchRow label="Use this theme after saving" desc={draft.dark ? "Set it as your dark mode theme." : "Set it as your light mode theme."}
      bind:checked={useNow} />
  </div>
</div>

<style>
  .narrow {
    max-width: 760px;
  }
  .card > md-outlined-text-field,
  .card > md-outlined-select {
    margin: 8px 0;
  }
  .banner.small {
    font-size: 13px;
    margin: 4px 0 8px;
  }
  .preview {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
</style>
