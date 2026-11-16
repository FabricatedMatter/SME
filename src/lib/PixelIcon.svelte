<script lang="ts" module>
  // Hand-drawn pixel icons. '#' = main colour, 'o' = shade, 'w' = highlight, '.' = empty.
  export const ICONS: Record<string, string[]> = {
    star: [
      "....#....",
      "...###...",
      "...#w#...",
      "#########",
      ".##www##.",
      "..#####..",
      "..##o##..",
      ".##...##.",
      ".#.....#.",
    ],
    note: [
      "....wwwww",
      "...######",
      "...#oooo#",
      "...#....#",
      "...#....#",
      ".###..###",
      "####.####",
      "#ww#.#ww#",
      ".##...##.",
    ],
    gear: [
      "...#.#...",
      ".#.###.#.",
      "..#####..",
      ".###o###.",
      "###o.o###",
      ".###o###.",
      "..#####..",
      ".#.###.#.",
      "...#.#...",
    ],
    folder: [
      ".........",
      "###......",
      "#ww#####.",
      "#wwwwwww#",
      "#########",
      "#o......#",
      "#o......#",
      "#oooooooo",
      ".........",
    ],
    play: [".#......", ".##.....", ".#w#....", ".#ww#...", ".#www#..", ".#ww#...", ".#w#....", ".##.....", ".#......"],
    stop: [".......", ".#####.", ".#www#.", ".#w..#.", ".#w..#.", ".#...#.", ".#####.", "......."],
    x: ["#.....#", "##...##", ".##.##.", "..###..", ".##.##.", "##...##", "#.....#"],
    plus: ["...#...", "...#...", "...#...", "#######", "...#...", "...#...", "...#..."],
    search: [
      ".####....",
      "#wo..#...",
      "#w....#..",
      "#.....#..",
      "#.....#..",
      ".#...#...",
      "..####o..",
      "......##.",
      ".......##",
    ],
    spark: [
      "....#....",
      "....#....",
      "...#w#...",
      "##ww#ww##",
      "...#w#...",
      "....#..#.",
      "....#.###",
      "........#",
      ".........",
    ],
    file: [".#####..", ".#ww#w#.", ".#w.####", ".#.....#", ".#.ooo.#", ".#.....#", ".#.ooo.#", ".#.....#", ".#######"],
    check: [".......#", "......##", "#....##.", "##..##..", ".####...", "..##...."],
    disk: [
      "########.",
      "#o#www#o#",
      "#o#w#w#o#",
      "#o#www#o#",
      "#ooooooo#",
      "#owwwwwo#",
      "#ow...wo#",
      "#owwwwwo#",
      "#########",
    ],
  };
</script>

<script lang="ts">
  let {
    name,
    scale = 2,
    color = "currentColor",
    title = "",
    flat = false,
  }: { name: keyof typeof ICONS | string; scale?: number; color?: string; title?: string; flat?: boolean } = $props();

  const grid = $derived(ICONS[name] ?? ICONS.star);
  const w = $derived(Math.max(...grid.map((r) => r.length)));
  const h = $derived(grid.length);
  const cells = $derived(
    grid.flatMap((row, y) =>
      [...row].map((ch, x) => ({ x, y, ch })).filter((c) => c.ch !== "."),
    ),
  );
</script>

<svg
  class="pixel-icon"
  width={w * scale}
  height={h * scale}
  viewBox="0 0 {w} {h}"
  shape-rendering="crispEdges"
  aria-hidden={title ? undefined : "true"}
  role={title ? "img" : undefined}
>
  {#if title}<title>{title}</title>{/if}
  {#each cells as c}
    <rect
      x={c.x}
      y={c.y}
      width="1"
      height="1"
      fill={c.ch === "w" && !flat ? "#ffffff" : color}
      opacity={flat ? 1 : c.ch === "o" ? 0.55 : c.ch === "w" ? 0.85 : 1}
    />
  {/each}
</svg>

<style>
  .pixel-icon {
    display: inline-block;
    vertical-align: middle;
    flex: none;
  }
</style>
