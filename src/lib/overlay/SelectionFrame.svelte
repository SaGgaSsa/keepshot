<script lang="ts">
  type Props = {
    style: string;
    width: number;
    height: number;
    handles: readonly string[];
    editable: boolean;
    inside: boolean;
  };

  let { style, width, height, handles, editable, inside }: Props = $props();
</script>

<div class="selection-outline" {style}>
  {#if editable}
    {#each handles as handle (handle)}
      <span class="handle handle-{handle}" data-handle={handle} aria-hidden="true"></span>
    {/each}
    <div class:inside class="dimension-chip">
      {width} × {height} px
    </div>
  {/if}
</div>

<style>
  .selection-outline {
    position: absolute;
    z-index: 2;
    border: 1.5px solid var(--color-primary);
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.4), 0 0 16px rgba(99, 102, 241, 0.35);
    pointer-events: none;
  }

  .dimension-chip {
    position: absolute;
    top: -29px;
    left: -1px;
    padding: 4px 7px;
    border: 1px solid var(--color-glass-rim);
    border-radius: var(--radius-sm);
    background: var(--color-glass);
    box-shadow: var(--shadow-l1);
    color: #7dd3fc;
    font: 500 11px/16px var(--font-mono);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .dimension-chip.inside {
    top: 4px;
    left: 4px;
  }

  .handle {
    position: absolute;
    width: 8px;
    height: 8px;
    border: 1.5px solid var(--color-primary);
    border-radius: 2px;
    background: #fff;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.8);
    pointer-events: auto;
  }

  .handle-nw {
    top: 0;
    left: 0;
    transform: translate(-50%, -50%);
    cursor: nwse-resize;
  }

  .handle-n {
    top: 0;
    left: 50%;
    transform: translate(-50%, -50%);
    cursor: ns-resize;
  }

  .handle-ne {
    top: 0;
    right: 0;
    transform: translate(50%, -50%);
    cursor: nesw-resize;
  }

  .handle-e {
    top: 50%;
    right: 0;
    transform: translate(50%, -50%);
    cursor: ew-resize;
  }

  .handle-se {
    right: 0;
    bottom: 0;
    transform: translate(50%, 50%);
    cursor: nwse-resize;
  }

  .handle-s {
    bottom: 0;
    left: 50%;
    transform: translate(-50%, 50%);
    cursor: ns-resize;
  }

  .handle-sw {
    bottom: 0;
    left: 0;
    transform: translate(-50%, 50%);
    cursor: nesw-resize;
  }

  .handle-w {
    top: 50%;
    left: 0;
    transform: translate(-50%, -50%);
    cursor: ew-resize;
  }
</style>
