<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { editor } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import Window from "../lib/Window.svelte";
  import Modal from "./Modal.svelte";

  let { onsettings }: { onsettings: () => void } = $props();
  let recent = $state<string[]>([]);
  let showNew = $state(false);
  let spec = $state({ dir: "", name: "", friendly_name: "", author: "", description: "" });
  let err = $state("");

  onMount(async () => {
    const s = await api.getSettings();
    recent = s.recent;
    spec.author = s.author;
  });

  async function pickDir() {
    const d = (await open({ directory: true, title: "Where should the mod folder go?" })) as string | null;
    if (d) spec.dir = d.replace(/\/$/, "") + "/" + (spec.name || "my_microdungeons");
  }

  async function create() {
    err = "";
    if (!spec.name.trim() || !spec.dir.trim()) {
      err = "Please choose a name and folder.";
      return;
    }
    try {
      await editor.createMod({ ...spec, friendly_name: spec.friendly_name || spec.name });
      const s = await api.getSettings();
      if (spec.author && s.author !== spec.author) await api.setSettings({ ...s, author: spec.author });
      showNew = false;
    } catch (e) {
      err = String(e);
    }
  }
</script>

<div class="welcome">
  <Window title="SME" subtitle="Starbound Microdungeon Editor" icon="map">
    <div class="body">
      {#if game.error}
        <div class="panel err">
          <p><PixelIcon name="warn" scale={2} color="var(--bad)" /> {game.error}</p>
          <button class="gold" onclick={onsettings}>Set Starbound folder…</button>
        </div>
      {:else if game.loading}
        <p class="muted">{game.loading}</p>
      {:else if game.status?.install}
        <p class="muted small">Game data: {game.status.install}</p>
      {/if}
      <div class="actions">
        <button class="gold big" onclick={() => (showNew = true)}><PixelIcon name="plus" scale={3} /> New mod</button>
        <button class="big" onclick={() => editor.openFolder()}><PixelIcon name="folder" scale={3} /> Open mod folder</button>
        <button class="big" onclick={() => editor.openDungeonFile()}><PixelIcon name="file" scale={3} /> Open .dungeon / map</button>
      </div>
      {#if recent.length}
        <div class="label">Recent</div>
        <div class="recent">
          {#each recent as r}
            <button class="link" onclick={() => editor.openFolder(r)}>{r}</button>
          {/each}
        </div>
      {/if}
      <p class="muted small">
        Opening an old mod made with Tiled works directly: SME reads the same <code>.dungeon</code> and map files and writes them back in
        the same format.
      </p>
    </div>
  </Window>
</div>

{#if showNew}
  <Modal title="New mod" subtitle="A folder that becomes your Starbound mod" icon="plus" onclose={() => (showNew = false)}>
    <label>Internal name (no spaces) <input bind:value={spec.name} placeholder="permafrost_microdungeons" oninput={() => {
      spec.name = spec.name.replace(/\s+/g, "_");
      if (spec.dir) spec.dir = spec.dir.replace(/[^/]*$/, spec.name);
    }} /></label>
    <label>Display name <input bind:value={spec.friendly_name} placeholder="The &quot;Permafrost&quot; microdungeon six-pack" /></label>
    <label>Author <input bind:value={spec.author} /></label>
    <label>Description <textarea rows="2" bind:value={spec.description}></textarea></label>
    <label>Folder
      <span class="row"><input bind:value={spec.dir} placeholder="/path/to/folder" /><button onclick={pickDir}>Browse…</button></span>
    </label>
    {#if err}<p class="bad">{err}</p>{/if}
    {#snippet footer()}
      <div class="spacer"></div>
      <button onclick={() => (showNew = false)}>Cancel</button>
      <button class="gold" onclick={create}>Create</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .welcome {
    width: min(760px, calc(100vw - 32px));
    margin: 8vh auto;
  }
  .body {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .big {
    padding: 12px 18px 14px;
    font-size: 18px;
    gap: 10px;
  }
  .recent {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .err {
    display: flex;
    gap: 10px;
    align-items: center;
    justify-content: space-between;
  }
  .err p {
    margin: 0;
    color: var(--bad);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .row input {
    flex: 1;
  }
  .spacer {
    flex: 1;
  }
  .bad {
    color: var(--bad);
  }
  .small {
    font-size: 13px;
  }
</style>
