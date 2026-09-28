<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  type MonitorMetrics = {
    label: string;
    name: string;
    x: number;
    y: number;
    width: number;
    height: number;
    scaleFactor: number;
    captureMs: number;
    prepareMs: number;
    loadMs: number;
    paintMs: number;
  };
  type CaptureMetrics = {
    session: string;
    captureTotalMs: number;
    emitMs: number;
    shownMs: number;
    monitors: MonitorMetrics[];
  };

  let metrics = $state<CaptureMetrics | null>(null);
  const duration = (value: number) => `${value.toFixed(1)} ms`;

  onMount(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void listen<CaptureMetrics>("capture:metrics", (event) => {
      metrics = event.payload;
    }).then((dispose) => {
      if (disposed) dispose();
      else unlisten = dispose;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

{#if metrics}
  <div class="summary">
    <div>
      <span>Capture + prepare</span>
      <strong>{duration(metrics.captureTotalMs)}</strong>
    </div>
    <div>
      <span>Frame event</span>
      <strong>{duration(metrics.emitMs)}</strong>
    </div>
    <div>
      <span>Overlay shown</span>
      <strong>{duration(metrics.shownMs)}</strong>
    </div>
  </div>
  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>Monitor</th><th>Bounds</th><th>Scale</th><th>Capture</th>
          <th>Prepare</th><th>Load</th><th>Paint</th>
        </tr>
      </thead>
      <tbody>
        {#each metrics.monitors as monitor (monitor.label)}
          <tr>
            <td>{monitor.name}<small>{monitor.label}</small></td>
            <td>{monitor.x}, {monitor.y} · {monitor.width} × {monitor.height}</td>
            <td>{monitor.scaleFactor.toFixed(2)}×</td>
            <td>{duration(monitor.captureMs)}</td>
            <td>{duration(monitor.prepareMs)}</td>
            <td>{duration(monitor.loadMs)}</td>
            <td>{duration(monitor.paintMs)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{:else}
  <p class="empty">Capture telemetry appears after the first screenshot.</p>
{/if}

<style>
  .summary {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    margin: 12px 0;
  }
  .summary div {
    display: grid;
    gap: 6px;
    padding: 10px;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
  }
  .summary span {
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .summary strong {
    color: var(--color-text-strong);
    font: 12px var(--font-mono);
  }
  .table-wrap {
    overflow: auto;
    max-height: 220px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    white-space: nowrap;
    text-align: left;
  }
  th, td {
    padding: 8px 6px;
    border-top: 1px solid var(--color-divider);
    color: var(--color-text-variant);
    font: 10px var(--font-mono);
  }
  th {
    color: var(--color-text-muted);
    text-transform: uppercase;
  }
  td small {
    display: block;
    margin-top: 3px;
    color: var(--color-text-muted);
  }
  .empty {
    color: var(--color-text-muted);
    font-size: 12px;
  }
  @media (max-width: 600px) {
    .summary {
      grid-template-columns: 1fr;
    }
  }
</style>
