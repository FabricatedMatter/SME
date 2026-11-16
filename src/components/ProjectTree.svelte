<script lang="ts">
  import { editor, joinRel, partFiles } from "../lib/editor.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import Modal from "./Modal.svelte";

  const v = $derived(editor.view);
  let showNew = $state(false);
  let newName = $state("");
  let newW = $state(40);
  let newH = $state(30);
  let anchors = $state(true);
  let err = $state("");

  function createDungeon() {
    err = "";
    const name = newName.trim().replace(/\s+/g, "_");
    if (!/^[A-Za-z0-9_-]+$/.test(name)) {
      err = "Use letters, digits, _ and - only.";
      return;
    }
    try {
      const d = editor.newDungeon(name, { width: newW, height: newH, anchors });
      showNew = false;
      newName = "";
      const part = editor.files[d].parts[0];
      void editor.openMap(joinRel(d, partFiles(part)[0]), d);
    } catch (e) {
      err = String(e);
    }
  }

  function newLootFile() {
    const base = (editor.project?.metadata?.name ?? "mod").replace(/\W+/g, "").toLowerCase() || "mod";
    let p = `treasure/${base}.treasurepools`;
    let i = 2;
    while (editor.files[p]) p = `treasure/${base}${i++}.treasurepools`;
    editor.setFile(p, {});
    editor.view = { kind: "loot", path: p };
  }

  const isMap = (p: string) => v.kind === "map" && v.path === p;
</script>

<nav class="tree">
  <button class="item" class:on={v.kind === "mod"} onclick={() => (editor.view = { kind: "mod" })}>
    <PixelIcon name="star" scale={2} color="var(--gold)" /> Mod info & testing
  </button>

  <div class="sect">
    <span class="label">Dungeons</span>
    <button class="small flat" title="New microdungeon" onclick={() => (showNew = true)}><PixelIcon name="plus" scale={2} /></button>
  </div>
  {#each editor.dungeonPaths as d}
    {@const dj = editor.files[d]}
    <button class="item" class:on={v.kind === "dungeon" && v.path === d} onclick={() => (editor.view = { kind: "dungeon", path: d })} title={d}>
      <PixelIcon name="map" scale={2} /> {dj?.metadata?.name ?? d}{editor.dirtyFiles.has(d) ? " •" : ""}
    </button>
    {#each dj?.parts ?? [] as part}
      {#each partFiles(part) as f}
        {@const mp = joinRel(d, f)}
        <button class="item sub" class:on={isMap(mp)} onclick={() => editor.openMap(mp, d)} title={mp}>
          <PixelIcon name="pencil" scale={2} color={(dj.metadata?.anchor ?? []).includes(part.name) ? "var(--gold)" : "var(--muted)"} />
          {part.name}{editor.maps[mp]?.dirty ? " •" : ""}
        </button>
      {/each}
    {/each}
  {/each}
  {#each editor.dungeonPatches as dp}
    <div class="item static" title="{dp.path}: adds parts to the game's own “{dp.target}” dungeon with a JSON patch">
      <PixelIcon name="map" scale={2} color="var(--muted)" /> {dp.target} <span class="muted small">(patched)</span>
    </div>
    {#each dp.parts as part}
      {#each partFiles(part) as f}
        {@const mp = joinRel(dp.path, f)}
        <button class="item sub" class:on={isMap(mp)} onclick={() => editor.openMap(mp)} title={mp}>
          <PixelIcon name="pencil" scale={2} color={dp.anchors.includes(part.name) ? "var(--gold)" : "var(--muted)"} />
          {part.name}{editor.maps[mp]?.dirty ? " •" : ""}
        </button>
      {/each}
    {/each}
  {/each}
  {#if !editor.dungeonPaths.length && !editor.dungeonPatches.length}
    <p class="muted small pad">No dungeons yet.</p>
  {/if}
  {#if editor.unlistedMaps.length}
    <div class="sect"><span class="label" title="Map files that no .dungeon or dungeon patch uses">Other maps</span></div>
    {#each editor.unlistedMaps as mp}
      <button class="item sub" class:on={isMap(mp)} onclick={() => editor.openMap(mp)} title={mp}>
        <PixelIcon name="pencil" scale={2} color="var(--muted)" /> {mp.split("/").pop()}
      </button>
    {/each}
  {/if}

  <div class="sect">
    <span class="label">Loot</span>
    <button class="small flat" title="New treasure pool file" onclick={newLootFile}><PixelIcon name="plus" scale={2} /></button>
  </div>
  {#each editor.poolPaths as p}
    <button class="item" class:on={v.kind === "loot" && v.path === p} onclick={() => (editor.view = { kind: "loot", path: p })} title={p}>
      <PixelIcon name="chest" scale={2} /> {p.split("/").pop()}{editor.dirtyFiles.has(p) ? " •" : ""}
    </button>
  {:else}
    <p class="muted small pad">No loot pools yet.</p>
  {/each}

  <div class="sect"><span class="label">World</span></div>
  <button class="item" class:on={v.kind === "spawn"} onclick={() => (editor.view = { kind: "spawn" })}>
    <PixelIcon name="globe" scale={2} /> Spawn rules ({editor.spawnRules.length}){editor.spawnDirty ? " •" : ""}
  </button>
</nav>

{#if showNew}
  <Modal title="New microdungeon" icon="map" onclose={() => (showNew = false)} width={440}>
    <label>Name (unique, used by spawn rules)
      <input bind:value={newName} placeholder="smallhut" onkeydown={(e) => e.key === "Enter" && createDungeon()} />
    </label>
    <div class="row">
      <label>Width <input type="number" min="4" max="500" bind:value={newW} /></label>
      <label>Height <input type="number" min="4" max="500" bind:value={newH} /></label>
    </div>
    <label class="chk"><input type="checkbox" bind:checked={anchors} /> Start with ground/air anchors along the bottom</label>
    <p class="muted small">Creates <code>dungeons/microdungeons/{newName || "name"}/</code> with a .dungeon file and a first part.</p>
    {#if err}<p class="bad">{err}</p>{/if}
    {#snippet footer()}
      <div style="flex:1"></div>
      <button onclick={() => (showNew = false)}>Cancel</button>
      <button class="gold" onclick={createDungeon}>Create</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .tree {
    display: flex;
    flex-direction: column;
    padding: 6px;
    gap: 1px;
    overflow-y: auto;
    min-height: 0;
    flex: 1;
  }
  .item {
    justify-content: flex-start;
    background: none;
    border: none;
    box-shadow: none;
    padding: 3px 6px;
    text-shadow: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: flex;
  }
  .item:hover:not(:disabled) {
    background: #2a2a2a;
    box-shadow: none;
  }
  .item.on {
    background: #3a3000;
    color: #fff;
  }
  .static {
    align-items: center;
    gap: 6px;
    cursor: default;
    color: var(--text);
  }
  .static:hover {
    background: none !important;
  }
  .sub {
    padding-left: 22px;
    font-size: 14px;
  }
  .sect {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 10px;
    padding: 0 6px;
  }
  .pad {
    padding: 2px 8px;
    margin: 0;
  }
  .small {
    font-size: 13px;
  }
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
  .bad {
    color: var(--bad);
  }
</style>
