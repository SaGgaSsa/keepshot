<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  type MonitorInfo = {
    label: string;
    name: string;
    x: number;
    y: number;
    width: number;
    height: number;
    scaleFactor: number;
  };
  type PendingFrame = { session: string; monitor: MonitorInfo };

  let monitor = $state<MonitorInfo | null>(null);
  let frameSrc = $state<string | null>(null);
  let imageElement = $state<HTMLImageElement>();
  let mismatch = $state(false);
  let loadSequence = 0;

  async function loadFrame(frame: PendingFrame) {
    const sequence = ++loadSequence;
    monitor = frame.monitor;
    frameSrc = `http://frame.localhost/${encodeURIComponent(frame.monitor.label)}?s=${encodeURIComponent(frame.session)}`;
    await tick();
    try {
      if (!imageElement) throw new Error("Frame image element is not mounted");
      await imageElement.decode();
      if (sequence !== loadSequence) return;
      const physicalWidth = window.innerWidth * window.devicePixelRatio;
      const physicalHeight = window.innerHeight * window.devicePixelRatio;
      mismatch = Math.abs(physicalWidth - frame.monitor.width) > 1 || Math.abs(physicalHeight - frame.monitor.height) > 1;
      await invoke("overlay_ready", { label: frame.monitor.label, session: frame.session });
    } catch (error) {
      if (sequence === loadSequence) frameSrc = null;
      console.error(`Could not load capture frame for ${frame.monitor.label}`, error);
    }
  }

  onMount(() => {
    const win = getCurrentWebviewWindow();
    let unlistenFrame: (() => void) | undefined;
    let unlistenClear: (() => void) | undefined;
    let disposed = false;
    const keyHandler = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void invoke("close_overlays");
      }
    };
    window.addEventListener("keydown", keyHandler);
    void (async () => {
      unlistenFrame = await win.listen<PendingFrame>("overlay:frame", (event) => void loadFrame(event.payload));
      if (disposed) {
        unlistenFrame();
        return;
      }
      unlistenClear = await win.listen("overlay:clear", () => {
        loadSequence += 1;
        frameSrc = null;
        monitor = null;
        mismatch = false;
      });
      if (disposed) {
        unlistenFrame();
        unlistenClear();
        return;
      }
      const pending = await invoke<PendingFrame | null>("pending_frame", { label: win.label });
      if (pending) await loadFrame(pending);
    })().catch((error) => console.error("Could not initialize capture overlay", error));
    return () => {
      disposed = true;
      window.removeEventListener("keydown", keyHandler);
      unlistenFrame?.();
      unlistenClear?.();
    };
  });
</script>

<svelte:head>
  <title>KeepShot Capture Overlay</title>
</svelte:head>

<main>
  {#if frameSrc}
    <img bind:this={imageElement} src={frameSrc} alt="Frozen monitor capture" draggable="false" />
  {/if}
  <div class="dimming"></div>
  {#if monitor}
    <aside class:misaligned={mismatch} aria-label="Overlay alignment diagnostics">
      <div class="hud-heading"><span class="dot"></span><strong>{monitor.label}</strong><span class="scale">{monitor.scaleFactor.toFixed(2)}×</span></div>
      <div class="monitor-name">{monitor.name}</div>
      <div class="hud-row"><span>PHYSICAL</span><b>{monitor.width} × {monitor.height}</b></div>
      <div class="hud-row"><span>WEBVIEW</span><b>{window.innerWidth} × {window.innerHeight} @ {window.devicePixelRatio.toFixed(2)} DPR</b></div>
      <div class="hud-row"><span>BACKEND SCALE</span><b>{monitor.scaleFactor.toFixed(2)}×</b></div>
      {#if mismatch}<div class="warning">DPI GEOMETRY MISMATCH</div>{/if}
    </aside>
  {/if}
</main>

<style>
  :global(html), :global(body) { width: 100%; height: 100%; min-width: 0; min-height: 0; margin: 0; overflow: hidden; user-select: none; cursor: crosshair; background: transparent; }
  main { position: fixed; inset: 0; overflow: hidden; background: transparent; }
  img { position: fixed; inset: 0; display: block; width: 100vw; height: 100vh; margin: 0; object-fit: fill; pointer-events: none; user-select: none; }
  .dimming { position: fixed; inset: 0; background: var(--color-dim-overlay); pointer-events: none; }
  aside { position: fixed; top: 16px; left: 16px; z-index: 2; min-width: 225px; padding: 11px 13px; border: 1px solid rgba(255,255,255,.1); border-radius: var(--radius-lg); background: var(--color-glass); backdrop-filter: blur(16px) saturate(180%); box-shadow: var(--shadow-l1); color: var(--color-text); font: 11px/1.4 var(--font-mono); font-variant-numeric: tabular-nums; }
  .hud-heading { display: flex; align-items: center; gap: 7px; color: var(--color-text-strong); }
  .hud-heading strong { font-weight: 600; }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--color-precision); box-shadow: 0 0 8px #0ea5e999; }
  .scale { margin-left: auto; color: #7dd3fc; }
  .monitor-name { overflow: hidden; margin: 4px 0 9px 13px; color: var(--color-text-muted); text-overflow: ellipsis; white-space: nowrap; }
  .hud-row { display: flex; justify-content: space-between; gap: 14px; padding-top: 5px; color: var(--color-text-muted); font-size: 10px; }
  .hud-row b { color: var(--color-text-variant); font-weight: 500; }
  .misaligned { border-color: rgba(239,68,68,.72); box-shadow: 0 0 0 1px rgba(239,68,68,.18), var(--shadow-l1); }
  .misaligned .dot { background: var(--color-danger); box-shadow: 0 0 8px #ef444499; }
  .warning { margin-top: 8px; padding-top: 7px; border-top: 1px solid rgba(239,68,68,.3); color: #fca5a5; font-size: 9px; letter-spacing: .06em; }
</style>
