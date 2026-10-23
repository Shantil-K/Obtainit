<script lang="ts">
  import { onMount } from "svelte";
  import { store, startListeners, type Route } from "./lib/state.svelte";
  import Apps from "./pages/Apps.svelte";
  import AddApp from "./pages/AddApp.svelte";
  import AppDetail from "./pages/AppDetail.svelte";
  import ImportExport from "./pages/ImportExport.svelte";
  import SettingsPage from "./pages/Settings.svelte";
  import Logs from "./pages/Logs.svelte";
  import ThemeEditor from "./pages/ThemeEditor.svelte";
  import Snackbar from "./components/Snackbar.svelte";

  let failed = $state("");

  onMount(async () => {
    try {
      await startListeners();
      await store.load();
    } catch (e) {
      failed = String(e);
    }
  });

  const nav: { page: Route["page"]; label: string; icon: string }[] = [
    { page: "apps", label: "Apps", icon: "apps" },
    { page: "import", label: "Import", icon: "swap_vert" },
    { page: "settings", label: "Settings", icon: "settings" },
    { page: "logs", label: "Logs", icon: "receipt_long" },
  ];

  const active = $derived(
    store.route.page === "app" || store.route.page === "add"
      ? "apps"
      : store.route.page === "theme"
        ? "settings"
        : store.route.page,
  );

  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement;
    const typing = t?.tagName === "INPUT" || t?.tagName === "TEXTAREA" || t?.tagName?.startsWith("MD-");
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "n") {
      e.preventDefault();
      store.go({ page: "add" });
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r") {
      e.preventDefault();
      store.checkAll();
    } else if (e.key === "Escape" && !typing && (store.route.page === "app" || store.route.page === "add" || store.route.page === "theme")) {
      store.back();
    } else if (e.altKey && e.key === "ArrowLeft") {
      store.back();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if failed}
  <div class="fatal">
    <md-icon>error</md-icon>
    <h2 class="md-typescale-headline-small">Obtainit couldn't start</h2>
    <p class="mono selectable">{failed}</p>
  </div>
{:else if !store.ready}
  <div class="fatal"><md-circular-progress indeterminate></md-circular-progress></div>
{:else}
  <div class="shell">
    <nav class="rail" aria-label="Main">
      <div class="fab-slot">
        <md-fab
          size="medium"
          variant="primary"
          aria-label="Add app"
          title="Add app (Ctrl+N)"
          onclick={() => store.go({ page: "add" })}
        >
          <md-icon slot="icon">add</md-icon>
        </md-fab>
      </div>
      <div class="dests">
        {#each nav as n}
          <button class="dest" class:on={active === n.page} onclick={() => store.go({ page: n.page } as Route)}>
            <span class="pill">
              <md-ripple></md-ripple>
              <md-icon class:filled={active === n.page}>{n.icon}</md-icon>
              {#if n.page === "apps" && store.updatesCount > 0}
                <span class="count">{store.updatesCount}</span>
              {/if}
            </span>
            <span class="lbl">{n.label}</span>
          </button>
        {/each}
      </div>
    </nav>

    <main class="content">
      {#if store.route.page === "apps"}
        <Apps />
      {:else if store.route.page === "add"}
        <AddApp initialUrl={store.route.url ?? ""} />
      {:else if store.route.page === "app"}
        {#key store.route.id}
          <AppDetail id={store.route.id} />
        {/key}
      {:else if store.route.page === "import"}
        <ImportExport />
      {:else if store.route.page === "settings"}
        <SettingsPage />
      {:else if store.route.page === "theme"}
        {#key store.route}
          <ThemeEditor id={store.route.id} dark={store.route.dark ?? false} from={store.route.from} />
        {/key}
      {:else if store.route.page === "logs"}
        <Logs />
      {/if}
    </main>
  </div>
  <Snackbar />
{/if}

<style>
  .fatal {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
  }
  .fatal md-icon {
    --md-icon-size: 48px;
    color: var(--md-sys-color-error);
  }

  .shell {
    display: flex;
    height: 100%;
  }

  .rail {
    width: var(--rail-width);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 20px 0;
    gap: 28px;
    background: var(--md-sys-color-surface);
  }

  .dests {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .dest {
    all: unset;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    width: 72px;
    color: var(--md-sys-color-on-surface-variant);
  }

  .pill {
    position: relative;
    width: 56px;
    height: 32px;
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 200ms;
  }

  .dest.on .pill {
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }

  .dest.on {
    color: var(--md-sys-color-on-surface);
  }

  .dest:focus-visible .pill {
    outline: 2px solid var(--md-sys-color-primary);
  }

  .lbl {
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.5px;
  }

  .count {
    position: absolute;
    top: 0;
    right: 8px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--md-sys-color-error);
    color: var(--md-sys-color-on-error, #fff);
    font-size: 11px;
    font-weight: 600;
    line-height: 16px;
    text-align: center;
  }

  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--md-sys-color-surface);
  }

  /* Compact windows: bottom navigation bar. */
  @media (max-width: 600px) {
    .shell {
      flex-direction: column-reverse;
    }
    .rail {
      width: 100%;
      flex-direction: row;
      padding: 12px 0 16px;
      background: var(--md-sys-color-surface-container);
      justify-content: center;
    }
    .dests {
      flex-direction: row;
      flex: 1;
      justify-content: space-around;
    }
    .fab-slot {
      position: fixed;
      right: 16px;
      bottom: 96px;
      z-index: 5;
    }
  }
</style>
