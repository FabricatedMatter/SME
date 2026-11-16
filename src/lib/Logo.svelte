<script lang="ts">
  // SME's logo: a little pixel hut (same art as the app icon, see scripts/make_icon.py).
  import logo from "./logo.json";

  let { scale = 2 }: { scale?: number } = $props();
  const pal = logo.palette as Record<string, string>;
  const cells = logo.art.flatMap((row, y) => [...row].map((ch, x) => ({ x, y, ch })).filter((c) => c.ch !== "."));
</script>

<svg class="logo" width={16 * scale} height={16 * scale} viewBox="0 0 16 16" shape-rendering="crispEdges" aria-hidden="true">
  {#each cells as c}<rect x={c.x} y={c.y} width="1" height="1" fill={pal[c.ch]} />{/each}
</svg>

<style>
  .logo {
    display: inline-block;
    vertical-align: middle;
    flex: none;
  }
</style>
