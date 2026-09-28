import type { PhysicalPoint, PhysicalRect } from "$lib/overlay/geometry";

type Base = { id: string; color: string };
type StrokedBase = Base & { strokeWidth: number };

export type ArrowLayer = StrokedBase & {
  type: "arrow";
  start: PhysicalPoint;
  end: PhysicalPoint;
  control?: PhysicalPoint;
};
export type LineLayer = StrokedBase & {
  type: "line";
  start: PhysicalPoint;
  end: PhysicalPoint;
};
export type SegmentLayer = ArrowLayer | LineLayer;
export type RectangleLayer = StrokedBase & {
  type: "rect";
  x: number;
  y: number;
  width: number;
  height: number;
};
export type EllipseLayer = StrokedBase & {
  type: "ellipse";
  x: number;
  y: number;
  width: number;
  height: number;
};
export type BoxLayer = RectangleLayer | EllipseLayer;
export type TextLayer = StrokedBase & {
  type: "text";
  x: number;
  y: number;
  text: string;
  fontSize: number;
};
export type MarkerLayer = StrokedBase & { type: "marker"; points: PhysicalPoint[] };
export type RedactMode = "pixelate" | "blur";
export type RedactLayer = {
  id: string;
  type: "redact";
  points: PhysicalPoint[];
  width: number;
  mode: RedactMode;
};
export type StepLayer = Base & {
  type: "step";
  x: number;
  y: number;
  number: number;
  radius: number;
};
export type Layer = SegmentLayer | BoxLayer | TextLayer | MarkerLayer | RedactLayer | StepLayer;
export type AnnotationDocument = { version: 2; layers: Layer[] };
export type AnnotationTool =
  | "select" | "arrow" | "line" | "rect" | "ellipse" | "text" | "marker" | "redact" | "step";
export type ResizeHandle = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";

type NewLayer =
  | Omit<ArrowLayer, "id">
  | Omit<LineLayer, "id">
  | Omit<RectangleLayer, "id">
  | Omit<EllipseLayer, "id">
  | Omit<TextLayer, "id">
  | Omit<MarkerLayer, "id">
  | Omit<RedactLayer, "id">
  | Omit<StepLayer, "id">;

export function isSegment(layer: Layer): layer is SegmentLayer {
  return layer.type === "arrow" || layer.type === "line";
}

export function isBox(layer: Layer): layer is BoxLayer {
  return layer.type === "rect" || layer.type === "ellipse";
}

export function isRedact(layer: Layer | null | undefined): layer is RedactLayer {
  return layer?.type === "redact";
}

export function isText(layer: Layer | null | undefined): layer is TextLayer {
  return layer?.type === "text";
}

export function createLayer(layer: Omit<ArrowLayer, "id">): ArrowLayer;
export function createLayer(layer: Omit<LineLayer, "id">): LineLayer;
export function createLayer(layer: Omit<RectangleLayer, "id">): RectangleLayer;
export function createLayer(layer: Omit<EllipseLayer, "id">): EllipseLayer;
export function createLayer(layer: Omit<TextLayer, "id">): TextLayer;
export function createLayer(layer: Omit<MarkerLayer, "id">): MarkerLayer;
export function createLayer(layer: Omit<RedactLayer, "id">): RedactLayer;
export function createLayer(layer: Omit<StepLayer, "id">): StepLayer;
export function createLayer(layer: NewLayer): Layer {
  const id = crypto.randomUUID();
  switch (layer.type) {
    case "arrow":
      return {
        id, type: "arrow", color: layer.color, strokeWidth: layer.strokeWidth,
        start: layer.start, end: layer.end, control: layer.control,
      };
    case "line":
      return {
        id, type: "line", color: layer.color, strokeWidth: layer.strokeWidth,
        start: layer.start, end: layer.end,
      };
    case "rect":
      return {
        id, type: "rect", color: layer.color, strokeWidth: layer.strokeWidth,
        x: layer.x, y: layer.y, width: layer.width, height: layer.height,
      };
    case "ellipse":
      return {
        id, type: "ellipse", color: layer.color, strokeWidth: layer.strokeWidth,
        x: layer.x, y: layer.y, width: layer.width, height: layer.height,
      };
    case "text":
      return {
        id, type: "text", color: layer.color, strokeWidth: layer.strokeWidth,
        x: layer.x, y: layer.y, text: layer.text, fontSize: layer.fontSize,
      };
    case "marker":
      return {
        id, type: "marker", color: layer.color, strokeWidth: layer.strokeWidth,
        points: layer.points.map((point) => ({ x: point.x, y: point.y })),
      };
    case "redact":
      return {
        id, type: "redact", points: layer.points.map((point) => ({ x: point.x, y: point.y })),
        width: layer.width, mode: layer.mode,
      };
    case "step":
      return {
        id, type: "step", x: layer.x, y: layer.y,
        number: layer.number, radius: layer.radius, color: layer.color,
      };
  }
}

export function quadraticPoint(layer: ArrowLayer, t: number): PhysicalPoint {
  const control = layer.control ?? midpoint(layer.start, layer.end);
  const inverse = 1 - t;
  return {
    x: inverse * inverse * layer.start.x + 2 * inverse * t * control.x + t * t * layer.end.x,
    y: inverse * inverse * layer.start.y + 2 * inverse * t * control.y + t * t * layer.end.y,
  };
}

export function arrowCurveHandlePoint(layer: ArrowLayer): PhysicalPoint {
  return quadraticPoint(layer, 0.5);
}

export function curveControlForMidpoint(
  layer: ArrowLayer,
  point: PhysicalPoint,
): PhysicalPoint {
  return {
    x: 2 * point.x - (layer.start.x + layer.end.x) / 2,
    y: 2 * point.y - (layer.start.y + layer.end.y) / 2,
  };
}

export function layerBounds(layer: Layer): PhysicalRect {
  switch (layer.type) {
    case "arrow": {
      const points = sampleArrow(layer);
      const headPadding = Math.max(layer.strokeWidth * 3.5, 10) * 0.42;
      return pointBounds(points, Math.max(layer.strokeWidth / 2, headPadding));
    }
    case "line":
      return pointBounds([layer.start, layer.end], layer.strokeWidth / 2);
    case "rect":
    case "ellipse":
      return {
        x: layer.x, y: layer.y, width: layer.width, height: layer.height,
      };
    case "redact":
      return pointBounds(layer.points, layer.width / 2);
    case "text": {
      const lines = layer.text.split("\n");
      const width = lines.reduce(
        (max, line) => Math.max(max, line.length * layer.fontSize * 0.65), 0,
      );
      return {
        x: layer.x, y: layer.y, width: Math.max(12, width),
        height: layer.fontSize * lines.length * 1.25,
      };
    }
    case "marker":
      return pointBounds(layer.points, layer.strokeWidth / 2);
    case "step":
      return {
        x: layer.x - layer.radius, y: layer.y - layer.radius,
        width: layer.radius * 2, height: layer.radius * 2,
      };
    default:
      return assertNever(layer);
  }
}

function sampleArrow(layer: ArrowLayer): PhysicalPoint[] {
  return Array.from({ length: 25 }, (_, index) => quadraticPoint(layer, index / 24));
}

function pointBounds(points: PhysicalPoint[], padding = 0): PhysicalRect {
  if (points.length === 0) return { x: 0, y: 0, width: 1, height: 1 };
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  const left = Math.min(...xs) - padding;
  const top = Math.min(...ys) - padding;
  return {
    x: left, y: top,
    width: Math.max(1, Math.max(...xs) - Math.min(...xs) + padding * 2),
    height: Math.max(1, Math.max(...ys) - Math.min(...ys) + padding * 2),
  };
}

function segmentDistance(point: PhysicalPoint, start: PhysicalPoint, end: PhysicalPoint): number {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const length = dx * dx + dy * dy;
  const projection = length === 0
    ? 0
    : Math.max(0, Math.min(1, ((point.x - start.x) * dx + (point.y - start.y) * dy) / length));
  return Math.hypot(point.x - start.x - projection * dx, point.y - start.y - projection * dy);
}

function polylineDistance(point: PhysicalPoint, points: PhysicalPoint[]): number {
  let distance = Number.POSITIVE_INFINITY;
  for (let index = 1; index < points.length; index += 1) {
    distance = Math.min(distance, segmentDistance(point, points[index - 1], points[index]));
  }
  return points.length === 1 ? segmentDistance(point, points[0], points[0]) : distance;
}

export function hitTest(layer: Layer, point: PhysicalPoint, tolerance = 6): boolean {
  switch (layer.type) {
    case "arrow":
      return polylineDistance(point, sampleArrow(layer)) <= tolerance + layer.strokeWidth / 2
        || hitArrowhead(layer, point, tolerance);
    case "line":
      return segmentDistance(point, layer.start, layer.end) <= tolerance + layer.strokeWidth / 2;
    case "rect": {
      const threshold = tolerance + layer.strokeWidth / 2;
      const within = point.x >= layer.x - threshold
        && point.x <= layer.x + layer.width + threshold
        && point.y >= layer.y - threshold
        && point.y <= layer.y + layer.height + threshold;
      const edge = Math.min(
        Math.abs(point.x - layer.x), Math.abs(point.x - layer.x - layer.width),
        Math.abs(point.y - layer.y), Math.abs(point.y - layer.y - layer.height),
      );
      return within && edge <= threshold;
    }
    case "ellipse": {
      const rx = Math.max(layer.width / 2, 0.5);
      const ry = Math.max(layer.height / 2, 0.5);
      const normalized = ((point.x - layer.x - rx) / rx) ** 2
        + ((point.y - layer.y - ry) / ry) ** 2;
      return Math.abs(normalized - 1) * Math.min(rx, ry)
        <= tolerance + layer.strokeWidth / 2;
    }
    case "text": {
      const bounds = layerBounds(layer);
      return insideExpanded(point, bounds, tolerance + layer.strokeWidth / 2);
    }
    case "marker":
      return polylineDistance(point, layer.points) <= tolerance + layer.strokeWidth / 2;
    case "redact":
      return polylineDistance(point, layer.points) <= layer.width / 2 + tolerance;
    case "step":
      return Math.hypot(point.x - layer.x, point.y - layer.y) <= layer.radius + tolerance;
    default:
      return assertNever(layer);
  }
}

function insideExpanded(point: PhysicalPoint, rect: PhysicalRect, amount: number): boolean {
  return point.x >= rect.x - amount && point.x <= rect.x + rect.width + amount
    && point.y >= rect.y - amount && point.y <= rect.y + rect.height + amount;
}

function hitArrowhead(layer: ArrowLayer, point: PhysicalPoint, tolerance: number): boolean {
  const control = layer.control ?? midpoint(layer.start, layer.end);
  const angle = Math.atan2(layer.end.y - control.y, layer.end.x - control.x);
  const samples = sampleArrow(layer);
  const pathLength = samples.slice(1).reduce(
    (sum, sample, index) => sum + distance(sample, samples[index] ?? sample),
    0,
  );
  const headLength = Math.min(
    Math.max(layer.strokeWidth * 3.5, 10),
    Math.max(1, pathLength * 0.8),
  );
  let remaining = headLength;
  let base = layer.start;
  for (let index = samples.length - 1; index > 0; index -= 1) {
    const current = samples[index];
    const previous = samples[index - 1];
    if (!current || !previous) continue;
    const segment = distance(current, previous);
    if (remaining <= segment) {
      const ratio = segment === 0 ? 0 : remaining / segment;
      base = {
        x: current.x + (previous.x - current.x) * ratio,
        y: current.y + (previous.y - current.y) * ratio,
      };
      break;
    }
    remaining -= segment;
  }
  const halfWidth = headLength * 0.42;
  const left = {
    x: base.x - Math.sin(angle) * halfWidth,
    y: base.y + Math.cos(angle) * halfWidth,
  };
  const right = {
    x: base.x + Math.sin(angle) * halfWidth,
    y: base.y - Math.cos(angle) * halfWidth,
  };
  return pointInTriangle(point, layer.end, left, right)
    || segmentDistance(point, layer.end, left) <= tolerance
    || segmentDistance(point, layer.end, right) <= tolerance
    || segmentDistance(point, left, right) <= tolerance;
}

function pointInTriangle(
  point: PhysicalPoint,
  a: PhysicalPoint,
  b: PhysicalPoint,
  c: PhysicalPoint,
): boolean {
  const cross = (first: PhysicalPoint, second: PhysicalPoint, test: PhysicalPoint) =>
    (test.x - second.x) * (first.y - second.y)
    - (first.x - second.x) * (test.y - second.y);
  const first = cross(point, a, b) < 0;
  const second = cross(point, b, c) < 0;
  const third = cross(point, c, a) < 0;
  return first === second && second === third;
}

export function moveLayer(layer: Layer, dx: number, dy: number): Layer {
  switch (layer.type) {
    case "arrow":
      return {
        id: layer.id, type: "arrow", color: layer.color, strokeWidth: layer.strokeWidth,
        start: movePoint(layer.start, dx, dy), end: movePoint(layer.end, dx, dy),
        control: layer.control ? movePoint(layer.control, dx, dy) : undefined,
      };
    case "line":
      return {
        id: layer.id, type: "line", color: layer.color, strokeWidth: layer.strokeWidth,
        start: movePoint(layer.start, dx, dy), end: movePoint(layer.end, dx, dy),
      };
    case "rect":
      return {
        id: layer.id, type: "rect", color: layer.color, strokeWidth: layer.strokeWidth,
        x: layer.x + dx, y: layer.y + dy, width: layer.width, height: layer.height,
      };
    case "ellipse":
      return {
        id: layer.id, type: "ellipse", color: layer.color, strokeWidth: layer.strokeWidth,
        x: layer.x + dx, y: layer.y + dy, width: layer.width, height: layer.height,
      };
    case "text":
      return {
        id: layer.id, type: "text", color: layer.color, strokeWidth: layer.strokeWidth,
        x: layer.x + dx, y: layer.y + dy, text: layer.text, fontSize: layer.fontSize,
      };
    case "marker":
      return {
        id: layer.id, type: "marker", color: layer.color, strokeWidth: layer.strokeWidth,
        points: layer.points.map((point) => movePoint(point, dx, dy)),
      };
    case "redact":
      return {
        id: layer.id, type: "redact",
        points: layer.points.map((point) => movePoint(point, dx, dy)),
        width: layer.width, mode: layer.mode,
      };
    case "step":
      return {
        id: layer.id, type: "step", color: layer.color,
        x: layer.x + dx, y: layer.y + dy, number: layer.number, radius: layer.radius,
      };
    default:
      return assertNever(layer);
  }
}

function movePoint(point: PhysicalPoint, dx: number, dy: number): PhysicalPoint {
  return { x: point.x + dx, y: point.y + dy };
}

export function resizeLayer(layer: Layer, handle: ResizeHandle, dx: number, dy: number): Layer {
  switch (layer.type) {
    case "arrow":
      return {
        id: layer.id, type: "arrow", color: layer.color, strokeWidth: layer.strokeWidth,
        start: handle === "nw" ? movePoint(layer.start, dx, dy) : layer.start,
        end: handle !== "nw" ? movePoint(layer.end, dx, dy) : layer.end,
        control: layer.control,
      };
    case "line":
      return {
        id: layer.id, type: "line", color: layer.color, strokeWidth: layer.strokeWidth,
        start: handle === "nw" ? movePoint(layer.start, dx, dy) : layer.start,
        end: handle !== "nw" ? movePoint(layer.end, dx, dy) : layer.end,
      };
    case "rect": {
      const bounds = resizeBounds(layer, handle, dx, dy);
      return {
        id: layer.id, type: "rect", color: layer.color, strokeWidth: layer.strokeWidth,
        x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height,
      };
    }
    case "ellipse": {
      const bounds = resizeBounds(layer, handle, dx, dy);
      return {
        id: layer.id, type: "ellipse", color: layer.color, strokeWidth: layer.strokeWidth,
        x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height,
      };
    }
    case "text":
    case "marker":
    case "redact":
    case "step":
      return layer;
    default:
      return assertNever(layer);
  }
}

function resizeBounds(
  layer: RectangleLayer | EllipseLayer,
  handle: ResizeHandle,
  dx: number,
  dy: number,
): PhysicalRect {
  let left = layer.x;
  let top = layer.y;
  let right = layer.x + layer.width;
  let bottom = layer.y + layer.height;
  if (handle.includes("w")) left = Math.min(right - 1, left + dx);
  if (handle.includes("e")) right = Math.max(left + 1, right + dx);
  if (handle.includes("n")) top = Math.min(bottom - 1, top + dy);
  if (handle.includes("s")) bottom = Math.max(top + 1, bottom + dy);
  return { x: left, y: top, width: right - left, height: bottom - top };
}

export function annotationHandle(
  layer: Layer,
  point: PhysicalPoint,
  tolerance = 8,
): ResizeHandle | null {
  switch (layer.type) {
    case "arrow":
    case "line": {
      const handles: { handle: ResizeHandle; point: PhysicalPoint }[] = [
        { handle: "nw", point: layer.start }, { handle: "se", point: layer.end },
      ];
      return handles.find((item) => distance(point, item.point) <= tolerance)?.handle ?? null;
    }
    case "rect":
    case "ellipse": {
      const points = rectHandlePoints(layer);
      return points.find((item) => distance(point, item.point) <= tolerance)?.handle ?? null;
    }
    case "text":
    case "marker":
    case "redact":
    case "step":
      return null;
    default:
      return assertNever(layer);
  }
}

function rectHandlePoints(
  layer: RectangleLayer | EllipseLayer,
): { handle: ResizeHandle; point: PhysicalPoint }[] {
  const { x, y, width, height } = layer;
  return [
    { handle: "nw", point: { x, y } },
    { handle: "n", point: { x: x + width / 2, y } },
    { handle: "ne", point: { x: x + width, y } },
    { handle: "e", point: { x: x + width, y: y + height / 2 } },
    { handle: "se", point: { x: x + width, y: y + height } },
    { handle: "s", point: { x: x + width / 2, y: y + height } },
    { handle: "sw", point: { x, y: y + height } },
    { handle: "w", point: { x, y: y + height / 2 } },
  ];
}

function distance(left: PhysicalPoint, right: PhysicalPoint): number {
  return Math.hypot(left.x - right.x, left.y - right.y);
}

function midpoint(start: PhysicalPoint, end: PhysicalPoint): PhysicalPoint {
  return { x: (start.x + end.x) / 2, y: (start.y + end.y) / 2 };
}

export function parseAnnotationDocument(json: unknown): AnnotationDocument {
  if (!isRecord(json) || (json.version !== 1 && json.version !== 2)) {
    throw new TypeError("Invalid annotation document version");
  }
  if (!Array.isArray(json.layers)) throw new TypeError("Invalid annotation layers");
  return { version: 2, layers: json.layers.map(parseLayer) };
}

function parseLayer(value: unknown): Layer {
  if (!isRecord(value) || typeof value.id !== "string" || value.id.length === 0) {
    throw new TypeError("Invalid annotation layer identity");
  }
  const id = value.id;
  switch (value.type) {
    case "arrow":
      return {
        id, type: "arrow", color: stringField(value, "color"),
        strokeWidth: positiveField(value, "strokeWidth"),
        start: pointField(value, "start"), end: pointField(value, "end"),
        control: value.control === undefined ? undefined : pointField(value, "control"),
      };
    case "line":
      return {
        id, type: "line", color: stringField(value, "color"),
        strokeWidth: positiveField(value, "strokeWidth"),
        start: pointField(value, "start"), end: pointField(value, "end"),
      };
    case "rect":
    case "ellipse":
      return {
        id, type: value.type, color: stringField(value, "color"),
        strokeWidth: positiveField(value, "strokeWidth"), ...rectFields(value),
      };
    case "text":
      if (typeof value.text !== "string") throw new TypeError("Invalid annotation text");
      return {
        id, type: "text", color: stringField(value, "color"),
        strokeWidth: positiveField(value, "strokeWidth"), ...rectPosition(value),
        text: value.text, fontSize: positiveField(value, "fontSize"),
      };
    case "marker":
      if (!Array.isArray(value.points) || value.points.length === 0) {
        throw new TypeError("Invalid marker points");
      }
      return {
        id, type: "marker", color: stringField(value, "color"),
        strokeWidth: positiveField(value, "strokeWidth"),
        points: value.points.map(parsePoint),
      };
    case "redact":
      if (value.mode !== "pixelate" && value.mode !== "blur") {
        throw new TypeError("Invalid redaction mode");
      }
      if (!Array.isArray(value.points) || value.points.length === 0) {
        throw new TypeError("Invalid redaction points");
      }
      return {
        id, type: "redact", points: value.points.map(parsePoint),
        width: positiveField(value, "width"), mode: value.mode,
      };
    case "step":
      if (!Number.isInteger(value.number) || (value.number as number) < 1) {
        throw new TypeError("Invalid step number");
      }
      return {
        id, type: "step", color: stringField(value, "color"),
        x: finiteField(value, "x"), y: finiteField(value, "y"),
        number: value.number as number, radius: positiveField(value, "radius"),
      };
    default:
      throw new TypeError("Unsupported annotation layer type");
  }
}

function rectFields(value: Record<string, unknown>): PhysicalRect {
  const rect = rectPosition(value);
  const width = finiteField(value, "width");
  const height = finiteField(value, "height");
  if (width < 0 || height < 0) throw new TypeError("Invalid annotation dimensions");
  return { ...rect, width, height };
}

function rectPosition(value: Record<string, unknown>): Pick<PhysicalRect, "x" | "y"> {
  return { x: finiteField(value, "x"), y: finiteField(value, "y") };
}

function pointField(value: Record<string, unknown>, key: string): PhysicalPoint {
  return parsePoint(value[key]);
}

function parsePoint(value: unknown): PhysicalPoint {
  if (!isRecord(value)) throw new TypeError("Invalid annotation point");
  return { x: finiteField(value, "x"), y: finiteField(value, "y") };
}

function finiteField(value: Record<string, unknown>, key: string): number {
  const field = value[key];
  if (typeof field !== "number" || !Number.isFinite(field)) {
    throw new TypeError(`Invalid annotation ${key}`);
  }
  return field;
}

function positiveField(value: Record<string, unknown>, key: string): number {
  const field = finiteField(value, key);
  if (field <= 0) throw new TypeError(`Invalid annotation ${key}`);
  return field;
}

function stringField(value: Record<string, unknown>, key: string): string {
  const field = value[key];
  if (typeof field !== "string") throw new TypeError(`Invalid annotation ${key}`);
  return field;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function assertNever(value: never): never {
  throw new Error(`Unsupported annotation layer: ${JSON.stringify(value)}`);
}
