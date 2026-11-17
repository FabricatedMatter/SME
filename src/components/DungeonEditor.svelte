<script lang="ts">
  import { api } from "../lib/api";
  import { editor, joinRel, partFiles } from "../lib/editor.svelte";
  import { game } from "../lib/game.svelte";
  import { MapDoc } from "../lib/mapdoc.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import { newRule } from "../lib/spawn";

  let { path }: { path: string } = $props();
  const d = $derived(editor.files[path]);
  const meta = $derived(d?.metadata ?? {});
  const parts = $derived<any[]>(d?.parts ?? []);
  const touch = () => editor.markDirty(path);

  const RULES: { key: string; label: string; arg?: "number" | "parts"; help: string }[] = [
    { key: "maxSpawnCount", label: "Max spawn count", arg: "number", help: "How often this part may appear in one dungeon." },
    { key: "doNotConnectToPart", label: "Don't connect to", arg: "parts", help: "Never attach this part to these parts." },
    { key: "doNotCombineWith", label: "Don't combine with", arg: "parts", help: "Never use together with these parts." },
    { key: "ignorePartMaximumRule", label: "Ignore part maximum", help: "Doesn't count towards maxParts." },
    { key: "allowOverdrawing", label: "Allow overdrawing", help: "Other parts may overwrite this part's tiles." },
    { key: "worldGenMustContainLiquid", label: "Must be in liquid", help: "Only spawn where there is ocean liquid." },
    { key: "worldGenMustNotContainLiquid", label: "Must not be in liquid", help: "Never spawn in ocean liquid." },
    { key: "worldGenMustContainSolidForeground", label: "Needs solid fg", help: "" },
    { key: "worldGenMustContainAirForeground", label: "Needs air fg", help: "" },
    { key: "worldGenMustContainSolidBackground", label: "Needs solid bg", help: "" },
    { key: "worldGenMustContainAirBackground", label: "Needs air bg", help: "" },
  ];

  function setMeta(k: string, v: any) {
    if (v === "" || v === undefined || v === null) delete d.metadata[k];
    else d.metadata[k] = v;
    touch();
  }

  function isAnchor(name: string) {
    return (meta.anchor ?? []).includes(name);
  }
  function setAnchor(name: string, on: boolean) {
    const a: string[] = [...(meta.anchor ?? [])].filter((x) => x !== name);
    if (on) a.push(name);
    d.metadata.anchor = a;
    touch();
  }

  function rule(part: any, key: string): any[] | undefined {
    return (part.rules ?? []).find((r: any) => r[0] === key);
  }
  function setRule(part: any, key: string, on: boolean, arg?: any) {
    part.rules = (part.rules ?? []).filter((r: any) => r[0] !== key);
    if (on) part.rules.push(arg === undefined ? [key] : [key, arg]);
    touch();
  }

  async function renamePart(part: any, name: string) {
    name = name.trim();
    if (!name || name === part.name) return;
    if (parts.some((p) => p.name === name)) {
      editor.toast(`A part named ${name} already exists`, "error");
      return;
    }
    const old = part.name;
    part.name = name;
    d.metadata.anchor = (meta.anchor ?? []).map((a: string) => (a === old ? name : a));
    for (const p of parts) for (const r of p.rules ?? []) if (Array.isArray(r[1]?.[0])) r[1][0] = r[1][0].map((x: string) => (x === old ? name : x));
    touch();
  }

  function addPart() {
    let i = parts.length + 1;
    while (parts.some((p) => p.name === `${meta.name}${i}`)) i++;
    const mpath = editor.addPart(path, `${meta.name}${i}`);
    void editor.openMap(mpath, path);
  }

  async function duplicatePart(part: any) {
    let i = 2;
    while (parts.some((p) => p.name === `${part.name}_${i}`)) i++;
    const name = `${part.name}_${i}`;
    const src = joinRel(path, partFiles(part)[0] ?? "");
    const file = `${name}.json`;
    const mpath = joinRel(path, file);
    try {
      const doc = editor.maps[src]?.doc ?? (await api.readMap(src)).doc;
      const md = new MapDoc(mpath, JSON.parse(JSON.stringify(doc)));
      md.dirty = true;
      editor.maps = { ...editor.maps, [mpath]: md };
      d.parts.push({ ...JSON.parse(JSON.stringify(part)), name, def: ["tmx", file] });
      if (isAnchor(part.name)) d.metadata.anchor = [...meta.anchor, name];
      touch();
    } catch (e) {
      editor.toast(String(e), "error");
    }
  }

  function deletePart(part: any) {
    if (!confirm(`Remove part "${part.name}"? Its map file will be deleted when you save.`)) return;
    editor.deletePart(path, part.name);
  }

  const spawnRules = $derived(editor.spawnRules.filter((r) => r.dungeons.includes(meta.name)));

  function addSpawn() {
    editor.spawnRules.push(newRule(meta.name));
    editor.spawnDirty = true;
    editor.view = { kind: "spawn" };
  }

  function deleteDungeon() {
    if (!confirm(`Delete the dungeon ${meta.name} and all its part maps when you save?`)) return;
    editor.deleteDungeon(path);
  }

  const nameClash = $derived(meta.name && game.index?.dungeons[meta.name]?.vanilla && game.index.dungeons[meta.name].file !== "/" + path);
</script>

{#if d}
  <div class="page">
    <section class="panel">
      <h2><PixelIcon name="map" scale={3} color="var(--gold)" /> {meta.name || "(unnamed)"} <span class="muted small">{path}</span></h2>
      <div class="form">
        <label>
          <span>Dungeon name <span class="muted">(unique ID used by spawn rules and /placedungeon)</span></span>
          <input value={meta.name ?? ""} onchange={(e) => setMeta("name", (e.currentTarget as HTMLInputElement).value.trim())} />
          {#if nameClash}<span class="bad small">This name is already used by a vanilla dungeon. The game will reject it.</span>{/if}
        </label>
        <label>
          <span>Display name <span class="muted">(optional)</span></span>
          <input value={meta.displayName ?? ""} onchange={(e) => setMeta("displayName", (e.currentTarget as HTMLInputElement).value)} />
        </label>
        <label>
          <span>Species</span>
          <input list="dl-species-d" value={meta.species ?? "generic"} onchange={(e) => setMeta("species", (e.currentTarget as HTMLInputElement).value)} />
          <datalist id="dl-species-d"><option value="generic"></option><option value="dummy"></option>{#each game.index?.species ?? [] as s}<option value={s}></option>{/each}</datalist>
        </label>
        <label>
          <span>Gravity <span class="muted">(blank = planet's)</span></span>
          <input type="number" value={meta.gravity ?? ""} onchange={(e) => setMeta("gravity", (e.currentTarget as HTMLInputElement).value === "" ? null : Number((e.currentTarget as HTMLInputElement).value))} />
        </label>
        <label>
          <span>Breathable</span>
          <select value={meta.breathable === undefined ? "" : String(meta.breathable)} onchange={(e) => {
            const v = (e.currentTarget as HTMLSelectElement).value;
            setMeta("breathable", v === "" ? null : v === "true");
          }}>
            <option value="">planet default</option><option value="true">yes</option><option value="false">no</option>
          </select>
        </label>
        <label>
          <span>Max parts</span>
          <input type="number" value={meta.maxParts ?? 100} onchange={(e) => setMeta("maxParts", Number((e.currentTarget as HTMLInputElement).value))} />
        </label>
        <label>
          <span>Max radius</span>
          <input type="number" value={meta.maxRadius ?? 1000000} onchange={(e) => setMeta("maxRadius", Number((e.currentTarget as HTMLInputElement).value))} />
        </label>
        <label>
          <span>Extend surface free space</span>
          <input type="number" value={meta.extendSurfaceFreeSpace ?? 0} onchange={(e) => setMeta("extendSurfaceFreeSpace", Number((e.currentTarget as HTMLInputElement).value) || null)} />
        </label>
        <label class="check">
          <input type="checkbox" checked={!!meta.protected} onchange={(e) => setMeta("protected", (e.currentTarget as HTMLInputElement).checked)} />
          <span>Protected <span class="muted">(players can't dig/break it)</span></span>
        </label>
      </div>
    </section>

    <section class="panel">
      <div class="bar">
        <h3>Parts</h3>
        <span class="muted small">A microdungeon picks one <b>anchor</b> part at random (weighted by chance) each time it spawns.</span>
        <div class="spacer"></div>
        <button class="gold small" onclick={addPart}><PixelIcon name="plus" scale={2} /> New part</button>
      </div>
      <table>
        <thead>
          <tr><th>Name</th><th title="Can start the dungeon">Anchor</th><th>Map</th><th title="Relative weight">Chance</th><th title="Threat level range">Threat</th><th>Rules</th><th></th></tr>
        </thead>
        <tbody>
          {#each parts as part (part)}
            {@const files = partFiles(part)}
            <tr>
              <td><input class="name" value={part.name} onchange={(e) => renamePart(part, (e.currentTarget as HTMLInputElement).value)} /></td>
              <td class="c"><input type="checkbox" checked={isAnchor(part.name)} onchange={(e) => setAnchor(part.name, (e.currentTarget as HTMLInputElement).checked)} /></td>
              <td>
                {#each files as f}
                  {@const mp = joinRel(path, f)}
                  <button class="small" onclick={() => editor.openMap(mp, path)} title="Edit {mp}">
                    <PixelIcon name="pencil" scale={2} /> {f}{editor.maps[mp]?.dirty ? " •" : ""}
                  </button>
                {/each}
                {#if part.def?.[0] === "image"}<span class="muted small">image part (not editable)</span>{/if}
              </td>
              <td><input class="num" type="number" step="0.1" min="0" value={part.chance ?? 1} onchange={(e) => {
                const v = Number((e.currentTarget as HTMLInputElement).value);
                if (v === 1) delete part.chance;
                else part.chance = v;
                touch();
              }} /></td>
              <td class="threat">
                <input class="num" type="number" placeholder="min" value={part.minimumThreatLevel ?? ""} onchange={(e) => {
                  const v = (e.currentTarget as HTMLInputElement).value;
                  if (v === "") delete part.minimumThreatLevel;
                  else part.minimumThreatLevel = Number(v);
                  touch();
                }} />–<input class="num" type="number" placeholder="max" value={part.maximumThreatLevel ?? ""} onchange={(e) => {
                  const v = (e.currentTarget as HTMLInputElement).value;
                  if (v === "") delete part.maximumThreatLevel;
                  else part.maximumThreatLevel = Number(v);
                  touch();
                }} />
              </td>
              <td class="rules">
                {#each RULES as r}
                  {@const cur = rule(part, r.key)}
                  {#if cur || !r.key.startsWith("worldGenMustContain")}
                    <span class="chip" class:on={!!cur} title={r.help}>
                      <input type="checkbox" checked={!!cur} onchange={(e) => setRule(part, r.key, (e.currentTarget as HTMLInputElement).checked, r.arg === "number" ? [1] : r.arg === "parts" ? [[]] : undefined)} />
                      {r.label}
                      {#if cur && r.arg === "number"}
                        <input class="num tiny" type="number" min="1" value={cur[1]?.[0] ?? 1} onchange={(e) => setRule(part, r.key, true, [Number((e.currentTarget as HTMLInputElement).value)])} />
                      {:else if cur && r.arg === "parts"}
                        <input class="tiny-text" placeholder="part names, comma-separated" value={(cur[1]?.[0] ?? []).join(", ")} onchange={(e) => setRule(part, r.key, true, [(e.currentTarget as HTMLInputElement).value.split(",").map((s) => s.trim()).filter(Boolean)])} />
                      {/if}
                    </span>
                  {/if}
                {/each}
              </td>
              <td class="acts">
                <button class="small" title="Duplicate part" onclick={() => duplicatePart(part)}><PixelIcon name="plus" scale={2} /></button>
                <button class="small red" title="Remove part" onclick={() => deletePart(part)}><PixelIcon name="trash" scale={2} /></button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>

    <section class="panel">
      <div class="bar">
        <h3>Where it spawns</h3>
        <div class="spacer"></div>
        <button class="small" onclick={addSpawn}><PixelIcon name="plus" scale={2} /> Add spawn rule</button>
        <button class="small" onclick={() => (editor.view = { kind: "spawn" })}><PixelIcon name="globe" scale={2} /> Spawn rules</button>
      </div>
      {#if spawnRules.length}
        <ul>
          {#each spawnRules as r}
            <li>{r.layer} · {r.mode} · {r.distribution ?? `random ${r.probability}`} · {r.biomes.length} biome(s): <span class="muted">{r.biomes.map((b) => b.split("/").pop()?.replace(".biome", "")).join(", ")}</span></li>
          {/each}
        </ul>
      {:else}
        <p class="warn">This dungeon doesn't spawn anywhere yet. Add a spawn rule to pick planets/biomes.</p>
      {/if}
      <p class="muted small">Test in game: install the mod (Test tab), then in chat type <code>/admin</code> and <code>/placedungeon {meta.name}</code>. It places the dungeon at your cursor, ignoring anchors.</p>
    </section>

    <div class="danger">
      <button class="red small" onclick={deleteDungeon}><PixelIcon name="trash" scale={2} /> Delete dungeon</button>
    </div>
  </div>
{/if}

<style>
  .page {
    padding: 14px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex: 1;
    min-height: 0;
  }
  h2,
  h3 {
    margin: 0 0 8px;
    font-weight: normal;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  h3 {
    margin: 0;
    color: var(--gold);
  }
  .form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 10px 16px;
  }
  .form label {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .form label.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
  }
  .spacer {
    flex: 1;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th {
    text-align: left;
    color: var(--muted);
    font-weight: normal;
    padding: 4px;
  }
  td {
    padding: 4px;
    border-top: 2px solid rgba(255, 255, 255, 0.06);
    vertical-align: top;
  }
  td.c {
    text-align: center;
  }
  .name {
    width: 150px;
  }
  .num {
    width: 64px;
  }
  .tiny {
    width: 48px;
    padding: 0 4px;
  }
  .tiny-text {
    width: 160px;
    padding: 0 4px;
  }
  .threat {
    white-space: nowrap;
  }
  .rules {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px;
    background: #1c1c1c;
    border: 2px solid #000;
    font-size: 13px;
    color: var(--muted);
  }
  .chip.on {
    color: var(--text);
    background: #2c2600;
  }
  .acts {
    white-space: nowrap;
  }
  .bad {
    color: var(--bad);
  }
  .warn {
    color: var(--warn);
  }
  .small {
    font-size: 13px;
  }
  ul {
    margin: 0;
    padding-left: 20px;
  }
  .danger {
    display: flex;
    justify-content: flex-end;
  }
</style>
