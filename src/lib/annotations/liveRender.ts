import type { Layer } from "./model";
import {
  arrowCurveHandlePoint,
  isBox,
  isRedact,
  isSegment,
  isText,
  layerBounds,
} from "./model";
import { renderLayers, type BaseSource } from "./render";
import type { OverlaySession } from "$lib/overlay/session";
import type { PhysicalPoint, PhysicalRect } from "$lib/overlay/geometry";

export type LiveRenderState = {
  canvas: HTMLCanvasElement | undefined;
  session: OverlaySession | null;
  selection: PhysicalRect | null;
  layers: Layer[];
  preview: Layer | null;
  selected: Layer | undefined;
  baseSource: BaseSource | null;
};

export function drawAnnotations(state: LiveRenderState): void {
  const { canvas, session, selection } = state;
  if (!canvas || !session) return;
  const dpr = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.round(window.innerWidth * dpr));
  const height = Math.max(1, Math.round(window.innerHeight * dpr));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, width, height);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  if (!selection) return;
  const left = (selection.x - session.bounds.x) / dpr;
  const top = (selection.y - session.bounds.y) / dpr;
  ctx.save();
  ctx.beginPath();
  ctx.rect(left, top, selection.width / dpr, selection.height / dpr);
  ctx.clip();
  const visibleLayers = state.preview ? [...state.layers, state.preview] : state.layers;
  renderLayers(
    ctx,
    visibleLayers,
    {
      originX: session.bounds.x,
      originY: session.bounds.y,
      scale: 1 / dpr,
    },
    visibleLayers.length ? state.baseSource ?? undefined : undefined,
    selection,
  );
  if (state.selected) drawLayerHandles(ctx, state.selected, dpr, session.bounds.x, session.bounds.y);
  ctx.restore();
}

export function createRenderScheduler(render: () => void): () => void {
  let scheduled = false;
  return () => {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      render();
    });
  };
}

function drawLayerHandles(
  ctx: CanvasRenderingContext2D,
  layer: Layer,
  dpr: number,
  originX: number,
  originY: number,
): void {
  const toCanvasX = (value: number) => (value - originX) / dpr;
  const toCanvasY = (value: number) => (value - originY) / dpr;
  const rectLayer = isBox(layer) ? layer : null;
  const anchors = isSegment(layer)
    ? [layer.start, layer.end]
    : rectLayer
      ? rectHandlePoints(rectLayer)
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
  } else if (rectLayer) {
    ctx.rect(
      toCanvasX(rectLayer.x),
      toCanvasY(rectLayer.y),
      rectLayer.width / dpr,
      rectLayer.height / dpr,
    );
  } else if (isText(layer)) {
    const textWidth = Math.max(12, layer.text.length * layer.fontSize * 0.65);
    ctx.rect(
      toCanvasX(layer.x),
      toCanvasY(layer.y),
      textWidth / dpr,
      layer.fontSize * 1.25 / dpr,
    );
  } else if (layer.type === "step") {
    ctx.arc(toCanvasX(layer.x), toCanvasY(layer.y), layer.radius / dpr, 0, Math.PI * 2);
  } else if (layer.type === "marker" || isRedact(layer)) {
    const bounds = layerBounds(layer);
    ctx.setLineDash([4, 3]);
    ctx.rect(
      toCanvasX(bounds.x) - 2,
      toCanvasY(bounds.y) - 2,
      bounds.width / dpr + 4,
      bounds.height / dpr + 4,
    );
  }
  ctx.stroke();
  ctx.setLineDash([]);
  for (const point of anchors) {
    ctx.beginPath();
    ctx.arc(toCanvasX(point.x), toCanvasY(point.y), 3, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
  }
  if (layer.type === "arrow") {
    const curve = arrowCurveHandlePoint(layer);
    ctx.beginPath();
    ctx.arc(toCanvasX(curve.x), toCanvasY(curve.y), 4, 0, Math.PI * 2);
    ctx.fillStyle = "#F59E0B";
    ctx.fill();
    ctx.stroke();
  }
  ctx.restore();
}

function rectHandlePoints(layer: Extract<Layer, { type: "rect" | "ellipse" }>): PhysicalPoint[] {
  const { x, y, width, height } = layer;
  return [
    { x, y },
    { x: x + width / 2, y },
    { x: x + width, y },
    { x: x + width, y: y + height / 2 },
    { x: x + width, y: y + height },
    { x: x + width / 2, y: y + height },
    { x, y: y + height },
    { x, y: y + height / 2 },
  ];
}
