<script lang="ts">
  import { reportError } from "$lib/reportError";
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import Toolbar from "$lib/overlay/Toolbar.svelte";
  import SelectionFrame from "$lib/overlay/SelectionFrame.svelte";
  import {
    annotationHandle,
    arrowCurveHandlePoint,
    createLayer,
    curveControlForMidpoint,
    hitTest,
    isRedact,
    isText,
    moveLayer,
    parseAnnotationDocument,
    resizeLayer,
    type AnnotationTool,
    type ArrowLayer,
    type Layer,
    type RedactMode,
    type ResizeHandle,
  } from "$lib/annotations/model";
  import { createFrameBaseSource, type BaseSource } from "$lib/annotations/render";
  import {
    createBrushLayer,
    createShapeLayer,
    fontSizeForWidth,
    markerWidth,
    nextStepNumber,
    redactWidth,
    sizeForLayer,
    simplifyPoints,
    stepRadius,
    updateBrushPoints,
    updateLayerStyle,
  } from "$lib/annotations/gestures";
  import {
    createRenderScheduler,
    drawAnnotations as renderLiveAnnotations,
  } from "$lib/annotations/liveRender";
  import { LayerHistory } from "$lib/annotations/history";
  import {
    actionBarPosition,
    clamp,
    cssPointToPhysical,
    normalizeRect,
    physicalPointToCss,
    physicalToCss,
    type MonitorGeometry,
    type PhysicalPoint,
    type PhysicalRect,
  } from "$lib/overlay/geometry";
  import {
    clampRect,
    contains,
    distance,
    fullMonitorRect,
    REGION_HANDLES as handleNames,
    resizeRect,
  } from "$lib/overlay/regionSelection";
  import { loadFrames, type HistoryEditSession, type OverlaySession } from "$lib/overlay/session";
  import { invokeSelectionAction } from "$lib/overlay/exporter";
  import { handleOverlayKeyDown } from "$lib/overlay/keyboard";
  type Gesture = {
    pointerId: number;
    mode: "draw" | "move" | "resize" | "annotate" | "moveLayer" | "resizeLayer" | "curve";
    start: PhysicalPoint;
    origin: PhysicalRect | null;
    handle?: string;
    layerId?: string;
    layerOrigin?: Layer;
    tool?: AnnotationTool;
  };

  let session = $state<OverlaySession | null>(null);
  let editSession = $state<HistoryEditSession | null>(null);
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
  let redactMode = $state<RedactMode>("pixelate");
  let markerPoints: PhysicalPoint[] = [];
  let brushFree = false;
  let baseSource: BaseSource | null = null;
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
    const edit = next.edit;
    editSession = edit ?? null;
    selectedLayerId = null;
    layerPreview = null;
    gesture = null;
    editingTextId = null;
    editingPoint = null;
    editingText = "";
    activeTool = "select";
    activeColor = "#EF4444";
    strokeWidth = 5;
    redactMode = "pixelate";
    baseSource = null;
    history.reset();
    syncHistoryFlags();
    previewRect = null;
    if (edit) {
      selection = edit.rect;
      const document = parseAnnotationDocument(edit.document);
      layers = document.layers.map((layer) => moveLayer(
        layer,
        edit.rect.x,
        edit.rect.y,
      ));
    }
    session = next;
    void waitForFrames(next, sequence, arrivedAt);
  }

  async function waitForFrames(next: OverlaySession, sequence: number, arrivedAt: number) {
    await tick();
    try {
      const loaded = await loadFrames(next, arrivedAt, {
        findCanvas: (label) => document.querySelector<HTMLCanvasElement>(
          `canvas[data-monitor-label="${CSS.escape(label)}"]`,
        ),
        isCurrent: () => sequence === loadSequence,
        now: () => performance.now(),
        fetchFrame: (url) => fetch(url),
      });
      if (sequence !== loadSequence) return;
      baseSource = createFrameBaseSource(next.bounds, loaded.frames);
      await invoke("overlay_ready", { session: next.session, timings: loaded.timings });
      if (sequence === loadSequence) loading = false;
    } catch (error) {
      if (sequence !== loadSequence) return;
      console.error("Could not prepare capture frames", error);
      loading = false;
      showNotice(reportError("overlay", error));
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
    renderLiveAnnotations({
      canvas: annotationCanvas,
      session,
      selection,
      layers,
      preview: layerPreview,
      selected: selectedLayer(),
      baseSource,
    });
  }

  const scheduleRender = createRenderScheduler(drawAnnotations);

  function appendLayerGesture(
    start: PhysicalPoint,
    end: PhysicalPoint,
    tool: AnnotationTool,
    shift: boolean,
  ): Layer | null {
    if (!session || (tool !== "arrow" && tool !== "line" && tool !== "rect" && tool !== "ellipse")) {
      return null;
    }
    return createShapeLayer(start, end, tool, shift, activeColor, strokeWidth);
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
    const regionHandle = !editSession && handle && selection;
    if (editSession && (!selection || !contains(selection, point))) return;
    if (activeTool !== "select" && !regionHandle && (!selection || !contains(selection, point))) return;
    const selected = selectedLayer();
    const curvePoint = selected?.type === "arrow" ? arrowCurveHandlePoint(selected) : null;
    if (activeTool === "select" && selected?.type === "arrow" && curvePoint
      && distance(curvePoint, point) <= 9) {
      if (event.detail >= 2) {
        if (!selected.control) return;
        const straight: ArrowLayer = {
          id: selected.id, type: "arrow", color: selected.color,
          strokeWidth: selected.strokeWidth, start: selected.start, end: selected.end,
        };
        commitLayers(layers.map((layer) => layer.id === selected.id ? straight : layer));
        return;
      }
      gesture = {
        pointerId: event.pointerId, mode: "curve", start: point, origin: selection,
        layerId: selected.id, layerOrigin: $state.snapshot(selected),
      };
      rootElement.setPointerCapture(event.pointerId);
      return;
    }
    const selectedHandle = activeTool === "select" && selected && selection && contains(selection, point)
      ? annotationHandle(selected, point)
      : null;
    const hit = selectedHandle && selected
      ? selected
      : activeTool === "select" && selection && contains(selection, point)
        // Redaction is final once drawn (undo is the only way back), so it is never selectable.
        ? [...layers].reverse().find((layer) => !isRedact(layer) && hitTest(layer, point))
        : undefined;
    let mode: Gesture["mode"] = "draw";
    if (regionHandle) mode = "resize";
    else if (hit && activeTool === "select") {
      selectedLayerId = hit.id;
      if (hit.type !== "redact") activeColor = hit.color;
      strokeWidth = sizeForLayer(hit);
      const annotationResize = selectedHandle && selected?.id === hit.id
        ? selectedHandle
        : annotationHandle(hit, point);
      if (isText(hit) && event.detail >= 2) {
        beginTextEdit(hit);
        return;
      }
      if (hit.type === "step" && event.detail >= 2) {
        editStepNumber(hit);
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
    } else if (activeTool === "step" && selection && contains(selection, point)) {
      const number = nextStepNumber(layers);
      const created = createLayer({
        type: "step", x: point.x, y: point.y, number,
        radius: stepRadius(strokeWidth), color: activeColor,
      });
      commitLayers([...layers, created]);
      selectedLayerId = created.id;
      event.preventDefault();
      return;
    } else if (editSession && activeTool === "select" && selection && contains(selection, point)) {
      selectedLayerId = null;
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
    } else if (mode === "annotate"
      && (activeTool === "marker" || activeTool === "redact")) {
      brushFree = event.shiftKey;
      markerPoints = [point];
      layerPreview = createBrushLayer(
        activeTool, markerPoints, activeColor, strokeWidth, redactMode,
      );
      scheduleRender();
    }
  }

  function editStepNumber(layer: Extract<Layer, { type: "step" }>) {
    const answer = window.prompt("Step number", String(layer.number));
    if (answer === null) return;
    const number = Number(answer);
    if (!Number.isInteger(number) || number < 1) return;
    const updated: Layer = {
      id: layer.id, type: "step", x: layer.x, y: layer.y,
      number, radius: layer.radius, color: layer.color,
    };
    commitLayers(layers.map((item) => item.id === layer.id ? updated : item));
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
      color: oldTextLayer?.color ?? activeColor,
      strokeWidth: oldTextLayer?.strokeWidth ?? strokeWidth,
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
      previewRect = clampRect(
        normalizeRect(currentGesture.start, point),
        session.bounds,
      );
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
      if (currentGesture.tool === "marker" || currentGesture.tool === "redact") {
        const next = updateBrushPoints(
          markerPoints,
          brushFree,
          currentGesture.start,
          point,
          event.shiftKey,
        );
        markerPoints = next.points;
        brushFree = next.free;
        layerPreview = createBrushLayer(
          currentGesture.tool, markerPoints, activeColor, strokeWidth, redactMode,
        );
      } else {
        layerPreview = appendLayerGesture(
          currentGesture.start, point, currentGesture.tool, event.shiftKey,
        );
      }
      scheduleRender();
    } else if (currentGesture.mode === "curve" && currentGesture.layerOrigin?.type === "arrow") {
      const originArrow = currentGesture.layerOrigin;
      const updated: ArrowLayer = {
        id: originArrow.id,
        type: "arrow",
        color: originArrow.color,
        strokeWidth: originArrow.strokeWidth,
        start: originArrow.start,
        end: originArrow.end,
        control: curveControlForMidpoint(originArrow, point),
      };
      layers = layers.map((layer) => layer.id === updated.id ? updated : layer);
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
        selection = fullMonitorRect(point, session.monitors, session.bounds);
      } else {
        selection = clampRect(
          normalizeRect(currentGesture.start, point),
          session.bounds,
        );
      }
    } else if (currentGesture.mode === "annotate" && currentGesture.tool) {
      let created: Layer | null;
      if (currentGesture.tool === "marker" || currentGesture.tool === "redact") {
        const next = updateBrushPoints(
          markerPoints,
          brushFree,
          currentGesture.start,
          point,
          event.shiftKey,
        );
        markerPoints = next.points;
        brushFree = next.free;
        const simplified = simplifyPoints(markerPoints);
        created = distance(currentGesture.start, point) > 3 && simplified.length > 1
          ? createBrushLayer(
            currentGesture.tool, simplified, activeColor, strokeWidth, redactMode,
          )
          : null;
      } else {
        created = appendLayerGesture(
          currentGesture.start, point, currentGesture.tool, event.shiftKey,
        );
      }
      if (created) {
        commitLayers([...layers, created]);
        selectedLayerId = isRedact(created) ? null : created.id;
      }
      layerPreview = null;
    } else if ((currentGesture.mode === "moveLayer" || currentGesture.mode === "resizeLayer"
      || currentGesture.mode === "curve")
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
    if ((currentGesture.mode === "moveLayer" || currentGesture.mode === "resizeLayer"
      || currentGesture.mode === "curve")
      && currentGesture.layerOrigin) {
      const previousLayer = currentGesture.layerOrigin;
      layers = layers.map((layer) => layer.id === previousLayer.id ? previousLayer : layer);
    }
    layerPreview = null;
    gesture = null;
    scheduleRender();
  }

  function onKeyDown(event: KeyboardEvent) {
    if (editSession && event.key === "Escape" && !editingPoint) {
      event.preventDefault();
      void closeCapture();
      return;
    }
    handleOverlayKeyDown(event, {
      editingText: editingTextId !== null || Boolean(editingPoint),
      selectedLayer: Boolean(selectedLayerId),
      hasSelection: Boolean(selection),
      busy,
      finishTextEdit,
      setTool: (tool) => activeTool = tool,
      undo,
      redo,
      deleteSelection: () => {
        if (!selectedLayerId) return;
        commitLayers(layers.filter((layer) => layer.id !== selectedLayerId));
        selectedLayerId = null;
      },
      clearSelection: () => {
        selectedLayerId = null;
        scheduleRender();
      },
      close: () => void closeCapture(),
      copy: () => void runAction("copy_selection"),
      save: () => void runAction("save_selection"),
      selectCursorMonitor: () => void selectCursorMonitor(),
    });
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

  function changeColor(color: string) {
    activeColor = color;
    const selected = selectedLayer();
    if (selected && selected.type !== "redact") updateSelected({ color });
  }

  function changeWidth(width: number) {
    strokeWidth = width;
    const selected = selectedLayer();
    if (!selected) return;
    if (selected.type === "redact") {
      const updated: Layer = {
        id: selected.id, type: "redact", points: selected.points,
        width: redactWidth(width), mode: selected.mode,
      };
      commitLayers(layers.map((layer) => layer.id === selected.id ? updated : layer));
      return;
    }
    if (selected.type === "step") {
      const radius = stepRadius(width);
      const updated: Layer = {
        id: selected.id, type: "step", x: selected.x, y: selected.y,
        number: selected.number, radius, color: selected.color,
      };
      commitLayers(layers.map((layer) => layer.id === selected.id ? updated : layer));
      return;
    }
    if (selected.type === "marker") {
      const markerStroke = markerWidth(width);
      updateSelected({ strokeWidth: markerStroke });
      return;
    }
    updateSelected({ strokeWidth: width });
  }

  function changeRedactMode(mode: RedactMode) {
    redactMode = mode;
    const selected = selectedLayer();
    if (!isRedact(selected)) return;
    const updated: Layer = {
      id: selected.id, type: "redact", points: selected.points,
      width: selected.width, mode,
    };
    commitLayers(layers.map((layer) => layer.id === selected.id ? updated : layer));
  }
  function updateSelected(change: { color?: string; strokeWidth?: number }) {
    const selected = selectedLayer();
    if (!selected) return;
    const updated = updateLayerStyle(selected, change);
    commitLayers(layers.map((layer) => layer.id === selected.id ? updated : layer));
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
      selection = fullMonitorRect(point, session.monitors, session.bounds);
    } catch (error) {
      showNotice(reportError("overlay", error));
    }
  }

  async function runAction(command: "copy_selection" | "save_selection") {
    if (!selection || !session || busy || gesture) return;
    const actionSession = session.session;
    busy = true;
    notice = "";
    try {
      await invokeSelectionAction(
        command,
        selection,
        layers,
        baseSource,
        editSession?.historyId ?? null,
      );
    } catch (error) {
      if (session?.session === actionSession) showNotice(reportError("overlay", error));
    } finally {
      if (session?.session === actionSession) busy = false;
    }
  }

  async function closeCapture() {
    try {
      await invoke("close_overlays");
    } catch (error) {
      showNotice(reportError("overlay", error));
    }
  }

  function onContextMenu(event: MouseEvent) {
    event.preventDefault();
    void closeCapture();
  }

  onMount(() => {
    document.documentElement.dataset.theme = "dark";
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
        editSession = null;
        layers = [];
        selectedLayerId = null;
        baseSource = null;
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
    })().catch((error) => showNotice(reportError("overlay", error)));
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
    <SelectionFrame
      style={rectStyle(visibleRect)}
      width={visibleRect.width}
      height={visibleRect.height}
      handles={handleNames}
      editable={Boolean(selection && !gesture && !editSession)}
      inside={rectPosition.y < 30}
    />
  {/if}

  {#if selection && !gesture && actionPosition}
    <Toolbar
      position={actionPosition}
      tool={activeTool}
      color={activeColor}
      strokeWidth={strokeWidth}
      {redactMode}
      showRedactMode={activeTool === "redact"}
      {canUndo}
      {canRedo}
      {busy}
      {notice}
      onTool={(tool) => activeTool = tool}
      onColor={changeColor}
      onWidth={changeWidth}
      onRedactMode={changeRedactMode}
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
