export type PhysicalRect = { x: number; y: number; width: number; height: number };
export type VirtualBounds = { x: number; y: number; width: number; height: number };
export type MonitorGeometry = PhysicalRect & { label: string; name: string; scaleFactor: number };
export type PhysicalPoint = { x: number; y: number };

export function cssPointToPhysical(
  clientX: number,
  clientY: number,
  bounds: VirtualBounds,
  devicePixelRatio: number,
): PhysicalPoint {
  return {
    x: clamp(Math.round(bounds.x + clientX * devicePixelRatio), bounds.x, bounds.x + bounds.width - 1),
    y: clamp(Math.round(bounds.y + clientY * devicePixelRatio), bounds.y, bounds.y + bounds.height - 1),
  };
}

export function physicalToCss(value: number, devicePixelRatio: number): number {
  return value / devicePixelRatio;
}

export function physicalPointToCss(
  point: PhysicalPoint,
  bounds: VirtualBounds,
  devicePixelRatio: number,
): PhysicalPoint {
  return {
    x: physicalToCss(point.x - bounds.x, devicePixelRatio),
    y: physicalToCss(point.y - bounds.y, devicePixelRatio),
  };
}

export function normalizeRect(start: PhysicalPoint, end: PhysicalPoint): PhysicalRect {
  const left = Math.min(start.x, end.x);
  const top = Math.min(start.y, end.y);
  return {
    x: left,
    y: top,
    width: Math.abs(end.x - start.x) + 1,
    height: Math.abs(end.y - start.y) + 1,
  };
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), Math.max(min, max));
}

export function monitorUnderPoint(
  point: PhysicalPoint,
  monitors: MonitorGeometry[],
): MonitorGeometry | undefined {
  return monitors.find(
    (monitor) =>
      point.x >= monitor.x &&
      point.y >= monitor.y &&
      point.x < monitor.x + monitor.width &&
      point.y < monitor.y + monitor.height,
  );
}

export function nearestMonitor(point: PhysicalPoint, monitors: MonitorGeometry[]): MonitorGeometry | undefined {
  return monitors.reduce<MonitorGeometry | undefined>((nearest, monitor) => {
    if (!nearest) return monitor;
    const distance = distanceToMonitor(point, monitor);
    const nearestDistance = distanceToMonitor(point, nearest);
    return distance < nearestDistance ? monitor : nearest;
  }, undefined);
}

export function actionBarPosition(
  rect: PhysicalRect,
  monitors: MonitorGeometry[],
  bounds: VirtualBounds,
  devicePixelRatio: number,
): PhysicalPoint & { width: number; height: number } {
  const corner = { x: rect.x + rect.width - 1, y: rect.y + rect.height - 1 };
  const monitor = monitorUnderPoint(corner, monitors) ?? nearestMonitor(corner, monitors);
  if (!monitor) return { x: 8, y: 8, width: 640, height: 52 };

  const barWidth = Math.min(640 * devicePixelRatio, monitor.width);
  const barHeight = Math.min((monitor.width < 640 * devicePixelRatio ? 104 : 52) * devicePixelRatio, monitor.height);
  const gap = 12 * devicePixelRatio;
  let x = rect.x + rect.width - barWidth;
  let y = rect.y + rect.height + gap;
  const monitorRight = monitor.x + monitor.width;
  const monitorBottom = monitor.y + monitor.height;

  if (y + barHeight > monitorBottom) y = rect.y - barHeight - gap;
  if (y < monitor.y) y = rect.y + rect.height - barHeight - 8 * devicePixelRatio;
  x = clamp(x, monitor.x, monitorRight - barWidth);
  y = clamp(y, monitor.y, monitorBottom - barHeight);

  return {
    ...physicalPointToCss({ x, y }, bounds, devicePixelRatio),
    width: physicalToCss(barWidth, devicePixelRatio),
    height: physicalToCss(barHeight, devicePixelRatio),
  };
}

function distanceToMonitor(point: PhysicalPoint, monitor: MonitorGeometry): number {
  const dx = Math.max(monitor.x - point.x, 0, point.x - (monitor.x + monitor.width));
  const dy = Math.max(monitor.y - point.y, 0, point.y - (monitor.y + monitor.height));
  return dx * dx + dy * dy;
}
