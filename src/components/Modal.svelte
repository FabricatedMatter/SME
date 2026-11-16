<script lang="ts">
  import type { Snippet } from "svelte";
  import Window from "../lib/Window.svelte";

  let {
    title,
    subtitle = "",
    icon = "star",
    onclose,
    children,
    footer,
    width = 520,
  }: { title: string; subtitle?: string; icon?: string; onclose: () => void; children: Snippet; footer?: Snippet; width?: number } = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="backdrop" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="box" style:width="{width}px">
    <Window {title} {subtitle} {icon} {onclose} {footer}>
      <div class="body">{@render children()}</div>
    </Window>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: grid;
    place-items: center;
    z-index: 50;
  }
  .box {
    max-width: calc(100vw - 32px);
    max-height: calc(100vh - 48px);
    display: flex;
  }
  .box > :global(.win) {
    flex: 1;
    max-height: calc(100vh - 48px);
  }
  .body {
    padding: 14px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
</style>
