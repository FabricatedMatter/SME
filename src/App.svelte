<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import DungeonEditor from "./components/DungeonEditor.svelte";
  import LootEditor from "./components/LootEditor.svelte";
  import MapEditor from "./components/MapEditor.svelte";
  import ModInfo from "./components/ModInfo.svelte";
  import Problems from "./components/Problems.svelte";
  import ProjectTree from "./components/ProjectTree.svelte";
  import SettingsDialog from "./components/SettingsDialog.svelte";
  import SpawnEditor from "./components/SpawnEditor.svelte";
  import Welcome from "./components/Welcome.svelte";
  import { api } from "./lib/api";
  import { editor } from "./lib/editor.svelte";
  import { game } from "./lib/game.svelte";
  import Logo from "./lib/Logo.svelte";
  import PixelIcon from "./lib/PixelIcon.svelte";

  let showSettings = $state(false);
  const v = $derived(editor.view);
  const title = $derived(editor.project?.metadata?.friendlyName ?? editor.project?.metadata?.name ?? editor.project?.root.split("/").pop() ?? "");

  $effect(() => {
    const t = `${editor.dirty ? "• " : ""}${title ? title + " · " : ""}SME: Starbound Microdungeon Editor`;
    void getCurrentWindow().setTitle(t).catch(() => {});
  });

  onMount(() => {
    void game.load().then(async () => {
      const p = await api.startupPath();
      if (p) await editor.openPath(p);
      const sv = await api.startupView();
      if (sv === "spawn" || sv === "mod") editor.view = { kind: sv };
      else if (sv === "loot" && editor.poolPaths[0]) editor.view = { kind: "loot", path: editor.poolPaths[0] };
      else if (sv === "dungeon" && editor.dungeonPaths[0]) editor.view = { kind: "dungeon", path: editor.dungeonPaths[0] };
      if (p && (await api.selftestMode())) await (await import("./lib/selftest")).selftest();
    });
    const key = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void editor.saveAll();
      }
    };
    window.addEventListener("keydown", key);
    const unlisten = getCurrentWindow().onCloseRequested(async (ev) => {
      if (editor.dirty && !confirm("You have unsaved changes. Quit anyway?")) ev.preventDefault();
    });
    return () => {
      window.removeEventListener("keydown", key);
      void unlisten.then((f) => f());
    };
  });
</script>

{#if !editor.project}
  <Welcome onsettings={() => (showSettings = true)} />
{:else}
  <div class="app">
    <header class="top">
      <div class="brand"><Logo scale={2} /> SME</div>
      <div class="proj" title={editor.project.root}>{title}</div>
      <div class="actions">
        <button class="gold" disabled={!editor.dirty} title="Save all (Ctrl+S)" onclick={() => editor.saveAll()}><PixelIcon name="disk" scale={2} /> Save</button>
        <button title="Test in game" onclick={() => (editor.view = { kind: "mod" })}><PixelIcon name="play" scale={2} /> Test</button>
        <button title="Settings" onclick={() => (showSettings = true)}><PixelIcon name="gear" scale={2} /></button>
        <button title="Close project" onclick={() => editor.close()}><PixelIcon name="x" scale={2} /></button>
      </div>
    </header>
    <div class="body">
      <aside class="left"><ProjectTree /></aside>
      <main class="center">
        {#if game.loading}
          <div class="loading">{game.loading}</div>
        {:else if game.error}
          <div class="loading bad">{game.error} <button onclick={() => (showSettings = true)}>Settings</button></div>
        {:else if v.kind === "map" && editor.maps[v.path]}
          {#key v.path}<MapEditor md={editor.maps[v.path]} />{/key}
        {:else if v.kind === "dungeon"}
          {#key v.path}<DungeonEditor path={v.path} />{/key}
        {:else if v.kind === "loot"}
          {#key v.path}<LootEditor path={v.path} />{/key}
        {:else if v.kind === "spawn"}
          <SpawnEditor />
        {:else}
          <ModInfo />
        {/if}
      </main>
    </div>
    <Problems />
  </div>
{/if}

{#if showSettings}<SettingsDialog onclose={() => (showSettings = false)} />{/if}

<div class="toasts">
  {#each editor.toasts as t (t.id)}
    <div class="toast {t.kind}">{t.text}</div>
  {/each}
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 6px 10px;
    background: var(--win-head);
    border-bottom: 2px solid #000;
  }
  .brand {
    display: flex;
    gap: 8px;
    align-items: center;
    color: var(--gold);
    font-size: 20px;
  }
  .proj {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .left {
    width: 240px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--win-body);
    border-right: 2px solid #000;
    min-height: 0;
  }
  .center {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: rgba(20, 20, 20, 0.6);
  }
  .loading {
    margin: auto;
    color: var(--muted);
  }
  .bad {
    color: var(--bad);
  }
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 48px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 100;
    max-width: 520px;
  }
  .toast {
    background: #141414;
    border: 2px solid #000;
    box-shadow:
      inset 0 0 0 2px #3a3a3a,
      4px 4px 0 rgba(0, 0, 0, 0.45);
    padding: 8px 12px;
  }
  .toast.error {
    box-shadow:
      inset 0 0 0 2px var(--btn-red-mid),
      4px 4px 0 rgba(0, 0, 0, 0.45);
    color: #ffb0b0;
  }
  .toast.good {
    box-shadow:
      inset 0 0 0 2px #2f6b22,
      4px 4px 0 rgba(0, 0, 0, 0.45);
    color: var(--good);
  }
</style>
