<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import {
    actionBarPosition,
    clamp,
    cssPointToPhysical,
    monitorUnderPoint,
    nearestMonitor,
    normalizeRect,
    physicalPointToCss,
    physicalToCss,
    type MonitorGeometry,
    type PhysicalPoint,
    type PhysicalRect,
    type VirtualBounds,
  } from "$lib/overlay/geometry";

  type OverlaySession = {
    session: string;
    bounds: VirtualBounds;
    monitors: MonitorGeometry[];
  };
  type ImageTiming = { label: string; loadMs: number; paintMs: number };
  type Gesture = {
    pointerId: number;
    mode: "draw" | "move" | "resize";
    start: PhysicalPoint;
    origin: PhysicalRect | null;
    handle?: string;
  };

  const handleNames = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
  let session = $state<OverlaySession | null>(null);
  let selection = $state<PhysicalRect | null>(null);
  let previewRect = $state<PhysicalRect | null>(null);
  let gesture = $state<Gesture | null>(null);
  let loading = $state(false);
  let busy = $state(false);
  let notice = $state("");
  let rootElement: HTMLElement;
  let loadSequence = 0;
  let noticeTimeout: ReturnType<typeof setTimeout> | undefined;

  const visibleRect = $derived(gesture?.mode === "draw" ? previewRect : selection);
  const actionPosition = $derived.by(() => {
    if (!selection || !session) return null;
    return actionBarPosition(selection, session.monitors, session.bounds, window.devicePixelRatio || 1);
  });
  const dimmerPath = $derived.by(() => {
    const width = window.innerWidth;
    const height = window.innerHeight;
    if (!visibleRect || !session) return `M0 0 H${width} V${height} H0 Z`;
    const dpr = window.devicePixelRatio || 1;
    const topLeft = physicalPointToCss({ x: visibleRect.x, y: visibleRect.y }, session.bounds, dpr);
    const left = topLeft.x;
    const top = topLeft.y;
    const right = left + physicalToCss(visibleRect.width, dpr);
    const bottom = top + physicalToCss(visibleRect.height, dpr);
    return `M0 0 H${width} V${height} H0 Z M${left} ${top} H${right} V${bottom} H${left} Z`;
  });

  function frameUrl(monitor: MonitorGeometry): string {
    if (!session) return "";
    return `http://frame.localhost/${encodeURIComponent(monitor.label)}?s=${encodeURIComponent(session.session)}`;
  }

  function monitorStyle(monitor: MonitorGeometry): string {
    if (!session) return "";
    const dpr = window.devicePixelRatio || 1;
    const left = physicalToCss(monitor.x - session.bounds.x, dpr);
    const top = physicalToCss(monitor.y - session.bounds.y, dpr);
    const width = physicalToCss(monitor.width, dpr);
    const height = physicalToCss(monitor.height, dpr);
    return `left:${left}px;top:${top}px;width:${width}px;height:${height}px`;
  }

  function rectStyle(rect: PhysicalRect): string {
    if (!session) return "";
    const dpr = window.devicePixelRatio || 1;
    const point = physicalPointToCss({ x: rect.x, y: rect.y }, session.bounds, dpr);
    return `left:${point.x}px;top:${point.y}px;width:${physicalToCss(rect.width, dpr)}px;height:${physicalToCss(rect.height, dpr)}px`;
  }

  function receiveFrames(next: OverlaySession) {
    if (session?.session === next.session) return;
    const sequence = ++loadSequence;
    const arrivedAt = performance.now();
    loading = true;
    selection = null;
    previewRect = null;
    session = next;
    void waitForFrames(next, sequence, arrivedAt);
  }

  async function waitForFrames(next: OverlaySession, sequence: number, arrivedAt: number) {
    await tick();
    try {
      const timings = await Promise.all(
        next.monitors.map(async (monitor): Promise<ImageTiming> => {
          const canvas = document.querySelector<HTMLCanvasElement>(`canvas[data-monitor-label="${CSS.escape(monitor.label)}"]`);
          if (!canvas) throw new Error(`Frame canvas for ${monitor.label} was not mounted`);
          // Frames arrive as raw opaque RGBA, so painting skips image decoding entirely.
          const response = await fetch(frameUrl(monitor));
          if (!response.ok) throw new Error(`Frame load failed for ${monitor.label}`);
          const buffer = await response.arrayBuffer();
          const loadedAt = performance.now();
          if (sequence !== loadSequence) throw new Error("Stale capture session");
          const expectedBytes = monitor.width * monitor.height * 4;
          if (buffer.byteLength !== expectedBytes) {
            throw new Error(`Frame for ${monitor.label} has ${buffer.byteLength} bytes, expected ${expectedBytes}`);
          }
          const context = canvas.getContext("2d", { alpha: false });
          if (!context) throw new Error(`Canvas 2D is unavailable for ${monitor.label}`);
          context.putImageData(new ImageData(new Uint8ClampedArray(buffer), monitor.width, monitor.height), 0, 0);
          return {
            label: monitor.label,
            loadMs: loadedAt - arrivedAt,
            paintMs: performance.now() - arrivedAt,
          };
        }),
      );
      if (sequence !== loadSequence) return;
      await invoke("overlay_ready", { session: next.session, timings });
      if (sequence === loadSequence) loading = false;
    } catch (error) {
      if (sequence !== loadSequence) return;
      loading = false;
      showNotice(String(error));
    }
  }

  function showNotice(message: string) {
    notice = message;
    if (noticeTimeout) clearTimeout(noticeTimeout);
    noticeTimeout = setTimeout(() => (notice = ""), 2600);
  }

  function pointerPoint(event: PointerEvent): PhysicalPoint {
    if (!session) return { x: 0, y: 0 };
    return cssPointToPhysical(
      event.clientX,
      event.clientY,
      session.bounds,
      window.devicePixelRatio || 1,
    );
  }

  function onPointerDown(event: PointerEvent) {
    if (!session || event.button !== 0 || busy) return;
    const point = pointerPoint(event);
    const target = event.target instanceof HTMLElement ? event.target : null;
    const handle = target?.dataset.handle;
    let mode: Gesture["mode"] = "draw";
    if (handle && selection) mode = "resize";
    else if (selection && contains(selection, point)) mode = "move";
    gesture = { pointerId: event.pointerId, mode, start: point, origin: selection, handle };
    rootElement.setPointerCapture(event.pointerId);
    if (mode === "draw") {
      selection = null;
      previewRect = { x: point.x, y: point.y, width: 1, height: 1 };
    }
  }

  function onPointerMove(event: PointerEvent) {
    if (!session) return;
    const point = pointerPoint(event);
    if (!gesture) {
      const target = event.target instanceof HTMLElement ? event.target : null;
      if (!target?.dataset.handle) rootElement.style.cursor = selection && contains(selection, point) ? "move" : "crosshair";
      return;
    }
    if (event.pointerId !== gesture.pointerId || !gesture.origin && gesture.mode !== "draw") return;
    const dx = point.x - gesture.start.x;
    const dy = point.y - gesture.start.y;
    if (gesture.mode === "draw") {
      previewRect = clampRect(normalizeRect(gesture.start, point));
    } else if (gesture.mode === "move" && gesture.origin) {
      const bounds = session.bounds;
      selection = {
        ...gesture.origin,
        x: clamp(gesture.origin.x + dx, bounds.x, bounds.x + bounds.width - gesture.origin.width),
        y: clamp(gesture.origin.y + dy, bounds.y, bounds.y + bounds.height - gesture.origin.height),
      };
    } else if (gesture.mode === "resize" && gesture.origin && gesture.handle) {
      selection = resizeRect(gesture.origin, gesture.handle, dx, dy, session.bounds);
    }
  }

  function onPointerUp(event: PointerEvent) {
    if (!gesture || event.pointerId !== gesture.pointerId || !session) return;
    const point = pointerPoint(event);
    if (gesture.mode === "draw") {
      if (distance(gesture.start, point) < 4) {
        selection = fullMonitorRect(point);
      } else {
        selection = clampRect(normalizeRect(gesture.start, point));
      }
    }
    gesture = null;
    previewRect = null;
  }

  function onPointerCancel(event: PointerEvent) {
    if (gesture?.pointerId !== event.pointerId) return;
    if (gesture.mode === "draw") {
      selection = null;
      previewRect = null;
    }
    gesture = null;
  }

  function clampRect(rect: PhysicalRect): PhysicalRect {
    if (!session) return rect;
    const bounds = session.bounds;
    const x = clamp(rect.x, bounds.x, bounds.x + bounds.width - 1);
    const y = clamp(rect.y, bounds.y, bounds.y + bounds.height - 1);
    const right = clamp(rect.x + rect.width, x + 1, bounds.x + bounds.width);
    const bottom = clamp(rect.y + rect.height, y + 1, bounds.y + bounds.height);
    return { x, y, width: right - x, height: bottom - y };
  }

  function fullMonitorRect(point: PhysicalPoint): PhysicalRect {
    if (!session) return { x: point.x, y: point.y, width: 1, height: 1 };
    const monitor = monitorUnderPoint(point, session.monitors) ?? nearestMonitor(point, session.monitors);
    return monitor
      ? { x: monitor.x, y: monitor.y, width: monitor.width, height: monitor.height }
      : { x: session.bounds.x, y: session.bounds.y, width: session.bounds.width, height: session.bounds.height };
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      void closeCapture();
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c") {
      event.preventDefault();
      void runAction("copy_selection");
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void runAction("save_selection");
    } else if (event.key === "Enter" && !event.repeat && !busy) {
      event.preventDefault();
      if (selection) void runAction("copy_selection");
      else void selectCursorMonitor();
    }
  }

  async function selectCursorMonitor() {
    if (!session) return;
    try {
      const point = await invoke<PhysicalPoint>("cursor_position");
      selection = fullMonitorRect(point);
    } catch (error) {
      showNotice(String(error));
    }
  }

  async function runAction(command: "copy_selection" | "save_selection") {
    if (!selection || !session || busy || gesture) return;
    const actionSession = session.session;
    busy = true;
    notice = "";
    try {
      await invoke(command, { rect: selection });
    } catch (error) {
      if (session?.session === actionSession) showNotice(String(error));
    } finally {
      if (session?.session === actionSession) busy = false;
    }
  }

  async function closeCapture() {
    try {
      await invoke("close_overlays");
    } catch (error) {
      showNotice(String(error));
    }
  }

  function onContextMenu(event: MouseEvent) {
    event.preventDefault();
    void closeCapture();
  }

  function contains(rect: PhysicalRect, point: PhysicalPoint): boolean {
    return point.x >= rect.x && point.x < rect.x + rect.width && point.y >= rect.y && point.y < rect.y + rect.height;
  }

  function distance(left: PhysicalPoint, right: PhysicalPoint): number {
    return Math.hypot(left.x - right.x, left.y - right.y);
  }

  function resizeRect(origin: PhysicalRect, handle: string, dx: number, dy: number, bounds: VirtualBounds): PhysicalRect {
    let left = origin.x;
    let top = origin.y;
    let right = origin.x + origin.width;
    let bottom = origin.y + origin.height;
    if (handle.includes("w")) left = clamp(origin.x + dx, bounds.x, right - 1);
    if (handle.includes("e")) right = clamp(origin.x + origin.width + dx, left + 1, bounds.x + bounds.width);
    if (handle.includes("n")) top = clamp(origin.y + dy, bounds.y, bottom - 1);
    if (handle.includes("s")) bottom = clamp(origin.y + origin.height + dy, top + 1, bounds.y + bounds.height);
    return { x: left, y: top, width: right - left, height: bottom - top };
  }

  onMount(() => {
    const win = getCurrentWebviewWindow();
    let unlistenFrame: (() => void) | undefined;
    let unlistenClear: (() => void) | undefined;
    let disposed = false;
    void (async () => {
      unlistenFrame = await win.listen<OverlaySession>("overlay:frame", (event) => receiveFrames(event.payload));
      if (disposed) {
        unlistenFrame();
        return;
      }
      unlistenClear = await win.listen("overlay:clear", () => {
        loadSequence += 1;
        session = null;
        selection = null;
        previewRect = null;
        gesture = null;
        loading = false;
        busy = false;
        notice = "";
      });
      if (disposed) {
        unlistenFrame();
        unlistenClear();
        return;
      }
      const pending = await invoke<OverlaySession | null>("pending_frame");
      if (pending) receiveFrames(pending);
    })().catch((error) => showNotice(String(error)));
    window.addEventListener("keydown", onKeyDown);
    return () => {
      disposed = true;
      window.removeEventListener("keydown", onKeyDown);
      unlistenFrame?.();
      unlistenClear?.();
      if (noticeTimeout) clearTimeout(noticeTimeout);
    };
  });
</script>

<svelte:head>
  <title>KeepShot Capture Overlay</title>
</svelte:head>

<main
  bind:this={rootElement}
  role="application"
  tabindex="-1"
  aria-label="Capture region selection"
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerCancel}
  oncontextmenu={onContextMenu}
>
  {#if session}
    {#each session.monitors as monitor (monitor.label)}
      <canvas
        class="monitor-frame"
        data-monitor-label={monitor.label}
        width={monitor.width}
        height={monitor.height}
        style={monitorStyle(monitor)}
        aria-label="Frozen frame of {monitor.name}"
      ></canvas>
    {/each}
    <svg class="dimmer-svg" aria-hidden="true">
      <defs>
        <clipPath id="dimmer-cutout" clipPathUnits="userSpaceOnUse">
          <path d={dimmerPath} fill-rule="evenodd" clip-rule="evenodd" />
        </clipPath>
      </defs>
      <rect width="100%" height="100%" clip-path="url(#dimmer-cutout)" />
    </svg>
  {/if}

  {#if visibleRect && session}
    {@const rectPosition = physicalPointToCss({ x: visibleRect.x, y: visibleRect.y }, session.bounds, window.devicePixelRatio || 1)}
    <div class="selection-outline" style={rectStyle(visibleRect)}>
      {#if selection && !gesture}
        {#each handleNames as handle (handle)}
          <span class="handle handle-{handle}" data-handle={handle} aria-hidden="true"></span>
        {/each}
        <div class:inside={rectPosition.y < 30} class="dimension-chip">
          {visibleRect.width} × {visibleRect.height} px
        </div>
      {/if}
    </div>
  {/if}

  {#if selection && !gesture && actionPosition}
    <div
      class="action-bar"
      role="toolbar"
      aria-label="Capture actions"
      tabindex="-1"
      class:working={busy}
      style={`left:${actionPosition.x}px;top:${actionPosition.y}px;width:${actionPosition.width}px;height:${actionPosition.height}px`}
      onpointerdown={(event) => event.stopPropagation()}
      onpointerup={(event) => event.stopPropagation()}
    >
      {#if notice}
        <div class="action-notice" role="status">{notice}</div>
      {/if}
      <button class="action-button copy-button" disabled={busy} onclick={() => runAction("copy_selection")} aria-label="Copy selection">
        <svg viewBox="0 0 20 20" aria-hidden="true"><rect x="7" y="7" width="9" height="10" rx="1.5" /><path d="M12 4H5.5A1.5 1.5 0 0 0 4 5.5V13" /></svg>
        <span>Copy</span><kbd>Ctrl C</kbd>
      </button>
      <button class="action-button" disabled={busy} onclick={() => runAction("save_selection")} aria-label="Save selection">
        <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M4 3.5h10l2 2V16a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z" /><path d="M7 3.5v5h6v-5M7 17v-5h6v5" /></svg>
        <span>Save</span><kbd>Ctrl S</kbd>
      </button>
      <button class="action-button close-button" disabled={busy} onclick={closeCapture} aria-label="Close capture">
        <svg viewBox="0 0 20 20" aria-hidden="true"><path d="m5 5 10 10M15 5 5 15" /></svg>
        <span>Close</span><kbd>Esc</kbd>
      </button>
    </div>
  {/if}

  {#if loading}
    <div class="loading-indicator">Preparing frames…</div>
  {/if}
</main>

<style>
  :global(html),
  :global(body) {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    margin: 0;
    overflow: hidden;
    user-select: none;
    cursor: crosshair;
    background: #000;
  }

  main {
    position: fixed;
    inset: 0;
    overflow: hidden;
    background: #000;
    cursor: crosshair;
    touch-action: none;
  }

  .monitor-frame {
    position: absolute;
    display: block;
    max-width: none;
    margin: 0;
    object-fit: fill;
    pointer-events: none;
    user-select: none;
  }

  .dimmer-svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    pointer-events: none;
  }

  .dimmer-svg rect {
    fill: rgba(10, 12, 16, 0.45);
  }

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

  .action-bar {
    position: absolute;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 4px;
    width: 326px;
    height: 52px;
    padding: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: var(--radius-xl);
    background: var(--color-glass);
    backdrop-filter: blur(16px) saturate(180%);
    box-shadow: var(--shadow-l1);
    cursor: default;
  }

  .action-bar.working {
    opacity: 0.72;
  }

  .action-button {
    display: flex;
    flex: 1;
    min-width: 0;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 38px;
    padding: 0 8px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: rgba(255, 255, 255, 0.06);
    color: var(--color-text);
    font: 600 12px var(--font-sans);
    cursor: pointer;
  }

  .action-button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.13);
    color: white;
  }

  .action-button:disabled {
    cursor: wait;
  }

  .action-button svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 1.6;
  }

  .copy-button {
    border-color: rgba(99, 102, 241, 0.5);
    background: var(--color-primary);
    color: #fff;
  }

  .copy-button:hover:not(:disabled) {
    background: var(--color-primary-hover);
  }

  kbd {
    padding: 3px 4px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.7);
    font: 600 9px var(--font-sans);
    white-space: nowrap;
  }

  .close-button:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.15);
    color: #fca5a5;
  }

  .action-notice {
    position: absolute;
    right: 0;
    bottom: calc(100% + 7px);
    max-width: 326px;
    overflow: hidden;
    padding: 7px 10px;
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-radius: var(--radius-md);
    background: rgba(24, 24, 27, 0.94);
    color: #fecaca;
    font: 11px/1.4 var(--font-sans);
    text-overflow: ellipsis;
    white-space: nowrap;
    box-shadow: var(--shadow-l1);
  }

  .loading-indicator {
    position: absolute;
    right: 16px;
    bottom: 16px;
    padding: 7px 10px;
    border: 1px solid var(--color-glass-rim);
    border-radius: var(--radius-md);
    background: var(--color-glass);
    color: var(--color-text-muted);
    font: 11px var(--font-mono);
  }
</style>
