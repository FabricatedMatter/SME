<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { game } from "../lib/game.svelte";
  import type { Settings } from "../lib/types";
  import Modal from "./Modal.svelte";

  let { onclose }: { onclose: () => void } = $props();
  let s = $state<Settings>({ starbound_dir: "", author: "", recent: [] });
  let err = $state("");

  onMount(async () => {
    s = await api.getSettings();
  });

  async function browse() {
    const d = (await open({ directory: true, title: "Starbound folder (contains assets/ and tiled/)" })) as string | null;
    if (d) s.starbound_dir = d;
  }

  async function saveSettings() {
    err = "";
    const st = await api.setSettings($state.snapshot(s));
    if (!st.install) {
      err = st.error ?? "Starbound not found there";
      return;
    }
    await game.load();
    onclose();
  }
</script>

<Modal title="Settings" icon="gear" {onclose}>
  <label>
    Starbound folder <span class="muted small">(leave empty to auto-detect from Steam)</span>
    <span class="row"><input bind:value={s.starbound_dir} placeholder={game.status?.install ?? "~/.local/share/Steam/steamapps/common/Starbound"} /><button onclick={browse}>Browse…</button></span>
  </label>
  <label>Default author for new mods <input bind:value={s.author} /></label>
  <p class="muted small">SME reads blocks, objects and loot from your own install at runtime. Nothing from the game is copied.</p>
  {#if err}<p class="bad">{err}</p>{/if}
  {#snippet footer()}
    <div style="flex:1"></div>
    <button onclick={onclose}>Cancel</button>
    <button class="gold" onclick={saveSettings}>Save</button>
  {/snippet}
</Modal>

<style>
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
  .bad {
    color: var(--bad);
  }
  .small {
    font-size: 13px;
  }
</style>
