<script lang="ts">
  import { store, statusOf } from "../lib/state.svelte";
  import { api } from "../lib/api";
  import type { App, Status } from "../lib/types";
  import TopBar from "../components/TopBar.svelte";
  import AppRow from "../components/AppRow.svelte";

  let query = $state("");
  let filter = $state<"all" | Status>("all");
  let category = $state<string | null>(null);
  let sortMenuOpen = $state(false);
  let confirmOpen = $state(false);
  let updating = $state(false);

  const order: Record<Status, number> = { update: 0, error: 1, installed: 2, track: 3, not_installed: 4 };

  const categories = $derived([...new Set(store.apps.flatMap((a) => a.categories))].sort());

  const counts = $derived.by(() => {
    const c: Record<string, number> = { all: store.apps.length };
    for (const a of store.apps) {
      const s = statusOf(a);
      c[s] = (c[s] ?? 0) + 1;
    }
    return c;
  });

  const sortBy = $derived(store.settings?.sort_by ?? "status");

  const visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    let list = store.apps.filter((a) => {
      if (filter !== "all" && statusOf(a) !== filter) return false;
      if (category && !a.categories.includes(category)) return false;
      if (!q) return true;
      return (
        a.name.toLowerCase().includes(q) ||
        a.author.toLowerCase().includes(q) ||
        a.url.toLowerCase().includes(q) ||
        a.categories.some((c) => c.toLowerCase().includes(q))
      );
    });
    const byName = (x: App, y: App) => x.name.localeCompare(y.name, undefined, { sensitivity: "base" });
    list = [...list].sort((x, y) => {
      if (x.favorite !== y.favorite) return x.favorite ? -1 : 1;
      switch (sortBy) {
        case "name":
          return byName(x, y);
        case "added":
          return y.added_at.localeCompare(x.added_at);
        case "released":
          return (y.latest?.published_at ?? "").localeCompare(x.latest?.published_at ?? "");
        default:
          return order[statusOf(x)] - order[statusOf(y)] || byName(x, y);
      }
    });
    return list;
  });

  const groups = $derived.by(() => {
    if (!store.settings?.group_by_category) return [{ name: "", apps: visible }];
    const m = new Map<string, App[]>();
    for (const a of visible) {
      const keys = a.categories.length ? a.categories : ["Uncategorized"];
      for (const k of keys) m.set(k, [...(m.get(k) ?? []), a]);
    }
    return [...m.entries()]
      .sort(([a], [b]) => (a === "Uncategorized" ? 1 : b === "Uncategorized" ? -1 : a.localeCompare(b)))
      .map(([name, apps]) => ({ name, apps }));
  });

  const updatable = $derived(store.apps.filter((a) => statusOf(a) === "update" && !a.settings.track_only));

  async function updateAll() {
    confirmOpen = false;
    updating = true;
    const failed = await store.run(() => api.updateAll());
    updating = false;
    if (failed && failed.length) store.toast(`Some updates failed:\n${failed.join("\n")}`, true);
    else if (failed) store.toast("All updates installed");
  }

  function askUpdateAll() {
    if (store.settings?.confirm_bulk_update) confirmOpen = true;
    else updateAll();
  }

  const filters: { key: "all" | Status; label: string }[] = [
    { key: "all", label: "All" },
    { key: "update", label: "Updates" },
    { key: "installed", label: "Installed" },
    { key: "not_installed", label: "Not installed" },
    { key: "track", label: "Track only" },
    { key: "error", label: "Errors" },
  ];

  const sorts = [
    { key: "status", label: "Status" },
    { key: "name", label: "Name" },
    { key: "released", label: "Latest release" },
    { key: "added", label: "Recently added" },
  ];
</script>

<TopBar title="Apps" subtitle={store.apps.length ? `${store.apps.length} tracked · ${store.updatesCount} update${store.updatesCount === 1 ? "" : "s"}` : ""}>
  {#snippet actions()}
    {#if updatable.length}
      <md-filled-tonal-button onclick={askUpdateAll} disabled={updating}>
        <md-icon slot="icon">system_update_alt</md-icon>
        Update all ({updatable.length})
      </md-filled-tonal-button>
    {/if}
    <md-icon-button
      title="Check for updates (Ctrl+R)"
      aria-label="Check for updates"
      onclick={() => store.checkAll()}
      disabled={store.checkingAll}
    >
      {#if store.checkingAll}
        <md-circular-progress indeterminate style="--md-circular-progress-size:24px"></md-circular-progress>
      {:else}
        <md-icon>refresh</md-icon>
      {/if}
    </md-icon-button>
    <span style="position:relative">
      <md-icon-button id="sort-anchor" aria-label="Sort" title="Sort" onclick={() => (sortMenuOpen = !sortMenuOpen)}>
        <md-icon>sort</md-icon>
      </md-icon-button>
      <md-menu anchor="sort-anchor" open={sortMenuOpen} onclosed={() => (sortMenuOpen = false)} positioning="popover">
        {#each sorts as s}
          <md-menu-item onclick={() => store.saveSettings({ sort_by: s.key })} selected={sortBy === s.key}>
            <md-icon slot="start">{sortBy === s.key ? "radio_button_checked" : "radio_button_unchecked"}</md-icon>
            <div slot="headline">{s.label}</div>
          </md-menu-item>
        {/each}
        <md-divider></md-divider>
        <md-menu-item onclick={() => store.saveSettings({ group_by_category: !store.settings?.group_by_category })}>
          <md-icon slot="start">{store.settings?.group_by_category ? "check_box" : "check_box_outline_blank"}</md-icon>
          <div slot="headline">Group by category</div>
        </md-menu-item>
      </md-menu>
    </span>
  {/snippet}
</TopBar>

<div class="page">
  {#if store.selfUpdate?.available}
    <div class="banner info self">
      <md-icon>system_update</md-icon>
      <div class="msg">Obtainit {store.selfUpdate.latest} is available (you have {store.selfUpdate.current}).</div>
      {#if store.selfUpdate.can_install}
        <md-filled-button onclick={() => store.updateSelf()} disabled={store.selfUpdating}>
          {store.selfUpdating ? "Updating…" : "Update & restart"}
        </md-filled-button>
      {:else}
        <md-text-button onclick={() => store.go({ page: "settings" })}>Details</md-text-button>
      {/if}
    </div>
  {/if}
  {#if store.apps.length === 0}
    <div class="empty">
      <div class="art"><md-icon>deployed_code_update</md-icon></div>
      <h2 class="md-typescale-headline-small">No apps yet</h2>
      <p class="muted">
        Add a GitHub, GitLab or Codeberg repository — or any download page — and Obtainit will keep it up to date.
      </p>
      <div class="row wrap" style="justify-content:center">
        <md-filled-button onclick={() => store.go({ page: "add" })}>
          <md-icon slot="icon">add</md-icon>Add an app
        </md-filled-button>
        <md-outlined-button onclick={() => store.go({ page: "import" })}>
          <md-icon slot="icon">upload_file</md-icon>Import (Obtainium, lifter)
        </md-outlined-button>
      </div>
    </div>
  {:else}
    <div class="search">
      <md-icon>search</md-icon>
      <input
        placeholder="Search apps"
        value={query}
        oninput={(e) => (query = (e.target as HTMLInputElement).value)}
        aria-label="Search apps"
      />
      {#if query}
        <md-icon-button aria-label="Clear" onclick={() => (query = "")}><md-icon>close</md-icon></md-icon-button>
      {/if}
    </div>

    <md-chip-set class="chips">
      {#each filters as f}
        {#if f.key === "all" || counts[f.key]}
          <md-filter-chip
            label="{f.label}{counts[f.key] ? ` ${counts[f.key]}` : ''}"
            selected={filter === f.key}
            onclick={() => (filter = filter === f.key && f.key !== "all" ? "all" : f.key)}
          ></md-filter-chip>
        {/if}
      {/each}
      {#each categories as c}
        <md-filter-chip label={c} selected={category === c} onclick={() => (category = category === c ? null : c)}>
          <md-icon slot="icon">label</md-icon>
        </md-filter-chip>
      {/each}
    </md-chip-set>

    {#each groups as g}
      {#if g.name}<div class="section-title"><md-icon>label</md-icon>{g.name}</div>{/if}
      <div class="list">
        {#each g.apps as app (app.id)}
          <AppRow {app} compact={store.settings?.list_density === "compact"} />
        {/each}
      </div>
    {/each}

    {#if visible.length === 0}
      <p class="muted none">No apps match.</p>
    {/if}
  {/if}
</div>

<md-dialog open={confirmOpen} onclosed={() => (confirmOpen = false)}>
  <md-icon slot="icon">system_update_alt</md-icon>
  <div slot="headline">Update {updatable.length} app{updatable.length === 1 ? "" : "s"}?</div>
  <div slot="content">
    <ul class="dlg-list">
      {#each updatable as a}
        <li><b>{a.name}</b> <span class="muted">{a.installed?.version} → {a.latest?.version}</span></li>
      {/each}
    </ul>
    {#if updatable.some((a) => a.installed?.kind && ["deb", "rpm", "pacman", "snap"].includes(a.installed.kind))}
      <p class="muted">System packages will ask for your password.</p>
    {/if}
  </div>
  <div slot="actions">
    <md-text-button onclick={() => (confirmOpen = false)}>Cancel</md-text-button>
    <md-filled-button onclick={updateAll}>Update all</md-filled-button>
  </div>
</md-dialog>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 56px;
    padding: 0 8px 0 20px;
    border-radius: 28px;
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface-variant);
    margin-bottom: 12px;
  }
  .search input {
    all: unset;
    flex: 1;
    font-size: 16px;
    color: var(--md-sys-color-on-surface);
  }
  .search input::placeholder {
    color: var(--md-sys-color-on-surface-variant);
  }
  .self {
    align-items: center;
    margin-bottom: 12px;
  }
  .chips {
    margin-bottom: 16px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .empty {
    max-width: 480px;
    margin: 10vh auto 0;
    text-align: center;
  }
  .art {
    width: 120px;
    height: 120px;
    margin: 0 auto 16px;
    border-radius: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
  .art md-icon {
    --md-icon-size: 56px;
  }
  .none {
    text-align: center;
    margin-top: 48px;
  }
  .dlg-list {
    margin: 0;
    padding-left: 18px;
    line-height: 1.8;
  }
</style>
