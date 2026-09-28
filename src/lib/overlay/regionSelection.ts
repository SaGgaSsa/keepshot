import {
  clamp,
  monitorUnderPoint,
  nearestMonitor,
  type MonitorGeometry,
  type PhysicalPoint,
  type PhysicalRect,
  type VirtualBounds,
} from "./geometry";

export const REGION_HANDLES = ["nw", "n", "ne", "e", "se", "s", "sw", "w"] as const;

export function clampRect(rect: PhysicalRect, bounds: VirtualBounds): PhysicalRect {
  const x = clamp(rect.x, bounds.x, bounds.x + bounds.width - 1);
  const y = clamp(rect.y, bounds.y, bounds.y + bounds.height - 1);
  const right = clamp(rect.x + rect.width, x + 1, bounds.x + bounds.width);
  const bottom = clamp(rect.y + rect.height, y + 1, bounds.y + bounds.height);
  return { x, y, width: right - x, height: bottom - y };
}

export function resizeRect(
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
  if (handle.includes("e")) {
    right = clamp(origin.x + origin.width + dx, left + 1, bounds.x + bounds.width);
  }
  if (handle.includes("n")) top = clamp(origin.y + dy, bounds.y, bottom - 1);
  if (handle.includes("s")) {
    bottom = clamp(origin.y + origin.height + dy, top + 1, bounds.y + bounds.height);
  }
  return { x: left, y: top, width: right - left, height: bottom - top };
}

export function fullMonitorRect(
  point: PhysicalPoint,
  monitors: MonitorGeometry[],
  bounds: VirtualBounds,
): PhysicalRect {
  const monitor = monitorUnderPoint(point, monitors) ?? nearestMonitor(point, monitors);
  return monitor
    ? { x: monitor.x, y: monitor.y, width: monitor.width, height: monitor.height }
    : { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height };
}

export function contains(rect: PhysicalRect, point: PhysicalPoint): boolean {
  return point.x >= rect.x && point.x < rect.x + rect.width
    && point.y >= rect.y && point.y < rect.y + rect.height;
}

export function distance(left: PhysicalPoint, right: PhysicalPoint): number {
  return Math.hypot(left.x - right.x, left.y - right.y);
}
