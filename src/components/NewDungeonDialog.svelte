<script lang="ts">
  // "New microdungeon": either a dungeon of your own (spawn rules decide where it appears), or
  // new parts for one of the game's microdungeons (a patch), picked by where it spawns.
  import { untrack } from "svelte";
  import { api } from "../lib/api";
  import { editor, joinRel, partFiles } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import Modal from "./Modal.svelte";
  import SpawnSites, { rarityOf } from "./SpawnSites.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let mode = $state<"own" | "game">("own");
  let newName = $state("");
  let newW = $state(40);
  let newH = $state(30);
  let anchors = $state(true);
  let err = $state("");

  const modPrefix = () => String(editor.project?.metadata?.name ?? "my").replace(/\W+/g, "").toLowerCase() || "my";
  const validName = (n: string) => /^[A-Za-z0-9_-]+$/.test(n);

  function createOwn() {
    err = "";
    const name = newName.trim().replace(/\s+/g, "_");
    if (!validName(name)) return void (err = "Use letters, digits, _ and - only.");
    try {
      const d = editor.newDungeon(name, { width: newW, height: newH, anchors });
      onclose();
      void editor.openMap(joinRel(d, partFiles(editor.files[d].parts[0])[0]), d);
    } catch (e) {
      err = String(e);
    }
  }

  // ── adding to a game dungeon ──
  const SPECIAL: Record<string, string> = {
    encounterdungeons: "NPC encounters",
    undergroundencounterdungeons: "NPC encounters (caves)",
    terrainfeatures: "Terrain features",
  };
  function label(n: string) {
    if (SPECIAL[n]) return SPECIAL[n];
    const m = /^underground(\w+?)microdungeons$/.exec(n);
    if (m) return `Cave layer ${m[1]}`;
    const b = n.replace(/microdungeons$/, "");
    return b.charAt(0).toUpperCase() + b.slice(1);
  }
  let q = $state("");
  let picked = $state<string | null>(null);
  const patched = $derived(new Map(editor.dungeonPatches.map((p) => [p.target, p.parts.length])));
  const candidates = $derived.by(() => {
    const ms = game.index?.microSpawns ?? {};
    const names = Object.keys(ms).filter((n) => game.index?.dungeons[n]?.vanilla);
    const rank = (n: string) => {
      const rar = new Set(ms[n].map((s) => rarityOf(s.distribution)));
      // the mod's own targets first, then biome sets, then caves, then terrain shapes / encounters
      return patched.has(n) ? 0 : rar.has("terrain shape") ? 4 : rar.has("encounter") ? 3 : n.startsWith("underground") ? 2 : 1;
    };
    const s = q.trim().toLowerCase();
    return names
      .filter((n) => !s || `${n} ${label(n)} ${ms[n].map((x) => x.planet).join(" ")}`.toLowerCase().includes(s))
      .sort((a, b) => rank(a) - rank(b) || label(a).localeCompare(label(b)));
  });
  const rarities = (n: string) => [...new Set((game.index?.microSpawns[n] ?? []).map((s) => rarityOf(s.distribution)))];

  let partName = $state("");
  // part names already in the picked dungeon (the game's + what this mod adds)
  let takenParts = $state<Set<string>>(new Set());
  $effect(() => {
    const n = picked;
    takenParts = new Set();
    if (!n) return;
    const file = game.index?.dungeons[n]?.file;
    const mine = editor.dungeonPatches.find((p) => p.target === n)?.parts.map((p) => p.name) ?? [];
    untrack(() => {
      if (!partName) partName = `${modPrefix()}_${n.replace(/microdungeons$/, "")}1`;
    });
    if (file)
      void api.assetJson(file).then((j: any) => {
        if (picked === n) takenParts = new Set([...(j?.parts ?? []).map((p: any) => p.name), ...mine]);
      });
  });
  function createInGame() {
    err = "";
    const name = partName.trim().replace(/\s+/g, "_");
    const file = picked ? game.index?.dungeons[picked]?.file : null;
    if (!picked || !file) return void (err = "Pick one of the game's dungeons first.");
    if (!validName(name)) return void (err = "Use letters, digits, _ and - only.");
    if (takenParts.has(name)) return void (err = `${label(picked)} already has a part called ${name}.`);
    const mp = editor.addPatchPart(file, name, { width: newW, height: newH, anchors });
    onclose();
    void editor.openMap(mp);
  }
</script>

<Modal title="New microdungeon" icon="map" {onclose} width={mode === "game" ? 680 : 460}>
  <div class="seg">
    <button class:gold={mode === "own"} onclick={() => (mode = "own")}>Your own dungeon</button>
    <button class:gold={mode === "game"} onclick={() => (mode = "game")}>Add to a game dungeon</button>
  </div>

  {#if mode === "own"}
    <label>Name (unique, used by spawn rules)
      <input bind:value={newName} placeholder="smallhut" onkeydown={(e) => e.key === "Enter" && createOwn()} />
    </label>
    <p class="muted small">Creates <code>dungeons/microdungeons/{newName || "name"}/</code> with a .dungeon file and a first part. Choose where it spawns under Spawn rules.</p>
  {:else}
    <p class="muted small">
      Your parts join one of the game's own microdungeons and spawn wherever it does, mixed in with its parts. Nothing to set up under Spawn rules.
    </p>
    <input type="search" placeholder="Search by name or planet (e.g. tundra, ocean)…" bind:value={q} />
    <div class="legend small muted">
      <span class="chipdemo main">green</span> everywhere on that planet type ·
      <span class="chipdemo other">grey, dashed</span> only in some areas (secondary biome or minibiome)
    </div>
    <div class="list">
      {#each candidates as n (n)}
        <button class="cand" class:on={picked === n} onclick={() => (picked = n)}>
          <div class="chead">
            <b>{label(n)}</b>
            <span class="muted small">{n}</span>
            <span class="spacer"></span>
            {#each rarities(n) as r}<span class="rar {r.replace(' ', '')}">{r}</span>{/each}
            <span class="muted small">{game.index?.dungeons[n]?.parts ?? 0} parts</span>
            {#if patched.has(n)}<span class="mine small">+{patched.get(n)} from your mod</span>{/if}
          </div>
          <SpawnSites sites={game.index?.microSpawns[n] ?? []} compact />
        </button>
      {:else}
        <p class="muted small">No dungeon matches.</p>
      {/each}
    </div>
    <label>New part's name (unique in that dungeon; your mod's name in it avoids clashes with other mods)
      <input bind:value={partName} onkeydown={(e) => e.key === "Enter" && createInGame()} />
    </label>
  {/if}

  <div class="row">
    <label>Width <input type="number" min="4" max="500" bind:value={newW} /></label>
    <label>Height <input type="number" min="4" max="500" bind:value={newH} /></label>
  </div>
  <label class="chk"><input type="checkbox" bind:checked={anchors} /> Start with ground/air anchors along the bottom</label>
  {#if err}<p class="bad">{err}</p>{/if}

  {#snippet footer()}
    <div style="flex:1"></div>
    <button onclick={onclose}>Cancel</button>
    {#if mode === "own"}
      <button class="gold" onclick={createOwn}>Create</button>
    {:else}
      <button class="gold" disabled={!picked} onclick={createInGame}>Add part{picked ? ` to ${label(picked)}` : ""}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  label.chk {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }
  .row {
    display: flex;
    gap: 10px;
  }
  .row label {
    flex: 1;
  }
  .seg {
    display: flex;
    gap: 3px;
  }
  .seg button {
    flex: 1;
  }
  .small {
    font-size: 13px;
  }
  .bad {
    color: var(--bad);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
  }
  .chipdemo {
    padding: 0 5px;
  }
  .chipdemo.main {
    background: #1d2a1d;
    border: 1px solid #3c6a3c;
    color: var(--text, #ddd);
  }
  .chipdemo.other {
    background: #1a1a1a;
    border: 1px dashed #4a4a4a;
  }
  .list {
    max-height: 320px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: rgba(0, 0, 0, 0.35);
    padding: 3px;
  }
  .cand {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
    padding: 5px 8px;
    text-align: left;
    white-space: normal;
    background: #141414;
    border: 2px solid transparent;
    box-shadow: none;
  }
  .cand:hover {
    border-color: var(--btn-hi);
  }
  .cand.on {
    border-color: var(--gold);
    background: #2a2400;
  }
  .chead {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }
  .spacer {
    flex: 1;
  }
  .rar {
    font-size: 12px;
    padding: 0 5px;
    background: #2a2a2a;
  }
  .rar.rare {
    background: #2a1f3a;
    color: #c9a0ff;
  }
  .rar.common {
    background: #1f2a1f;
    color: #9fd59f;
  }
  .mine {
    color: var(--gold);
  }
</style>
