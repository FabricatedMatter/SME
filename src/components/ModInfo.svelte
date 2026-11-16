<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "../lib/api";
  import { editor } from "../lib/editor.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";

  const p = $derived(editor.project!);
  const metaPath = $derived(p.metadata_file);
  const meta = $derived(editor.files[metaPath] ?? null);

  const FIELDS: { k: string; label: string; help?: string; area?: boolean }[] = [
    { k: "name", label: "Internal name", help: "Unique ID of the mod (no spaces). Other mods use it in requires/includes." },
    { k: "friendlyName", label: "Display name" },
    { k: "author", label: "Author" },
    { k: "version", label: "Version" },
    { k: "description", label: "Description", area: true },
    { k: "link", label: "Link (e.g. Steam Workshop page)" },
    { k: "tags", label: "Tags", help: "Steam workshop tags, separated by |" },
  ];

  function setMeta(k: string, v: any) {
    const m = { ...(meta ?? {}) };
    if (v === "" || v === null) delete m[k];
    else m[k] = v;
    editor.setFile(metaPath, m);
  }

  let installState = $state<string | null>(null);
  let busy = $state(false);
  let log = $state("");
  async function refreshInstall() {
    try {
      installState = await api.installState();
    } catch {
      installState = null;
    }
  }
  $effect(() => {
    void p.root;
    void refreshInstall();
  });

  async function install(method: "symlink" | "copy" | "pak") {
    busy = true;
    try {
      if (editor.dirty && !(await editor.saveAll())) return;
      const msg = await api.installMod(method);
      editor.toast(msg, "good");
    } catch (e) {
      editor.toast(String(e), "error");
    } finally {
      busy = false;
      void refreshInstall();
    }
  }
  async function uninstall() {
    try {
      editor.toast(await api.uninstallMod());
    } catch (e) {
      editor.toast(String(e), "error");
    }
    void refreshInstall();
  }
  async function exportPak() {
    const out = await save({ title: "Export mod as .pak", defaultPath: `${meta?.name ?? "mod"}.pak`, filters: [{ name: "Starbound pak", extensions: ["pak"] }] });
    if (!out) return;
    if (editor.dirty && !(await editor.saveAll())) return;
    try {
      const n = await api.exportPak(out);
      editor.toast(`Packed ${n} files into ${out}`, "good");
    } catch (e) {
      editor.toast(String(e), "error");
    }
  }
  async function readLog() {
    try {
      const all = await api.starboundLog();
      const lines = all.split("\n");
      const interesting = lines.filter((l) => /\[(Error|Warn)\]|exception|dungeon|treasure/i.test(l));
      log = (interesting.length ? interesting : lines).slice(-80).join("\n");
    } catch (e) {
      log = String(e);
    }
  }
</script>

<div class="page">
  <section class="panel">
    <h2><PixelIcon name="star" scale={3} color="var(--gold)" /> {meta?.friendlyName ?? meta?.name ?? p.root.split("/").pop()}</h2>
    <p class="muted small">{p.root}</p>
    {#if meta === null}
      <p class="warn">This folder has no <code>_metadata</code> file, so the game won't know the mod's name.</p>
      <button class="gold" onclick={() => editor.setFile("_metadata", { name: p.root.split("/").pop(), friendlyName: p.root.split("/").pop(), author: "", version: "1.0", description: "", priority: 0, includes: [], requires: [] })}>Create _metadata</button>
    {:else}
      <div class="form">
        {#each FIELDS as f}
          <label class:wide={f.area}>
            <span>{f.label} {#if f.help}<span class="muted small">· {f.help}</span>{/if}</span>
            {#if f.area}
              <textarea rows="3" value={meta[f.k] ?? ""} onchange={(e) => setMeta(f.k, (e.currentTarget as HTMLTextAreaElement).value)}></textarea>
            {:else}
              <input value={meta[f.k] ?? ""} onchange={(e) => setMeta(f.k, (e.currentTarget as HTMLInputElement).value)} />
            {/if}
          </label>
        {/each}
        <label>
          <span>Load priority <span class="muted small">· higher loads later</span></span>
          <input type="number" value={meta.priority ?? 0} onchange={(e) => setMeta("priority", Number((e.currentTarget as HTMLInputElement).value))} />
        </label>
        <label>
          <span>Requires <span class="muted small">· mod names, comma-separated</span></span>
          <input value={(meta.requires ?? []).join(", ")} onchange={(e) => setMeta("requires", (e.currentTarget as HTMLInputElement).value.split(",").map((s) => s.trim()).filter(Boolean))} />
        </label>
        <label>
          <span>Includes <span class="muted small">· load after these mods if present</span></span>
          <input value={(meta.includes ?? []).join(", ")} onchange={(e) => setMeta("includes", (e.currentTarget as HTMLInputElement).value.split(",").map((s) => s.trim()).filter(Boolean))} />
        </label>
      </div>
    {/if}
  </section>

  <section class="panel">
    <h3><PixelIcon name="play" scale={2} color="var(--gold)" /> Test in game</h3>
    <p>
      Installed: {#if installState}<span class="good">{installState}</span>{:else}<span class="muted">no</span>{/if}
    </p>
    <div class="row">
      <button class="gold" disabled={busy} onclick={() => install("symlink")} title="Starbound reads your project folder directly; saves show up after restarting the game">Link into mods folder</button>
      <button disabled={busy} onclick={() => install("copy")}>Copy into mods folder</button>
      <button disabled={busy} onclick={() => install("pak")}>Pack .pak into mods folder</button>
      <button class="red" disabled={busy || !installState} onclick={uninstall}>Remove</button>
      <button onclick={() => api.modsFolder().then((f) => openPath(f))}><PixelIcon name="folder" scale={2} /> Open mods folder</button>
    </div>
    <ol class="steps">
      <li>Install (above) and start Starbound. Mods are only read at game start.</li>
      <li>Open chat and type <code>/admin</code>.</li>
      <li>Point at a spot and type <code>/placedungeon &lt;name&gt;</code>:
        {#each editor.dungeonNames as n}<code class="pick">/placedungeon {n}</code>{/each}
      </li>
      <li>To see natural spawning, visit a new planet whose biome is in your spawn rules.</li>
    </ol>
    <div class="row">
      <button onclick={readLog}>Show errors from starbound.log</button>
      <button onclick={exportPak}><PixelIcon name="disk" scale={2} /> Export .pak…</button>
    </div>
    {#if log}<pre>{log}</pre>{/if}
  </section>
</div>

<style>
  .page {
    padding: 14px;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h2,
  h3 {
    margin: 0 0 6px;
    font-weight: normal;
    display: flex;
    gap: 10px;
    align-items: center;
  }
  h3 {
    color: var(--gold);
  }
  .form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 10px 16px;
  }
  .form label {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .form label.wide {
    grid-column: 1 / -1;
  }
  .row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }
  .steps li {
    margin: 4px 0;
  }
  .pick {
    margin-left: 8px;
    user-select: all;
  }
  pre {
    background: #0a0a0a;
    font-family: monospace;
    font-size: 12px;
    padding: 8px;
    max-height: 320px;
    overflow: auto;
    white-space: pre-wrap;
  }
  .good {
    color: var(--good);
  }
  .warn {
    color: var(--warn);
  }
  .small {
    font-size: 13px;
  }
</style>
