import {
  createLayer,
  type Layer,
  type RedactMode,
} from "$lib/annotations/model";
import type { PhysicalPoint } from "$lib/overlay/geometry";

export type ShapeTool = "arrow" | "line" | "rect" | "ellipse";
export type BrushTool = "marker" | "redact";
export type BrushPointState = { points: PhysicalPoint[]; free: boolean };

export function constrainPoint(
  start: PhysicalPoint,
  point: PhysicalPoint,
  shift: boolean,
): PhysicalPoint {
  if (!shift) return point;
  const dx = point.x - start.x;
  const dy = point.y - start.y;
  const angle = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * Math.PI / 4;
  const length = Math.max(Math.abs(dx), Math.abs(dy));
  return {
    x: Math.round(start.x + Math.cos(angle) * length),
    y: Math.round(start.y + Math.sin(angle) * length),
  };
}

export function createShapeLayer(
  start: PhysicalPoint,
  end: PhysicalPoint,
  tool: ShapeTool,
  shift: boolean,
  color: string,
  strokeWidth: number,
): Layer | null {
  if (distance(start, end) <= 3) return null;
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
  switch (tool) {
    case "arrow":
      return createLayer({ type: "arrow", color, strokeWidth, start, end: snapped });
    case "line":
      return createLayer({ type: "line", color, strokeWidth, start, end: snapped });
    case "rect":
      return createLayer({
        type: "rect", color, strokeWidth, x, y,
        width: Math.max(1, width), height: Math.max(1, height),
      });
    case "ellipse":
      return createLayer({
        type: "ellipse", color, strokeWidth, x, y,
        width: Math.max(1, width), height: Math.max(1, height),
      });
    default:
      return assertNever(tool);
  }
}

export function markerWidth(size: number): number {
  if (size === 3) return 14;
  if (size === 8) return 34;
  return 22;
}

export function redactWidth(size: number): number {
  if (size === 3) return 16;
  if (size === 8) return 44;
  return 28;
}

export function stepRadius(size: number): number {
  if (size === 3) return 14;
  if (size === 8) return 28;
  return 20;
}

export function sizeForLayer(layer: Layer): number {
  switch (layer.type) {
    case "marker":
      return layer.strokeWidth <= 14 ? 3 : layer.strokeWidth <= 22 ? 5 : 8;
    case "redact":
      return layer.width <= 16 ? 3 : layer.width <= 28 ? 5 : 8;
    case "step":
      return layer.radius <= 14 ? 3 : layer.radius <= 20 ? 5 : 8;
    case "arrow":
    case "line":
    case "rect":
    case "ellipse":
    case "text":
      return layer.strokeWidth;
    default:
      return assertNever(layer);
  }
}

export function nextStepNumber(layers: readonly Layer[]): number {
  return layers.reduce(
    (maximum, layer) => layer.type === "step" ? Math.max(maximum, layer.number) : maximum,
    0,
  ) + 1;
}

export function updateBrushPoints(
  previous: readonly PhysicalPoint[],
  wasFree: boolean,
  start: PhysicalPoint,
  point: PhysicalPoint,
  free: boolean,
): BrushPointState {
  if (!free) return { points: [start, axisPoint(start, point)], free: false };
  const points = wasFree ? [...previous] : [start];
  const lastPoint = points[points.length - 1];
  if (!lastPoint || distance(lastPoint, point) >= 1) points.push(point);
  return { points, free: true };
}

export function axisPoint(start: PhysicalPoint, point: PhysicalPoint): PhysicalPoint {
  const dx = point.x - start.x;
  const dy = point.y - start.y;
  return Math.abs(dx) >= Math.abs(dy)
    ? { x: point.x, y: start.y }
    : { x: start.x, y: point.y };
}

export function createBrushLayer(
  tool: BrushTool,
  points: PhysicalPoint[],
  color: string,
  size: number,
  mode: RedactMode,
): Layer {
  if (tool === "marker") {
    return createLayer({ type: "marker", color, strokeWidth: markerWidth(size), points });
  }
  return createLayer({ type: "redact", points, width: redactWidth(size), mode });
}

export function simplifyPoints(points: readonly PhysicalPoint[]): PhysicalPoint[] {
  const simplified: PhysicalPoint[] = [];
  for (const point of points) {
    const previous = simplified[simplified.length - 1];
    if (!previous || distance(previous, point) >= 2) simplified.push(point);
  }
  return simplified;
}

export function fontSizeForWidth(width: number): number {
  if (width === 3) return 16;
  if (width === 5) return 24;
  return 36;
}

export function updateLayerStyle(
  layer: Layer,
  change: { color?: string; strokeWidth?: number },
): Layer {
  if (layer.type === "redact") return layer;
  if (layer.type === "step") {
    return {
      id: layer.id, type: "step", x: layer.x, y: layer.y, number: layer.number,
      radius: layer.radius, color: change.color ?? layer.color,
    };
  }
  const color = change.color ?? layer.color;
  const nextWidth = change.strokeWidth ?? layer.strokeWidth;
  switch (layer.type) {
    case "arrow":
      return {
        id: layer.id, type: "arrow", color, strokeWidth: nextWidth,
        start: layer.start, end: layer.end, control: layer.control,
      };
    case "line":
      return {
        id: layer.id, type: "line", color, strokeWidth: nextWidth,
        start: layer.start, end: layer.end,
      };
    case "rect":
      return {
        id: layer.id, type: "rect", color, strokeWidth: nextWidth,
        x: layer.x, y: layer.y, width: layer.width, height: layer.height,
      };
    case "ellipse":
      return {
        id: layer.id, type: "ellipse", color, strokeWidth: nextWidth,
        x: layer.x, y: layer.y, width: layer.width, height: layer.height,
      };
    case "text":
      return {
        id: layer.id, type: "text", color, strokeWidth: nextWidth,
        x: layer.x, y: layer.y, text: layer.text,
        fontSize: change.strokeWidth === undefined ? layer.fontSize : fontSizeForWidth(nextWidth),
      };
    case "marker":
      return {
        id: layer.id, type: "marker", color, strokeWidth: nextWidth, points: layer.points,
      };
    default:
      return assertNever(layer);
  }
}

function distance(left: PhysicalPoint, right: PhysicalPoint): number {
  return Math.hypot(left.x - right.x, left.y - right.y);
}

function assertNever(value: never): never {
  throw new Error(`Unsupported annotation layer: ${JSON.stringify(value)}`);
}
