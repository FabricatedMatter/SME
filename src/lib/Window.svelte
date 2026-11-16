<script lang="ts">
  import type { Snippet } from "svelte";
  import PixelIcon from "./PixelIcon.svelte";

  let {
    title,
    subtitle = "",
    icon = "star",
    iconColor = "var(--gold)",
    onclose,
    actions,
    footer,
    children,
    fill = false,
  }: {
    title: string;
    subtitle?: string;
    icon?: string;
    iconColor?: string;
    onclose?: () => void;
    actions?: Snippet;
    footer?: Snippet;
    children: Snippet;
    fill?: boolean;
  } = $props();
</script>

<!-- Starbound-style window: rounded header with icon/title/subtitle, dark body, footer, hard drop shadow. -->
<section class="win" class:fill>
  <header class="win-head">
    <div class="win-icon"><PixelIcon name={icon} scale={3} color={iconColor} /></div>
    <div class="win-titles">
      <div class="win-title">{title}</div>
      {#if subtitle}<div class="win-sub">{subtitle}</div>{/if}
    </div>
    {#if actions}<div class="win-actions">{@render actions()}</div>{/if}
    {#if onclose}
      <button class="win-close" onclick={onclose} title="Close"><PixelIcon name="x" scale={2} /></button>
    {/if}
  </header>
  <div class="win-body">{@render children()}</div>
  <footer class="win-foot">{#if footer}{@render footer()}{/if}</footer>
</section>

<style>
  .win {
    display: flex;
    flex-direction: column;
    min-height: 0;
    filter: drop-shadow(4px 4px 0 rgba(0, 0, 0, 0.45));
  }
  .win.fill {
    height: 100%;
  }
  .win-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px 6px 8px;
    min-height: 50px;
    background: var(--win-head);
    border-bottom: 2px solid #000;
    clip-path: polygon(
      6px 0,
      calc(100% - 6px) 0,
      calc(100% - 6px) 2px,
      calc(100% - 2px) 2px,
      calc(100% - 2px) 6px,
      100% 6px,
      100% 100%,
      0 100%,
      0 6px,
      2px 6px,
      2px 2px,
      6px 2px
    );
  }
  .win-icon {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    background: #000;
    box-shadow:
      inset 0 0 0 2px #000,
      inset 0 0 0 4px #3c3c3c;
  }
  .win-titles {
    flex: 1;
    min-width: 0;
    line-height: 1.15;
  }
  .win-title {
    color: #fff;
    font-size: var(--fs-title);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .win-sub {
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .win-actions {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .win-close {
    width: 26px;
    height: 26px;
    padding: 0;
    display: grid;
    place-items: center;
    color: #fff;
    background: var(--btn-red-fill);
    border: 2px solid #000;
    box-shadow:
      inset 0 2px 0 var(--btn-red-hi),
      inset 0 -2px 0 var(--btn-red-lo);
    align-self: flex-start;
  }
  .win-close:hover {
    background: #8a3a3a;
  }
  .win-body {
    flex: 1;
    min-height: 0;
    background: var(--win-body);
    border-left: 2px solid var(--win-edge);
    border-right: 2px solid #0a0a0a;
    display: flex;
    flex-direction: column;
  }
  .win-foot {
    min-height: 12px;
    background: var(--win-head);
    border-top: 2px solid #000;
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - 6px),
      calc(100% - 2px) calc(100% - 6px),
      calc(100% - 2px) calc(100% - 2px),
      calc(100% - 6px) calc(100% - 2px),
      calc(100% - 6px) 100%,
      6px 100%,
      6px calc(100% - 2px),
      2px calc(100% - 2px),
      2px calc(100% - 6px),
      0 calc(100% - 6px)
    );
    padding: 4px 10px;
    color: var(--muted);
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
