import type { PhysicalPoint, PhysicalRect } from "$lib/overlay/geometry";

type Base = { id: string; color: string; strokeWidth: number };

export type ArrowLayer = Base & {
  type: "arrow";
  start: PhysicalPoint;
  end: PhysicalPoint;
};

export type LineLayer = Base & {
  type: "line";
  start: PhysicalPoint;
  end: PhysicalPoint;
};

export type SegmentLayer = ArrowLayer | LineLayer;

export type RectangleLayer = Base & {
  type: "rect";
  x: number;
  y: number;
  width: number;
  height: number;
};

export type EllipseLayer = Base & {
  type: "ellipse";
  x: number;
  y: number;
  width: number;
  height: number;
};

export type BoxLayer = RectangleLayer | EllipseLayer;

export type TextLayer = Base & {
  type: "text";
  x: number;
  y: number;
  text: string;
  fontSize: number;
};

export type Layer = SegmentLayer | BoxLayer | TextLayer;
export type AnnotationDocument = { version: 1; layers: Layer[] };
export type AnnotationTool = "select" | "arrow" | "line" | "rect" | "ellipse" | "text";
export type ResizeHandle = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";

type NewLayer =
  | Omit<ArrowLayer, "id">
  | Omit<LineLayer, "id">
  | Omit<RectangleLayer, "id">
  | Omit<EllipseLayer, "id">
  | Omit<TextLayer, "id">;

export function isSegment(layer: Layer): layer is SegmentLayer {
  return layer.type === "arrow" || layer.type === "line";
}

export function isBox(layer: Layer): layer is BoxLayer {
  return layer.type === "rect" || layer.type === "ellipse";
}

export function isText(layer: Layer | null | undefined): layer is TextLayer {
  return layer?.type === "text";
}

export function createLayer(layer: Omit<ArrowLayer, "id">): ArrowLayer;
export function createLayer(layer: Omit<LineLayer, "id">): LineLayer;
export function createLayer(layer: Omit<RectangleLayer, "id">): RectangleLayer;
export function createLayer(layer: Omit<EllipseLayer, "id">): EllipseLayer;
export function createLayer(layer: Omit<TextLayer, "id">): TextLayer;
export function createLayer(layer: NewLayer): Layer {
  const id = crypto.randomUUID();
  switch (layer.type) {
    case "arrow":
      return {
        id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "arrow",
        start: layer.start,
        end: layer.end,
      };
    case "line":
      return {
        id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "line",
        start: layer.start,
        end: layer.end,
      };
    case "rect":
      return {
        id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "rect",
        x: layer.x,
        y: layer.y,
        width: layer.width,
        height: layer.height,
      };
    case "ellipse":
      return {
        id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "ellipse",
        x: layer.x,
        y: layer.y,
        width: layer.width,
        height: layer.height,
      };
    case "text":
      return {
        id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "text",
        x: layer.x,
        y: layer.y,
        text: layer.text,
        fontSize: layer.fontSize,
      };
  }
}

export function layerBounds(layer: Layer): PhysicalRect {
  switch (layer.type) {
    case "arrow":
    case "line": {
      const x = Math.min(layer.start.x, layer.end.x);
      const y = Math.min(layer.start.y, layer.end.y);
      return {
        x,
        y,
        width: Math.max(1, Math.abs(layer.end.x - layer.start.x)),
        height: Math.max(1, Math.abs(layer.end.y - layer.start.y)),
      };
    }
    case "rect":
    case "ellipse":
      return { x: layer.x, y: layer.y, width: layer.width, height: layer.height };
    case "text": {
      const lines = layer.text.split("\n");
      const width = lines.reduce((max, line) => Math.max(max, line.length * layer.fontSize * 0.65), 0);
      return {
        x: layer.x,
        y: layer.y,
        width: Math.max(12, width),
        height: layer.fontSize * lines.length * 1.25,
      };
    }
  }
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

export function hitTest(layer: Layer, point: PhysicalPoint, tolerance = 6): boolean {
  const threshold = tolerance + layer.strokeWidth / 2;
  switch (layer.type) {
    case "arrow":
    case "line":
      return segmentDistance(point, layer.start, layer.end) <= threshold;
    case "text": {
      const bounds = layerBounds(layer);
      return point.x >= bounds.x - tolerance
        && point.x <= bounds.x + bounds.width + tolerance
        && point.y >= bounds.y - tolerance
        && point.y <= bounds.y + bounds.height + tolerance;
    }
    case "rect": {
      const withinBounds = point.x >= layer.x - threshold
        && point.x <= layer.x + layer.width + threshold
        && point.y >= layer.y - threshold
        && point.y <= layer.y + layer.height + threshold;
      const nearestEdge = Math.min(
        Math.abs(point.x - layer.x),
        Math.abs(point.x - layer.x - layer.width),
        Math.abs(point.y - layer.y),
        Math.abs(point.y - layer.y - layer.height),
      );
      return withinBounds && nearestEdge <= threshold;
    }
    case "ellipse": {
      const rx = Math.max(layer.width / 2, 0.5);
      const ry = Math.max(layer.height / 2, 0.5);
      const normalized = ((point.x - layer.x - rx) / rx) ** 2 + ((point.y - layer.y - ry) / ry) ** 2;
      return Math.abs(normalized - 1) * Math.min(rx, ry) <= threshold;
    }
  }
}

export function moveLayer(layer: Layer, dx: number, dy: number): Layer {
  switch (layer.type) {
    case "arrow":
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "arrow",
        start: { x: layer.start.x + dx, y: layer.start.y + dy },
        end: { x: layer.end.x + dx, y: layer.end.y + dy },
      };
    case "line":
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "line",
        start: { x: layer.start.x + dx, y: layer.start.y + dy },
        end: { x: layer.end.x + dx, y: layer.end.y + dy },
      };
    case "rect":
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "rect",
        x: layer.x + dx,
        y: layer.y + dy,
        width: layer.width,
        height: layer.height,
      };
    case "ellipse":
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "ellipse",
        x: layer.x + dx,
        y: layer.y + dy,
        width: layer.width,
        height: layer.height,
      };
    case "text":
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "text",
        x: layer.x + dx,
        y: layer.y + dy,
        text: layer.text,
        fontSize: layer.fontSize,
      };
  }
}

export function resizeLayer(layer: Layer, handle: ResizeHandle, dx: number, dy: number): Layer {
  switch (layer.type) {
    case "arrow": {
      const start = handle === "nw"
        ? { x: layer.start.x + dx, y: layer.start.y + dy }
        : layer.start;
      const end = handle === "nw"
        ? layer.end
        : { x: layer.end.x + dx, y: layer.end.y + dy };
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "arrow",
        start,
        end,
      };
    }
    case "line": {
      const start = handle === "nw"
        ? { x: layer.start.x + dx, y: layer.start.y + dy }
        : layer.start;
      const end = handle === "nw"
        ? layer.end
        : { x: layer.end.x + dx, y: layer.end.y + dy };
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "line",
        start,
        end,
      };
    }
    case "rect": {
      const bounds = resizeBounds(layer, handle, dx, dy);
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "rect",
        ...bounds,
      };
    }
    case "ellipse": {
      const bounds = resizeBounds(layer, handle, dx, dy);
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "ellipse",
        ...bounds,
      };
    }
    case "text":
      return {
        id: layer.id,
        color: layer.color,
        strokeWidth: layer.strokeWidth,
        type: "text",
        x: layer.x,
        y: layer.y,
        text: layer.text,
        fontSize: layer.fontSize,
      };
  }
}

function resizeBounds(
  layer: BoxLayer,
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

export function annotationHandle(layer: Layer, point: PhysicalPoint, tolerance = 8): ResizeHandle | null {
  switch (layer.type) {
    case "arrow":
    case "line": {
      const handles = [
        { handle: "nw" as const, point: layer.start },
        { handle: "se" as const, point: layer.end },
      ];
      return handles.find((item) =>
        Math.hypot(point.x - item.point.x, point.y - item.point.y) <= tolerance,
      )?.handle ?? null;
    }
    case "rect":
    case "ellipse": {
      const handles: { handle: ResizeHandle; point: PhysicalPoint }[] = [
        { handle: "nw", point: { x: layer.x, y: layer.y } },
        { handle: "n", point: { x: layer.x + layer.width / 2, y: layer.y } },
        { handle: "ne", point: { x: layer.x + layer.width, y: layer.y } },
        { handle: "e", point: { x: layer.x + layer.width, y: layer.y + layer.height / 2 } },
        { handle: "se", point: { x: layer.x + layer.width, y: layer.y + layer.height } },
        { handle: "s", point: { x: layer.x + layer.width / 2, y: layer.y + layer.height } },
        { handle: "sw", point: { x: layer.x, y: layer.y + layer.height } },
        { handle: "w", point: { x: layer.x, y: layer.y + layer.height / 2 } },
      ];
      return handles.find((item) =>
        Math.hypot(point.x - item.point.x, point.y - item.point.y) <= tolerance,
      )?.handle ?? null;
    }
    case "text":
      return null;
  }
}
