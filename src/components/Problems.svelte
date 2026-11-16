<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import PixelIcon from "../lib/PixelIcon.svelte";
  import type { Problem } from "../lib/types";

  let open = $state(false);
  const errors = $derived(editor.problems.filter((p) => p.severity === "error").length);
  const warnings = $derived(editor.problems.length - errors);

  function go(p: Problem) {
    if (p.file.endsWith(".json")) {
      const dungeon = editor.dungeonPaths.find((d) => p.file.startsWith(d.split("/").slice(0, -1).join("/")));
      void editor.openMap(p.file, dungeon).then(() => {
        if (p.x !== null && p.y !== null) editor.goto = { path: p.file, x: p.x, y: p.y, n: Date.now() };
      });
    } else if (p.file.endsWith(".dungeon")) editor.view = { kind: "dungeon", path: p.file };
    else if (p.file.endsWith(".treasurepools")) editor.view = { kind: "loot", path: p.file };
    else if (p.file.endsWith(".patch")) editor.view = { kind: "spawn" };
  }
</script>

<div class="problems" class:open>
  <div class="bar">
    <button class="small" onclick={() => (open = !open)}>
      <PixelIcon name="warn" scale={2} color={errors ? "var(--bad)" : warnings ? "var(--warn)" : "var(--good)"} />
      {errors} errors · {warnings} warnings
    </button>
    <button class="small" disabled={editor.checking} onclick={() => {
      open = true;
      void editor.check();
    }}><PixelIcon name="check" scale={2} /> {editor.checking ? "Checking…" : "Check mod"}</button>
    <span class="muted small">Catches problems that would otherwise only show up in starbound.log.</span>
  </div>
  {#if open}
    <div class="list">
      {#each editor.problems as p}
        <button class="p" onclick={() => go(p)}>
          <span class={p.severity}>{p.severity}</span>
          <span class="file muted">{p.file}{p.x !== null ? ` @ ${p.x},${p.y}` : ""}</span>
          <span class="msg">{p.message}</span>
        </button>
      {:else}
        <p class="muted pad">No problems found (or not checked yet).</p>
      {/each}
    </div>
  {/if}
</div>

<style>
  .problems {
    border-top: 2px solid #000;
    background: rgba(13, 13, 13, 0.92);
  }
  .bar {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 4px 8px;
  }
  .list {
    max-height: 200px;
    overflow-y: auto;
    padding: 0 8px 6px;
  }
  .p {
    display: grid;
    grid-template-columns: 70px minmax(120px, 280px) 1fr;
    gap: 10px;
    width: 100%;
    background: none;
    border: none;
    box-shadow: none;
    text-shadow: none;
    padding: 2px 4px;
    text-align: left;
    white-space: normal;
    justify-content: start;
  }
  .p:hover:not(:disabled) {
    background: #222;
    box-shadow: none;
  }
  .file {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .error {
    color: var(--bad);
  }
  .warning {
    color: var(--warn);
  }
  .info {
    color: var(--blue);
  }
  .small {
    font-size: 13px;
  }
  .pad {
    padding: 4px;
    margin: 0;
  }
</style>
