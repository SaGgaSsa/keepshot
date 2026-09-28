<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import Toolbar from "$lib/overlay/Toolbar.svelte";
  import {
    annotationHandle,
    createLayer,
    hitTest,
    isBox,
    isSegment,
    isText,
    moveLayer,
    resizeLayer,
    type AnnotationTool,
    type Layer,
    type ResizeHandle,
  } from "$lib/annotations/model";
  import { renderLayers } from "$lib/annotations/render";
  import { LayerHistory } from "$lib/annotations/history";
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
    mode: "draw" | "move" | "resize" | "annotate" | "moveLayer" | "resizeLayer";
    start: PhysicalPoint;
    origin: PhysicalRect | null;
    handle?: string;
    layerId?: string;
    layerOrigin?: Layer;
    tool?: AnnotationTool;
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
  let annotationCanvas = $state<HTMLCanvasElement | undefined>(undefined);
  let textEditor = $state<HTMLTextAreaElement | undefined>(undefined);
  let layers = $state<Layer[]>([]);
  let selectedLayerId = $state<string | null>(null);
  let activeTool = $state<AnnotationTool>("select");
  let activeColor = $state("#EF4444");
  let strokeWidth = $state(5);
  let editingTextId = $state<string | null>(null);
  let editingText = $state("");
  let editingPoint = $state<PhysicalPoint | null>(null);
  let layerPreview = $state<Layer | null>(null);
  let canUndo = $state(false);
  let canRedo = $state(false);
  const history = new LayerHistory();

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

  // WebView2 custom-protocol throughput is per request, so frames are fetched as parallel row bands.
  const FRAME_BAND_BYTES = 2 * 1024 * 1024;

  function frameBands(monitor: MonitorGeometry): [number, number][] {
    const rowsPerBand = Math.max(1, Math.floor(FRAME_BAND_BYTES / (monitor.width * 4)));
    const bands: [number, number][] = [];
    for (let start = 0; start < monitor.height; start += rowsPerBand) {
      bands.push([start, Math.min(monitor.height, start + rowsPerBand)]);
    }
    return bands;
  }

  function frameUrl(monitor: MonitorGeometry, startRow: number, endRow: number): string {
    if (!session) return "";
    const query = `s=${encodeURIComponent(session.session)}&y0=${startRow}&y1=${endRow}`;
    return `http://frame.localhost/${encodeURIComponent(monitor.label)}?${query}`;
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
    const width = physicalToCss(rect.width, dpr);
    const height = physicalToCss(rect.height, dpr);
    return `left:${point.x}px;top:${point.y}px;width:${width}px;height:${height}px`;
  }

  function receiveFrames(next: OverlaySession) {
    if (session?.session === next.session) return;
    const sequence = ++loadSequence;
    const arrivedAt = performance.now();
    loading = true;
    selection = null;
    layers = [];
    selectedLayerId = null;
    layerPreview = null;
    gesture = null;
    editingTextId = null;
    editingPoint = null;
    editingText = "";
    activeTool = "select";
    activeColor = "#EF4444";
    strokeWidth = 5;
    history.reset();
    syncHistoryFlags();
    previewRect = null;
    session = next;
    void waitForFrames(next, sequence, arrivedAt);
  }

  async function waitForFrames(next: OverlaySession, sequence: number, arrivedAt: number) {
    await tick();
    try {
      const timings = await Promise.all(
        next.monitors.map(async (monitor): Promise<ImageTiming> => {
          const selector = `canvas[data-monitor-label="${CSS.escape(monitor.label)}"]`;
          const canvas = document.querySelector<HTMLCanvasElement>(selector);
          if (!canvas) throw new Error(`Frame canvas for ${monitor.label} was not mounted`);
          // Frames arrive as raw opaque RGBA, so painting skips image decoding entirely.
          const context = canvas.getContext("2d", { alpha: false });
          if (!context) throw new Error(`Canvas 2D is unavailable for ${monitor.label}`);
          let loadedAt = arrivedAt;
          await Promise.all(
            frameBands(monitor).map(async ([startRow, endRow]) => {
              const response = await fetch(frameUrl(monitor, startRow, endRow));
              if (!response.ok) throw new Error(`Frame load failed for ${monitor.label}`);
              const buffer = await response.arrayBuffer();
              loadedAt = Math.max(loadedAt, performance.now());
              if (sequence !== loadSequence) throw new Error("Stale capture session");
              const rows = endRow - startRow;
              if (buffer.byteLength !== monitor.width * rows * 4) {
                throw new Error(`Frame band for ${monitor.label} has an unexpected size`);
              }
              const imageData = new ImageData(
                new Uint8ClampedArray(buffer),
                monitor.width,
                rows,
              );
              context.putImageData(imageData, 0, startRow);
            }),
          );
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
      console.error("Could not prepare capture frames", error);
      loading = false;
      showNotice(String(error));
    }
  }

  function showNotice(message: string) {
    notice = message;
    if (noticeTimeout) clearTimeout(noticeTimeout);
    noticeTimeout = setTimeout(() => (notice = ""), 2600);
  }

  function commitLayers(next: Layer[]) {
    history.push(layers);
    layers = next;
    syncHistoryFlags();
    drawAnnotations();
  }

  function syncHistoryFlags() { canUndo = history.canUndo; canRedo = history.canRedo; }

  function selectedLayer(): Layer | undefined { return layers.find((layer) => layer.id === selectedLayerId); }

  function drawAnnotations() {
    if (!annotationCanvas || !session) return;
    const dpr = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(window.innerWidth * dpr));
    const height = Math.max(1, Math.round(window.innerHeight * dpr));
    if (annotationCanvas.width !== width || annotationCanvas.height !== height) {
      annotationCanvas.width = width;
      annotationCanvas.height = height;
    }
    const ctx = annotationCanvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, width, height);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    if (selection) {
      const left = (selection.x - session.bounds.x) / dpr;
      const top = (selection.y - session.bounds.y) / dpr;
      ctx.save();
      ctx.beginPath();
      ctx.rect(left, top, selection.width / dpr, selection.height / dpr);
      ctx.clip();
      const visibleLayers = layerPreview ? [...layers, layerPreview] : layers;
      renderLayers(ctx, visibleLayers, {
        originX: session.bounds.x,
        originY: session.bounds.y,
        scale: 1 / dpr,
      });
      const selected = selectedLayer();
      if (selected) drawLayerHandles(ctx, selected, dpr);
      ctx.restore();
    }
  }

  function drawLayerHandles(ctx: CanvasRenderingContext2D, layer: Layer, dpr: number) {
    const originX = session?.bounds.x ?? 0;
    const originY = session?.bounds.y ?? 0;
    const toCanvasX = (value: number) => (value - originX) / dpr;
    const toCanvasY = (value: number) => (value - originY) / dpr;
    const anchors = isSegment(layer)
      ? [layer.start, layer.end]
      : isBox(layer)
        ? [
            { x: layer.x, y: layer.y },
            { x: layer.x + layer.width / 2, y: layer.y },
            { x: layer.x + layer.width, y: layer.y },
            { x: layer.x + layer.width, y: layer.y + layer.height / 2 },
            { x: layer.x + layer.width, y: layer.y + layer.height },
            { x: layer.x + layer.width / 2, y: layer.y + layer.height },
            { x: layer.x, y: layer.y + layer.height },
            { x: layer.x, y: layer.y + layer.height / 2 },
          ]
        : [];

    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.strokeStyle = "#6366F1";
    ctx.fillStyle = "#fff";
    ctx.lineWidth = 1.5;
    ctx.beginPath();

    if (isSegment(layer)) {
      ctx.moveTo(toCanvasX(layer.start.x), toCanvasY(layer.start.y));
      ctx.lineTo(toCanvasX(layer.end.x), toCanvasY(layer.end.y));
    } else if (isBox(layer)) {
      ctx.rect(
        toCanvasX(layer.x),
        toCanvasY(layer.y),
        layer.width / dpr,
        layer.height / dpr,
      );
    } else if (isText(layer)) {
      const textWidth = Math.max(12, layer.text.length * layer.fontSize * 0.65);
      ctx.rect(toCanvasX(layer.x), toCanvasY(layer.y), textWidth / dpr, layer.fontSize * 1.25 / dpr);
    }

    ctx.stroke();
    for (const point of anchors) {
      ctx.beginPath();
      ctx.arc(toCanvasX(point.x), toCanvasY(point.y), 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
    ctx.restore();
  }

  function scheduleRender() { requestAnimationFrame(drawAnnotations); }

  function constrainPoint(start: PhysicalPoint, point: PhysicalPoint, shift: boolean): PhysicalPoint {
    if (!shift) return point;
    const dx = point.x - start.x; const dy = point.y - start.y;
    const angle = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * Math.PI / 4;
    const length = Math.max(Math.abs(dx), Math.abs(dy));
    return {
      x: Math.round(start.x + Math.cos(angle) * length),
      y: Math.round(start.y + Math.sin(angle) * length),
    };
  }

  function appendLayerGesture(
    start: PhysicalPoint,
    end: PhysicalPoint,
    tool: AnnotationTool,
    shift: boolean,
  ): Layer | null {
    if (!session || distance(start, end) <= 3) return null;
    const snapped = tool === "line" || tool === "arrow"
      ? constrainPoint(start, end, shift)
      : end;
    let x = Math.min(start.x, snapped.x);
    let y = Math.min(start.y, snapped.y);
    let width = Math.abs(snapped.x - start.x);
    let height = Math.abs(snapped.y - start.y);

    if ((tool === "rect" || tool === "ellipse") && shift) {
      const side = Math.max(width, height);
      width = side;
      height = side;
      x = snapped.x < start.x ? start.x - side : start.x;
      y = snapped.y < start.y ? start.y - side : start.y;
    }

    const base = { color: activeColor, strokeWidth };
    if (tool === "arrow") {
      return createLayer({ ...base, type: "arrow", start, end: snapped });
    }
    if (tool === "line") {
      return createLayer({ ...base, type: "line", start, end: snapped });
    }
    if (tool === "rect") {
      return createLayer({
        ...base,
        type: "rect",
        x,
        y,
        width: Math.max(1, width),
        height: Math.max(1, height),
      });
    }
    if (tool === "ellipse") {
      return createLayer({
        ...base,
        type: "ellipse",
        x,
        y,
        width: Math.max(1, width),
        height: Math.max(1, height),
      });
    }
    return null;
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
    const regionHandle = handle && selection;
    if (activeTool !== "select" && !regionHandle && (!selection || !contains(selection, point))) return;
    const selected = selectedLayer();
    const selectedHandle = activeTool === "select" && selected && selection && contains(selection, point)
      ? annotationHandle(selected, point)
      : null;
    const hit = selectedHandle && selected
      ? selected
      : activeTool === "select" && selection && contains(selection, point)
        ? [...layers].reverse().find((layer) => hitTest(layer, point))
        : undefined;
    let mode: Gesture["mode"] = "draw";
    if (regionHandle) mode = "resize";
    else if (hit && activeTool === "select") {
      selectedLayerId = hit.id;
      activeColor = hit.color;
      strokeWidth = hit.strokeWidth;
      const annotationResize = selectedHandle && selected?.id === hit.id
        ? selectedHandle
        : annotationHandle(hit, point);
      if (isText(hit) && event.detail >= 2) {
        beginTextEdit(hit);
        return;
      }
      mode = annotationResize ? "resizeLayer" : "moveLayer";
      gesture = {
        pointerId: event.pointerId,
        mode,
        start: point,
        origin: selection,
        layerId: hit.id,
        layerOrigin: $state.snapshot(hit),
        handle: annotationResize ?? undefined,
      };
      rootElement.setPointerCapture(event.pointerId);
      scheduleRender();
      return;
    } else if (activeTool !== "select" && selection && contains(selection, point)) mode = "annotate";
    else if (selection && contains(selection, point)) { selectedLayerId = null; mode = "move"; }
    else { selectedLayerId = null; }
    gesture = { pointerId: event.pointerId, mode, start: point, origin: selection, handle, tool: activeTool };
    rootElement.setPointerCapture(event.pointerId);
    if (mode === "draw") {
      selection = null;
      previewRect = { x: point.x, y: point.y, width: 1, height: 1 };
    } else if (mode === "annotate" && activeTool === "text") {
      gesture = null;
      beginTextEdit(null, point);
      event.preventDefault();
    }
  }

  function beginTextEdit(layer: Layer | null, point?: PhysicalPoint) {
    if (!session) return;
    const textLayer = layer && isText(layer) ? layer : null;
    editingTextId = textLayer?.id ?? null;
    if (!layer) selectedLayerId = null;
    editingText = textLayer?.text ?? "";
    editingPoint = textLayer ? { x: textLayer.x, y: textLayer.y } : point ?? null;
    void tick().then(() => textEditor?.focus());
  }

  function finishTextEdit(cancel = false) {
    const id = editingTextId; const point = editingPoint; const value = editingText;
    editingTextId = null; editingPoint = null; editingText = "";
    if (cancel || !point || !value.trim()) {
      if (id && !cancel && !value.trim()) commitLayers(layers.filter((layer) => layer.id !== id));
      return;
    }
    const oldLayer = id ? layers.find((item) => item.id === id) : undefined;
    const oldTextLayer = oldLayer && isText(oldLayer) ? oldLayer : null;
    const fontSize = oldTextLayer?.fontSize
      ?? (strokeWidth === 3 ? 16 : strokeWidth === 5 ? 24 : 36);
    const textLayer = createLayer({
      type: "text",
      color: oldLayer?.color ?? activeColor,
      strokeWidth: oldLayer?.strokeWidth ?? strokeWidth,
      x: point.x,
      y: point.y,
      text: value,
      fontSize,
    });
    const savedLayer = id ? { ...textLayer, id } : textLayer;
    if (isText(savedLayer)) {
      if (id) commitLayers(layers.map((item) => item.id === id ? savedLayer : item));
      else commitLayers([...layers, savedLayer]);
    } else {
      commitLayers([...layers, savedLayer]);
    }
    selectedLayerId = savedLayer.id;
  }

  function onPointerMove(event: PointerEvent) {
    if (!session) return;
    const point = pointerPoint(event);
    const currentGesture = gesture;
    if (!currentGesture) {
      const target = event.target instanceof HTMLElement ? event.target : null;
      if (!target?.dataset.handle) {
        rootElement.style.cursor = selection && contains(selection, point) ? "move" : "crosshair";
      }
      return;
    }
    if (event.pointerId !== currentGesture.pointerId
      || (!currentGesture.origin && currentGesture.mode !== "draw")) return;
    const dx = point.x - currentGesture.start.x;
    const dy = point.y - currentGesture.start.y;
    if (currentGesture.mode === "draw") {
      previewRect = clampRect(normalizeRect(currentGesture.start, point));
    } else if (currentGesture.mode === "move" && currentGesture.origin) {
      const bounds = session.bounds;
      selection = {
        ...currentGesture.origin,
        x: clamp(
          currentGesture.origin.x + dx,
          bounds.x,
          bounds.x + bounds.width - currentGesture.origin.width,
        ),
        y: clamp(
          currentGesture.origin.y + dy,
          bounds.y,
          bounds.y + bounds.height - currentGesture.origin.height,
        ),
      };
    } else if (currentGesture.mode === "resize" && currentGesture.origin && currentGesture.handle) {
      selection = resizeRect(currentGesture.origin, currentGesture.handle, dx, dy, session.bounds);
    } else if (currentGesture.mode === "annotate" && currentGesture.tool) {
      layerPreview = appendLayerGesture(currentGesture.start, point, currentGesture.tool, event.shiftKey);
      scheduleRender();
    } else if ((currentGesture.mode === "moveLayer" || currentGesture.mode === "resizeLayer")
      && currentGesture.layerOrigin && currentGesture.layerId) {
      const updated = currentGesture.mode === "moveLayer"
        ? moveLayer(currentGesture.layerOrigin, dx, dy)
        : resizeLayer(currentGesture.layerOrigin, currentGesture.handle as ResizeHandle, dx, dy);
      layers = layers.map((layer) => layer.id === currentGesture.layerId ? updated : layer);
      scheduleRender();
    }
    scheduleRender();
  }

  function onPointerUp(event: PointerEvent) {
    const currentGesture = gesture;
    if (!currentGesture || event.pointerId !== currentGesture.pointerId || !session) return;
    const point = pointerPoint(event);
    if (currentGesture.mode === "draw") {
      if (distance(currentGesture.start, point) < 4) {
        selection = fullMonitorRect(point);
      } else {
        selection = clampRect(normalizeRect(currentGesture.start, point));
      }
    } else if (currentGesture.mode === "annotate" && currentGesture.tool) {
      const created = appendLayerGesture(currentGesture.start, point, currentGesture.tool, event.shiftKey);
      if (created) { commitLayers([...layers, created]); selectedLayerId = created.id; }
      layerPreview = null;
    } else if ((currentGesture.mode === "moveLayer" || currentGesture.mode === "resizeLayer")
      && currentGesture.layerOrigin) {
      if (distance(currentGesture.start, point) > 0) {
        const previousLayer = currentGesture.layerOrigin;
        history.push(layers.map((layer) => layer.id === previousLayer.id ? previousLayer : layer));
        syncHistoryFlags();
      }
    }
    gesture = null;
    previewRect = null;
    layerPreview = null; scheduleRender();
  }

  function onPointerCancel(event: PointerEvent) {
    const currentGesture = gesture;
    if (!currentGesture || currentGesture.pointerId !== event.pointerId) return;
    if (currentGesture.mode === "draw") {
      selection = null;
      previewRect = null;
    }
    if ((currentGesture.mode === "moveLayer" || currentGesture.mode === "resizeLayer")
      && currentGesture.layerOrigin) {
      const previousLayer = currentGesture.layerOrigin;
      layers = layers.map((layer) => layer.id === previousLayer.id ? previousLayer : layer);
    }
    layerPreview = null;
    gesture = null;
    scheduleRender();
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
      : {
          x: session.bounds.x,
          y: session.bounds.y,
          width: session.bounds.width,
          height: session.bounds.height,
        };
  }

  function onKeyDown(event: KeyboardEvent) {
    if (
      event.key === "Enter"
      && event.target instanceof HTMLElement
      && event.target.closest(".toolbar")
    ) return;
    if (editingTextId !== null || editingPoint) {
      if (event.key === "Escape") { event.preventDefault(); finishTextEdit(true); }
      else if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); finishTextEdit(); }
      return;
    }
    const key = event.key.toLowerCase();
    if (!(event.ctrlKey || event.metaKey) && !event.altKey && !event.repeat) {
      const tools: Record<string, AnnotationTool> = {
        v: "select",
        a: "arrow",
        l: "line",
        r: "rect",
        e: "ellipse",
        t: "text",
      };
      const tool = tools[key];
      if (tool) {
        activeTool = tool;
        return;
      }
    }
    if ((event.ctrlKey || event.metaKey) && key === "z") {
      event.preventDefault();
      if (event.shiftKey) redo();
      else undo();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && key === "y") {
      event.preventDefault();
      redo();
      return;
    }
    if ((key === "delete" || key === "backspace") && selectedLayerId) {
      event.preventDefault();
      commitLayers(layers.filter((layer) => layer.id !== selectedLayerId));
      selectedLayerId = null;
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      if (selectedLayerId) { selectedLayerId = null; scheduleRender(); } else void closeCapture();
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

  function undo() {
    const previous = history.undo(layers);
    syncHistoryFlags();
    if (previous) {
      layers = previous;
      selectedLayerId = null;
      scheduleRender();
    }
  }

  function redo() {
    const next = history.redo(layers);
    syncHistoryFlags();
    if (next) {
      layers = next;
      selectedLayerId = null;
      scheduleRender();
    }
  }

  function changeColor(color: string) { activeColor = color; if (selectedLayerId) updateSelected({ color }); }
  function changeWidth(width: number) {
    strokeWidth = width;
    if (selectedLayerId) updateSelected({ strokeWidth: width });
  }
  function updateSelected(change: { color?: string; strokeWidth?: number }) {
    const selected = selectedLayer();
    if (!selected) return;
    const updated = updateLayerStyle(selected, change);
    commitLayers(layers.map((layer) => layer.id === selected.id ? updated : layer));
  }

  function updateLayerStyle(
    layer: Layer,
    change: { color?: string; strokeWidth?: number },
  ): Layer {
    const color = change.color ?? layer.color;
    const nextWidth = change.strokeWidth ?? layer.strokeWidth;
    switch (layer.type) {
      case "arrow":
        return {
          id: layer.id,
          type: "arrow",
          color,
          strokeWidth: nextWidth,
          start: layer.start,
          end: layer.end,
        };
      case "line":
        return {
          id: layer.id,
          type: "line",
          color,
          strokeWidth: nextWidth,
          start: layer.start,
          end: layer.end,
        };
      case "rect":
        return {
          id: layer.id,
          type: "rect",
          color,
          strokeWidth: nextWidth,
          x: layer.x,
          y: layer.y,
          width: layer.width,
          height: layer.height,
        };
      case "ellipse":
        return {
          id: layer.id,
          type: "ellipse",
          color,
          strokeWidth: nextWidth,
          x: layer.x,
          y: layer.y,
          width: layer.width,
          height: layer.height,
        };
      case "text":
        return {
          id: layer.id,
          type: "text",
          color,
          strokeWidth: nextWidth,
          x: layer.x,
          y: layer.y,
          text: layer.text,
          fontSize: change.strokeWidth === undefined ? layer.fontSize : fontSizeForWidth(nextWidth),
        };
    }
  }

  function fontSizeForWidth(width: number): number {
    if (width === 3) return 16;
    if (width === 5) return 24;
    return 36;
  }

  function textEditorStyle(point: PhysicalPoint): string {
    const cssPoint = session
      ? physicalPointToCss(point, session.bounds, window.devicePixelRatio || 1)
      : { x: 0, y: 0 };
    const selected = selectedLayer();
    const fontSize = isText(selected) ? selected.fontSize : fontSizeForWidth(strokeWidth);
    const color = isText(selected) ? selected.color : activeColor;
    const dpr = window.devicePixelRatio || 1;
    const lines = editingText.split("\n");
    const width = Math.max(100, ...lines.map((line) => line.length * fontSize * 0.65 / dpr));
    const height = Math.max(30, lines.length * fontSize * 1.25 / dpr);
    return [
      `left:${cssPoint.x}px;`,
      `top:${cssPoint.y}px;`,
      `width:${width}px;`,
      `height:${height}px;`,
      `color:${color};`,
      `font-size:${fontSize / dpr}px;`,
    ].join("");
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
      const layerBytes = layers.length ? rasterizeAnnotations(selection) : new Uint8Array(0);
      await invoke(command, layerBytes, { headers: { "x-keepshot-rect": JSON.stringify(selection) } });
    } catch (error) {
      if (session?.session === actionSession) showNotice(String(error));
    } finally {
      if (session?.session === actionSession) busy = false;
    }
  }

  function rasterizeAnnotations(rect: PhysicalRect): Uint8Array {
    const canvas = document.createElement("canvas");
    canvas.width = rect.width;
    canvas.height = rect.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Canvas 2D is unavailable");
    renderLayers(ctx, layers, {
      originX: rect.x,
      originY: rect.y,
      scale: 1,
    });
    const pixels = ctx.getImageData(0, 0, rect.width, rect.height).data;
    return new Uint8Array(pixels.buffer);
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
    return point.x >= rect.x
      && point.x < rect.x + rect.width
      && point.y >= rect.y
      && point.y < rect.y + rect.height;
  }

  function distance(left: PhysicalPoint, right: PhysicalPoint): number {
    return Math.hypot(left.x - right.x, left.y - right.y);
  }

  function resizeRect(
    origin: PhysicalRect,
    handle: string,
    dx: number,
    dy: number,
    bounds: VirtualBounds,
  ): PhysicalRect {
    let left = origin.x;
    let top = origin.y;
    let right = origin.x + origin.width;
    let bottom = origin.y + origin.height;
    if (handle.includes("w")) left = clamp(origin.x + dx, bounds.x, right - 1);
    if (handle.includes("e")) right = clamp(origin.x + origin.width + dx, left + 1, bounds.x + bounds.width);
    if (handle.includes("n")) top = clamp(origin.y + dy, bounds.y, bottom - 1);
    if (handle.includes("s")) {
      bottom = clamp(origin.y + origin.height + dy, top + 1, bounds.y + bounds.height);
    }
    return { x: left, y: top, width: right - left, height: bottom - top };
  }

  onMount(() => {
    const win = getCurrentWebviewWindow();
    let unlistenFrame: (() => void) | undefined;
    let unlistenClear: (() => void) | undefined;
    let disposed = false;
    void (async () => {
      unlistenFrame = await win.listen<OverlaySession>("overlay:frame", (event) => {
        receiveFrames(event.payload);
      });
      if (disposed) {
        unlistenFrame();
        return;
      }
      unlistenClear = await win.listen("overlay:clear", () => {
        loadSequence += 1;
        session = null;
        selection = null;
        layers = [];
        selectedLayerId = null;
        layerPreview = null;
        editingPoint = null;
        editingTextId = null;
        editingText = "";
        history.reset();
        syncHistoryFlags();
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
    window.addEventListener("resize", scheduleRender);
    return () => {
      disposed = true;
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("resize", scheduleRender);
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
    <canvas class="annotation-canvas" bind:this={annotationCanvas} aria-hidden="true"></canvas>
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
    {@const rectPosition = physicalPointToCss(
      { x: visibleRect.x, y: visibleRect.y },
      session.bounds,
      window.devicePixelRatio || 1,
    )}
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
    <Toolbar
      position={actionPosition}
      tool={activeTool}
      color={activeColor}
      strokeWidth={strokeWidth}
      {canUndo}
      {canRedo}
      {busy}
      {notice}
      onTool={(tool) => activeTool = tool}
      onColor={changeColor}
      onWidth={changeWidth}
      onUndo={undo}
      onRedo={redo}
      onCopy={() => runAction("copy_selection")}
      onSave={() => runAction("save_selection")}
      onClose={closeCapture}
    />
  {/if}

  {#if editingPoint && session}
    {@const editorStyle = textEditorStyle(editingPoint)}
    <textarea
      bind:this={textEditor}
      class="text-editor"
      value={editingText}
      oninput={(event) => editingText = event.currentTarget.value}
      onpointerdown={(event) => event.stopPropagation()}
      onpointerup={(event) => event.stopPropagation()}
      onkeydown={(event) => {
        event.stopPropagation();
        if (event.key === "Enter" && !event.shiftKey) {
          event.preventDefault();
          finishTextEdit();
        } else if (event.key === "Escape") {
          event.preventDefault();
          finishTextEdit(true);
        }
      }}
      onblur={() => { if (editingPoint) finishTextEdit(); }}
      style={editorStyle}
    ></textarea>
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

  .annotation-canvas {
    position: absolute;
    z-index: 1;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
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

  .text-editor {
    position: absolute;
    z-index: 4;
    min-width: 100px;
    min-height: 1.35em;
    width: max-content;
    overflow: hidden;
    padding: 0;
    border: 0;
    outline: 1px solid rgba(99,102,241,.65);
    background: transparent;
    font-family: Inter, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
    font-weight: 600;
    line-height: 1.25;
    resize: none;
    user-select: text;
    white-space: pre;
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
