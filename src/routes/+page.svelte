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
    bmpMs: number;
    loadMs: number;
    decodeMs: number;
  };

  type CaptureMetrics = {
    session: string;
    captureTotalMs: number;
    emitMs: number;
    shownMs: number;
    monitors: MonitorMetrics[];
  };

  let metrics = $state<CaptureMetrics | null>(null);

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

  const duration = (value: number) => `${value.toFixed(1)} ms`;
</script>

<svelte:head>
  <title>KeepShot · Capture</title>
  <meta name="description" content="KeepShot multi-monitor capture spike" />
</svelte:head>

<main>
  <header class="topbar">
    <div class="brand-mark" aria-hidden="true">K</div>
    <div class="brand-copy">
      <span class="eyebrow">LOCAL SCREEN CAPTURE</span>
      <h1>KeepShot</h1>
    </div>
    <div class="status"><span></span> Ready</div>
  </header>

  <section class="hero">
    <div class="hero-copy">
      <p class="eyebrow">MULTI-MONITOR SPIKE</p>
      <h2>Capture your whole workspace.</h2>
      <p class="description">A frozen frame is prepared for every display inside one overlay, aligned to physical pixels.</p>
      <div class="shortcut-hint"><span>Press</span><kbd>Ctrl</kbd><b>+</b><kbd>Shift</kbd><b>+</b><kbd>X</kbd><span>to capture</span></div>
    </div>
    <div class="hero-art" aria-hidden="true">
      <div class="screen screen-back"></div>
      <div class="screen screen-front"><i></i><i></i><i></i><strong></strong></div>
      <div class="reticle">＋</div>
      <div class="art-chip">PIXEL ALIGNED</div>
    </div>
  </section>

  <section class="metrics-panel" aria-label="Latest capture metrics">
    <div class="section-heading">
      <div>
        <p class="eyebrow">CAPTURE TELEMETRY</p>
        <h2>Latest session</h2>
      </div>
      {#if metrics}
        <div class="session-id">SESSION <span>{metrics.session}</span></div>
      {:else}
        <div class="awaiting"><span></span> Waiting for first capture</div>
      {/if}
    </div>

    {#if metrics}
      <div class="summary-grid">
        <div class="summary-card"><span>Capture + BMP</span><strong>{duration(metrics.captureTotalMs)}</strong></div>
        <div class="summary-card"><span>Frame event emitted</span><strong>{duration(metrics.emitMs)}</strong></div>
        <div class="summary-card"><span>Overlay shown</span><strong>{duration(metrics.shownMs)}</strong></div>
      </div>
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>Monitor</th><th>Physical bounds</th><th>Scale</th><th>Capture</th><th>BMP</th><th>Load</th><th>Decode</th></tr>
          </thead>
          <tbody>
            {#each metrics.monitors as monitor (monitor.label)}
              <tr>
                <td><strong>{monitor.name}</strong><small>{monitor.label}</small></td>
                <td class="mono">{monitor.x}, {monitor.y} · {monitor.width} × {monitor.height}</td>
                <td class="mono">{monitor.scaleFactor.toFixed(2)}×</td>
                <td class="mono">{duration(monitor.captureMs)}</td>
                <td class="mono">{duration(monitor.bmpMs)}</td>
                <td class="mono">{duration(monitor.loadMs)}</td>
                <td class="mono">{duration(monitor.decodeMs)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <div class="empty-state">
        <div class="empty-icon">⌖</div>
        <strong>No capture yet</strong>
        <span>Press the shortcut to capture every connected display.</span>
      </div>
    {/if}
  </section>
  <footer><span>Frames stay on this device</span><span>ESC closes the capture overlay</span></footer>
</main>

<style>
  main {
    width: min(1040px, calc(100% - 64px));
    min-height: 100vh;
    margin: 0 auto;
    padding: 28px 0 22px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }

  .topbar,
  .section-heading,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .topbar {
    justify-content: flex-start;
    gap: 12px;
  }

  .brand-mark {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 11px;
    color: white;
    font-weight: 700;
    background: linear-gradient(145deg, #818cf8, var(--color-primary-hover));
    box-shadow: 0 6px 18px #6366f144;
  }

  .brand-copy h1 {
    margin: 2px 0 0;
    font-size: 15px;
    line-height: 18px;
  }

  .eyebrow {
    margin: 0;
    color: var(--color-text-muted);
    font: 600 10px/1.2 var(--font-mono);
    letter-spacing: 0.1em;
  }

  .status,
  .awaiting {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--color-text-variant);
    font-size: 12px;
  }

  .status {
    margin-left: auto;
    padding: 7px 11px;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-full);
    background: var(--color-surface-low);
  }

  .status span,
  .awaiting span {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-success);
    box-shadow: 0 0 10px #10b98188;
  }

  .hero {
    position: relative;
    display: flex;
    align-items: center;
    min-height: 244px;
    overflow: hidden;
    padding: 34px 40px;
    border: 1px solid var(--color-glass-rim);
    border-radius: 20px;
    background:
      radial-gradient(ellipse at 85% 30%, #6366f11e, transparent 42%),
      linear-gradient(120deg, #1b1b20, #17171c);
    box-shadow: var(--shadow-l1);
  }

  .hero-copy {
    z-index: 1;
    max-width: 510px;
  }

  .hero h2 {
    margin: 13px 0 8px;
    color: var(--color-text-strong);
    font-size: 28px;
    line-height: 1.2;
    letter-spacing: -0.03em;
  }

  .description {
    max-width: 420px;
    margin: 0;
    color: var(--color-text-muted);
    font-size: 13px;
    line-height: 1.6;
  }

  .shortcut-hint {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 20px;
    color: var(--color-text-variant);
    font-size: 12px;
  }

  kbd {
    min-width: 23px;
    padding: 4px 7px;
    border: 1px solid var(--color-glass-rim);
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.07);
    color: white;
    font: 600 10px var(--font-sans);
    text-align: center;
    box-shadow: inset 0 1px #ffffff12;
  }

  .shortcut-hint b {
    color: #71717a;
    font-weight: 400;
  }

  .hero-art {
    position: absolute;
    top: 25px;
    right: 42px;
    width: 330px;
    height: 194px;
    opacity: 0.9;
  }

  .screen {
    position: absolute;
    width: 210px;
    height: 134px;
    border: 1px solid #ffffff35;
    border-radius: 11px;
    background: linear-gradient(135deg, #272833, #1a1b22);
    box-shadow: 0 18px 30px #0005;
  }

  .screen-back {
    top: 5px;
    right: 9px;
    transform: rotate(7deg);
    background: linear-gradient(135deg, #272c3a, #1a1c25);
  }

  .screen-front {
    bottom: 6px;
    left: 9px;
    padding: 19px 15px;
    border-color: #818cf877;
    box-shadow: 0 0 0 1px #6366f125, 0 18px 35px #0007;
  }

  .screen-front i {
    display: block;
    width: 68%;
    height: 5px;
    margin-bottom: 8px;
    border-radius: 4px;
    background: #ffffff12;
  }

  .screen-front i:nth-child(2) {
    width: 90%;
  }

  .screen-front i:nth-child(3) {
    width: 52%;
  }

  .screen-front strong {
    display: block;
    width: 50px;
    height: 30px;
    margin-top: 14px;
    border: 1px solid #818cf8aa;
    border-radius: 3px;
    background: #6366f114;
  }

  .reticle {
    position: absolute;
    top: 76px;
    right: 93px;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 1px solid #38bdf899;
    border-radius: 50%;
    background: #10151bcc;
    color: #7dd3fc;
    font: 20px/1 var(--font-mono);
  }

  .art-chip {
    position: absolute;
    right: -2px;
    bottom: -1px;
    padding: 6px 8px;
    border: 1px solid var(--color-glass-rim);
    border-radius: 5px;
    background: var(--color-glass);
    color: #a5b4fc;
    font: 9px var(--font-mono);
    letter-spacing: 0.05em;
  }

  .metrics-panel {
    overflow: hidden;
    border: 1px solid var(--color-glass-ambient);
    border-radius: 16px;
    background: var(--color-surface-low);
    box-shadow: var(--shadow-l1);
  }

  .section-heading {
    min-height: 76px;
    padding: 17px 22px;
    border-bottom: 1px solid var(--color-divider);
  }

  .section-heading h2 {
    margin: 5px 0 0;
    color: var(--color-text-strong);
    font-size: 16px;
  }

  .session-id {
    color: var(--color-text-muted);
    font: 10px var(--font-mono);
  }

  .session-id span {
    margin-left: 5px;
    color: #a5b4fc;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    padding: 16px 20px 14px;
  }

  .summary-card {
    display: grid;
    gap: 7px;
    padding: 12px 14px;
    border: 1px solid var(--color-divider);
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.025);
  }

  .summary-card span {
    color: var(--color-text-muted);
    font-size: 11px;
  }

  .summary-card strong {
    color: white;
    font: 500 16px var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .table-wrap {
    overflow-x: auto;
    padding: 0 20px 16px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    white-space: nowrap;
  }

  th {
    padding: 10px 12px;
    color: var(--color-text-muted);
    font: 600 10px var(--font-mono);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  td {
    padding: 12px;
    border-top: 1px solid var(--color-divider);
    color: var(--color-text-variant);
    font-size: 11px;
  }

  td strong,
  td small {
    display: block;
  }

  td strong {
    color: var(--color-text);
    font-size: 12px;
    font-weight: 500;
  }

  td small {
    margin-top: 4px;
    color: var(--color-text-muted);
    font: 10px var(--font-mono);
  }

  .mono {
    color: #d4d4d8;
    font: 10px var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .empty-state {
    display: grid;
    justify-items: center;
    gap: 8px;
    padding: 38px 20px 42px;
    color: var(--color-text-muted);
    font-size: 12px;
  }

  .empty-state strong {
    color: var(--color-text-variant);
    font-size: 13px;
    font-weight: 500;
  }

  .empty-icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    margin-bottom: 3px;
    border: 1px solid #0ea5e944;
    border-radius: 10px;
    color: var(--color-precision);
    font-size: 19px;
  }

  footer {
    margin-top: auto;
    color: #71717a;
    font: 10px var(--font-mono);
  }

  @media (max-width: 760px) {
    main {
      width: calc(100% - 32px);
      padding-top: 18px;
      gap: 18px;
    }

    .hero {
      min-height: 230px;
      padding: 26px 22px;
    }

    .hero h2 {
      max-width: 350px;
      font-size: 24px;
    }

    .hero-art {
      right: -100px;
      opacity: 0.32;
    }

    .summary-grid {
      grid-template-columns: 1fr;
    }

    .section-heading {
      align-items: flex-start;
      flex-direction: column;
      gap: 10px;
    }

    footer {
      flex-wrap: wrap;
      gap: 12px;
    }
  }
</style>
