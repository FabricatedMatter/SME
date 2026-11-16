<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import type { MapDoc } from "../lib/mapdoc.svelte";
  import { MapRenderer } from "../lib/render/mapRenderer";
  import type { Obj, ObjGroup } from "../lib/types";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import Thumb from "./Thumb.svelte";

  let { md }: { md: MapDoc } = $props();

  const sel = $derived.by(() => {
    void md.rev;
    return editor.selected.map((id) => md.findObject(id)).filter((e): e is { group: ObjGroup; obj: Obj } => !!e);
  });
  const one = $derived(sel.length === 1 ? sel[0] : null);
  const merged = $derived(one ? md.objectProps(one.group, one.obj) : {});
  const objName = $derived(merged.object ? String(merged.object) : "");
  const info = $derived(objName ? game.objects[objName] : undefined);
  const brush = $derived(one?.obj.tile ? game.brush(md.ref(one.obj.tile)) : undefined);

  // ── parameters (JSON stored as a string property) ──
  let paramsText = $state("");
  let paramsError = $state("");
  let paramsFor = -1;
  $effect(() => {
    const o = one?.obj;
    if (!o) return;
    if (paramsFor === o.id) return;
    paramsFor = o.id;
    paramsText = paramsPretty(o);
    paramsError = "";
  });

  function paramsObj(o: Obj): Record<string, any> {
    const p = o.properties.parameters;
    if (!p) return {};
    if (typeof p === "object") return p;
    try {
      return JSON.parse(p);
    } catch {
      return {};
    }
  }
  function paramsPretty(o: Obj) {
    const p = o.properties.parameters;
    if (!p) return "";
    try {
      return JSON.stringify(typeof p === "string" ? JSON.parse(p) : p, null, 2);
    } catch {
      return String(p);
    }
  }

  function setParams(o: Obj, obj: Record<string, any>) {
    md.begin();
    if (Object.keys(obj).length) o.properties = { ...o.properties, parameters: JSON.stringify(obj) };
    else {
      const { parameters: _, ...rest } = o.properties;
      o.properties = rest;
    }
    md.commit();
    md.touchObjects();
    paramsText = paramsPretty(o);
  }

  function commitParamsText() {
    if (!one) return;
    if (!paramsText.trim()) {
      setParams(one.obj, {});
      paramsError = "";
      return;
    }
    try {
      const j = JSON.parse(paramsText);
      if (typeof j !== "object" || Array.isArray(j)) throw new Error("must be a JSON object");
      setParams(one.obj, j);
      paramsError = "";
    } catch (e) {
      paramsError = String(e);
    }
  }

  // ── chest loot ──
  const pools = $derived(one ? ((paramsObj(one.obj).treasurePools as string[] | undefined) ?? []) : []);
  let newPool = $state("");
  const allPools = $derived([...Object.keys(editor.projectPools), ...Object.keys(game.index?.pools ?? {})]);

  function addPool() {
    if (!one || !newPool.trim()) return;
    const p = paramsObj(one.obj);
    p.treasurePools = [...(p.treasurePools ?? []), newPool.trim()];
    setParams(one.obj, p);
    newPool = "";
  }
  function removePool(i: number) {
    if (!one) return;
    const p = paramsObj(one.obj);
    p.treasurePools = (p.treasurePools ?? []).filter((_: string, j: number) => j !== i);
    if (!p.treasurePools.length) delete p.treasurePools;
    setParams(one.obj, p);
  }
  function openPool(name: string) {
    const f = editor.projectPools[name];
    if (f) editor.view = { kind: "loot", path: f };
  }

  // ── raw properties ──
  let newKey = $state("");
  let newVal = $state("");
  function setProp(o: Obj, k: string, v: string | null) {
    md.begin();
    const p = { ...o.properties };
    if (v === null) delete p[k];
    else p[k] = v;
    o.properties = p;
    md.commit();
    md.touchObjects();
  }

  function flip(o: Obj) {
    md.begin();
    o.flipX = !o.flipX;
    md.commit();
    md.touchObjects();
  }

  function moveGroup(o: Obj, from: ObjGroup, to: string) {
    md.begin();
    from.objects = from.objects.filter((x) => x.id !== o.id);
    md.group(to).objects.push(o);
    md.commit();
    md.touchObjects();
  }

  // ── map size ──
  let newW = $state(0);
  let newH = $state(0);
  let anchorX = $state<"left" | "center" | "right">("center");
  let anchorY = $state<"top" | "center" | "bottom">("bottom");
  $effect(() => {
    void md.rev;
    newW = md.width;
    newH = md.height;
  });
  function resize() {
    const dw = newW - md.width,
      dh = newH - md.height;
    const dx = anchorX === "left" ? 0 : anchorX === "right" ? dw : Math.floor(dw / 2);
    const dy = anchorY === "top" ? 0 : anchorY === "bottom" ? dh : Math.floor(dh / 2);
    md.resize(newW, newH, dx, dy);
  }

  const counts = $derived.by(() => {
    void md.rev;
    let objects = 0,
      anchors = 0,
      spawns = 0,
      wires = 0;
    for (const { obj } of md.allObjects()) {
      if (obj.polyline) wires++;
      else if (!obj.tile) spawns++;
      else if (md.ref(obj.tile)?.tileset.endsWith("miscellaneous.json")) anchors++;
      else objects++;
    }
    return { objects, anchors, spawns, wires };
  });
</script>

<div class="insp">
  {#if one}
    {@const o = one.obj}
    <div class="head">
      {#if o.tile}<Thumb image={brush?.image ?? null} size={40} flip={o.flipX} />{/if}
      <div class="titles">
        <div class="title">
          {#if o.polyline}Wire ({o.polyline.length} points){:else if !o.tile}{MapRenderer.rectLabel({ ...one.group.properties, ...o.properties }).text}{:else}{brush?.label ?? objName ?? "Tile object"}{/if}
        </div>
        <div class="muted small">
          {#if objName}{objName}{/if}
          {#if info}· {info.category || info.object_type}{/if}
          {#if o.tile}
            {@const a = md.tileAnchor(one.group, o)}
            · at {a.x},{a.y}
          {:else if !o.polyline}
            · {Math.round(o.width / 8)}×{Math.round(o.height / 8)} tiles
          {/if}
        </div>
      </div>
      <button class="red small" title="Delete (Del)" onclick={() => {
        md.begin();
        md.removeObject(o.id);
        md.commit();
        editor.selected = [];
      }}><PixelIcon name="trash" scale={2} /></button>
    </div>

    {#if o.tile && objName}
      <label class="row"><input type="checkbox" checked={o.flipX} onchange={() => flip(o)} /> Facing left (flipped)</label>
    {/if}

    {#if info?.container}
      <div class="section">
        <div class="label"><PixelIcon name="chest" scale={2} color="var(--gold)" /> Loot ({info.slot_count} slots)</div>
        {#if pools.length}
          <p class="muted small">The chest rolls one of these pools when the world generates.</p>
          {#each pools as p, i}
            <div class="pool">
              {#if editor.projectPools[p]}
                <button class="link" onclick={() => openPool(p)}>{p}</button>
              {:else}
                <span class:bad={!game.index?.pools[p]}>{p}</span>
                {#if game.index?.pools[p]}<span class="muted small">vanilla</span>{:else}<span class="bad small">unknown</span>{/if}
              {/if}
              <button class="small flat" title="Remove" onclick={() => removePool(i)}><PixelIcon name="x" scale={2} /></button>
            </div>
          {/each}
        {:else}
          <p class="muted small">No pool set: the chest uses its default loot (often empty).</p>
        {/if}
        <div class="row">
          <input list="dl-pools" placeholder="Add a treasure pool…" bind:value={newPool} onkeydown={(e) => e.key === "Enter" && addPool()} />
          <button class="small gold" onclick={addPool}>Add</button>
        </div>
        <datalist id="dl-pools">{#each allPools as p}<option value={p}></option>{/each}</datalist>
      </div>
    {/if}

    {#if o.tile && objName}
      <div class="section">
        <div class="label">Object parameters (JSON)</div>
        <textarea rows="5" bind:value={paramsText} onblur={commitParamsText} placeholder={'{ "treasurePools": ["…"] }'}></textarea>
        {#if paramsError}<span class="bad small">{paramsError}</span>{/if}
      </div>
    {/if}

    <div class="section">
      <div class="label">Properties</div>
      {#each Object.entries(o.properties).filter(([k]) => k !== "parameters") as [k, v]}
        <div class="prop">
          <span class="key" title={k}>{k}</span>
          <input value={typeof v === "string" ? v : JSON.stringify(v)} onchange={(e) => setProp(o, k, (e.currentTarget as HTMLInputElement).value)} />
          <button class="small flat" onclick={() => setProp(o, k, null)}><PixelIcon name="x" scale={2} /></button>
        </div>
      {/each}
      <div class="prop">
        <input placeholder="key" bind:value={newKey} />
        <input placeholder="value" bind:value={newVal} />
        <button
          class="small"
          onclick={() => {
            if (newKey.trim()) setProp(o, newKey.trim(), newVal);
            newKey = newVal = "";
          }}><PixelIcon name="plus" scale={2} /></button>
      </div>
      <label class="row small">
        Layer group
        <select value={one.group.name} onchange={(e) => moveGroup(o, one.group, (e.currentTarget as HTMLSelectElement).value)}>
          {#each md.doc.groups as g}<option>{g.name}</option>{/each}
        </select>
      </label>
    </div>
  {:else if sel.length > 1}
    <p class="pad">{sel.length} objects selected. Drag to move, Del to delete, Ctrl+C to copy.</p>
  {:else}
    <div class="section">
      <div class="label">Map</div>
      <p class="small muted">{md.path}</p>
      <p class="small">{md.width}×{md.height} tiles · {counts.objects} objects · {counts.anchors} anchors · {counts.spawns} spawns · {counts.wires} wires</p>
      <div class="row">
        <label>W <input type="number" min="1" max="1000" bind:value={newW} /></label>
        <label>H <input type="number" min="1" max="1000" bind:value={newH} /></label>
      </div>
      <div class="row">
        <select bind:value={anchorX}><option value="left">keep left</option><option value="center">centre</option><option value="right">keep right</option></select>
        <select bind:value={anchorY}><option value="top">keep top</option><option value="center">centre</option><option value="bottom">keep bottom</option></select>
      </div>
      <button class="small" disabled={newW === md.width && newH === md.height} onclick={resize}>Resize map</button>
    </div>
    {#if md.warnings.length}
      <div class="section">
        <div class="label">Import notes</div>
        {#each md.warnings as w}<p class="small warn">{w}</p>{/each}
      </div>
    {/if}
    <p class="pad muted small">Select an object (V) to edit it. Chests get a loot picker here.</p>
  {/if}
</div>

<style>
  .insp {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    overflow-y: auto;
    min-height: 0;
  }
  .head {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-top: 2px solid rgba(255, 255, 255, 0.08);
    padding-top: 6px;
  }
  .label {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .row input,
  .row select {
    flex: 1;
    min-width: 0;
    width: 0;
  }
  .row label {
    display: flex;
    gap: 4px;
    align-items: center;
    flex: 1;
  }
  .pool {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .pool > :first-child {
    flex: 1;
  }
  .prop {
    display: grid;
    grid-template-columns: 1fr 1.4fr auto;
    gap: 4px;
    align-items: center;
  }
  .prop input {
    min-width: 0;
    width: 100%;
  }
  .key {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--muted);
  }
  textarea {
    width: 100%;
    font-size: 13px;
    resize: vertical;
  }
  .small {
    font-size: 13px;
  }
  .bad {
    color: var(--bad);
  }
  .warn {
    color: var(--warn);
  }
  .pad {
    padding: 4px;
  }
  p {
    margin: 0;
  }
</style>
