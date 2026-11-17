<script lang="ts">
  import { editor, joinRel, partFiles } from "../lib/editor.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
  import NewDungeonDialog from "./NewDungeonDialog.svelte";

  const v = $derived(editor.view);
  let showNew = $state(false);
  function newLootFile() {
    const base = (editor.project?.metadata?.name ?? "mod").replace(/\W+/g, "").toLowerCase() || "mod";
    let p = `treasure/${base}.treasurepools`;
    let i = 2;
    while (editor.files[p]) p = `treasure/${base}${i++}.treasurepools`;
    editor.setFile(p, {});
    editor.view = { kind: "loot", path: p };
  }

  const isMap = (p: string) => v.kind === "map" && v.path === p;

  // ── rows: right-click menu and trash button ──
  interface Row {
    label: string;
    icon: string;
    color?: string;
    sub?: boolean;
    title: string;
    on: boolean;
    open: () => void;
    del?: () => void;
    delLabel?: string;
  }
  let menu = $state<{ items: MenuItem[]; x: number; y: number; title: string } | null>(null);
  function openMenu(e: MouseEvent, r: Row) {
    e.preventDefault();
    const items: MenuItem[] = [{ label: "Open", icon: "file", action: r.open }];
    if (r.del) items.push({ label: r.delLabel ?? "Delete…", icon: "trash", danger: true, action: r.del });
    menu = { items, x: e.clientX, y: e.clientY, title: r.label };
  }

  function askDeleteDungeon(d: string) {
    const name = editor.files[d]?.metadata?.name ?? d;
    const maps = editor.dungeonMaps(d).length;
    const rules = editor.spawnRules.filter((r) => r.dungeons.includes(name)).length;
    if (!confirm(`Delete the dungeon "${name}"?\n\n${d} and its ${maps} map file(s) are deleted when you save.${rules ? `\nIt is also removed from ${rules} spawn rule(s).` : ""}`)) return;
    editor.deleteDungeon(d);
  }
  function askDeletePart(d: string, part: string) {
    if (!confirm(`Remove the part "${part}" from ${editor.files[d]?.metadata?.name ?? d}?\n\nIts map file is deleted when you save (unless another part uses it).`)) return;
    editor.deletePart(d, part);
  }
  function askDeletePatch(path: string, target: string, parts: number) {
    if (!confirm(`Delete ${path}?\n\nThe ${parts} part(s) it adds to the game's "${target}" dungeon and their map files are deleted when you save.`)) return;
    editor.deletePatch(path);
  }
  function askDeletePatchPart(path: string, target: string, part: string) {
    if (!confirm(`Remove the part "${part}" from what your mod adds to the game's "${target}" dungeon?\n\nIts map file is deleted when you save (unless another part uses it).`)) return;
    editor.deletePatchPart(path, part);
  }
  function askDeleteLoot(p: string) {
    const names = Object.keys(editor.files[p] ?? {});
    const used = editor.containersUsing(names);
    if (!confirm(`Delete ${p} with its ${names.length} loot table(s)?${used ? `\n\n${used} container(s) in open maps use them; Check will flag those.` : ""}`)) return;
    editor.deleteLootFile(p);
  }
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
    {@render row({
      label: (dj?.metadata?.name ?? d) + (editor.dirtyFiles.has(d) ? " •" : ""),
      icon: "map",
      title: d,
      on: v.kind === "dungeon" && v.path === d,
      open: () => (editor.view = { kind: "dungeon", path: d }),
      del: () => askDeleteDungeon(d),
      delLabel: "Delete dungeon…",
    })}
    {#each dj?.parts ?? [] as part}
      {#each partFiles(part) as f}
        {@const mp = joinRel(d, f)}
        {@render row({
          label: part.name + (editor.maps[mp]?.dirty ? " •" : ""),
          icon: "pencil",
          color: (dj.metadata?.anchor ?? []).includes(part.name) ? "var(--gold)" : "var(--muted)",
          sub: true,
          title: mp,
          on: isMap(mp),
          open: () => editor.openMap(mp, d),
          del: () => askDeletePart(d, part.name),
          delLabel: "Remove part…",
        })}
      {/each}
    {/each}
  {/each}
  {#each editor.dungeonPatches as dp}
    {@render row({
      label: `${dp.target} (patched)`,
      icon: "map",
      color: "var(--muted)",
      title: `${dp.path}: adds parts to the game's own “${dp.target}” dungeon with a JSON patch`,
      on: false,
      open: () => {},
      del: () => askDeletePatch(dp.path, dp.target, dp.parts.length),
      delLabel: "Delete patch…",
    })}
    {#each dp.parts as part}
      {#each partFiles(part) as f}
        {@const mp = joinRel(dp.path, f)}
        {@render row({
          label: part.name + (editor.maps[mp]?.dirty ? " •" : ""),
          icon: "pencil",
          color: dp.anchors.includes(part.name) ? "var(--gold)" : "var(--muted)",
          sub: true,
          title: mp,
          on: isMap(mp),
          open: () => editor.openMap(mp),
          del: () => askDeletePatchPart(dp.path, dp.target, part.name),
          delLabel: "Remove part…",
        })}
      {/each}
    {/each}
  {/each}
  {#if !editor.dungeonPaths.length && !editor.dungeonPatches.length}
    <p class="muted small pad">No dungeons yet.</p>
  {/if}
  {#if editor.unlistedMaps.length}
    <div class="sect"><span class="label" title="Map files that no .dungeon or dungeon patch uses">Other maps</span></div>
    {#each editor.unlistedMaps as mp}
      {@render row({
        label: mp.split("/").pop() ?? mp,
        icon: "pencil",
        color: "var(--muted)",
        sub: true,
        title: mp,
        on: isMap(mp),
        open: () => editor.openMap(mp),
        del: () => confirm(`Delete the map ${mp} when you save?`) && editor.deleteMap(mp),
        delLabel: "Delete map…",
      })}
    {/each}
  {/if}

  <div class="sect">
    <span class="label">Loot</span>
    <button class="small flat" title="New loot file" onclick={newLootFile}><PixelIcon name="plus" scale={2} /></button>
  </div>
  {#each editor.poolPaths as p}
    {@render row({
      label: (p.split("/").pop() ?? p) + (editor.dirtyFiles.has(p) ? " •" : ""),
      icon: "chest",
      title: p,
      on: v.kind === "loot" && v.path === p,
      open: () => (editor.view = { kind: "loot", path: p }),
      del: () => askDeleteLoot(p),
      delLabel: "Delete loot file…",
    })}
  {:else}
    <p class="muted small pad">No loot tables yet.</p>
  {/each}

  <div class="sect"><span class="label">World</span></div>
  <button class="item" class:on={v.kind === "spawn"} onclick={() => (editor.view = { kind: "spawn" })}>
    <PixelIcon name="globe" scale={2} /> Spawn rules ({editor.spawnRules.length}){editor.spawnDirty ? " •" : ""}
  </button>
</nav>

{#snippet row(r: Row)}
  <div class="rowwrap" role="group" oncontextmenu={(e) => openMenu(e, r)}>
    <button class="item" class:sub={r.sub} class:on={r.on} onclick={r.open} title={r.title}>
      <PixelIcon name={r.icon} scale={2} color={r.color} /> <span class="txt">{r.label}</span>
    </button>
    {#if r.del}
      <button class="trash small flat" title={r.delLabel} onclick={r.del}><PixelIcon name="trash" scale={2} /></button>
    {/if}
  </div>
{/snippet}

{#if menu}<ContextMenu items={menu.items} x={menu.x} y={menu.y} title={menu.title} onclose={() => (menu = null)} />{/if}

{#if showNew}<NewDungeonDialog onclose={() => (showNew = false)} />{/if}

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
  .rowwrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  .rowwrap > .item {
    flex: 1;
    min-width: 0;
  }
  .txt {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .trash {
    position: absolute;
    right: 2px;
    padding: 2px 4px;
    opacity: 0;
    color: var(--muted);
  }
  .rowwrap:hover .trash {
    opacity: 1;
  }
  .trash:hover {
    color: var(--bad);
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
</style>
