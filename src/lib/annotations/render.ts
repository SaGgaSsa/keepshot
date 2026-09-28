import type { PhysicalPoint, PhysicalRect } from "$lib/overlay/geometry";
import type { ArrowLayer, Layer, RedactLayer } from "./model";
import { quadraticPoint } from "./model";

export type RenderTransform = { originX: number; originY: number; scale: number };
export type BaseFrame = PhysicalRect & { canvas: HTMLCanvasElement };
export type BaseSource = {
  originX: number;
  originY: number;
  draw(ctx: CanvasRenderingContext2D, rect: PhysicalRect): void;
  canvasFor(rect: PhysicalRect): HTMLCanvasElement;
  invalidate(): void;
};

export function createFrameBaseSource(
  bounds: PhysicalRect,
  frames: BaseFrame[],
): BaseSource {
  const canvases = new Map<string, HTMLCanvasElement>();
  return {
    originX: bounds.x,
    originY: bounds.y,
    draw(ctx, rect) {
      const canvas = this.canvasFor(rect);
      ctx.drawImage(canvas, rect.x, rect.y);
    },
    canvasFor(rect) {
      const key = rectKey(rect);
      const cached = canvases.get(key);
      if (cached) return cached;
      const canvas = document.createElement("canvas");
      canvas.width = Math.max(1, Math.ceil(rect.width));
      canvas.height = Math.max(1, Math.ceil(rect.height));
      const ctx = canvas.getContext("2d", { alpha: false });
      if (!ctx) throw new Error("Canvas 2D is unavailable");
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      for (const frame of frames) {
        const left = Math.max(rect.x, frame.x);
        const top = Math.max(rect.y, frame.y);
        const right = Math.min(rect.x + rect.width, frame.x + frame.width);
        const bottom = Math.min(rect.y + rect.height, frame.y + frame.height);
        if (right <= left || bottom <= top) continue;
        ctx.drawImage(
          frame.canvas,
          left - frame.x,
          top - frame.y,
          right - left,
          bottom - top,
          left - rect.x,
          top - rect.y,
          right - left,
          bottom - top,
        );
      }
      canvases.set(key, canvas);
      if (canvases.size > 12) {
        const oldest = canvases.keys().next().value;
        if (oldest) canvases.delete(oldest);
      }
      return canvas;
    },
    invalidate() {
      canvases.clear();
    },
  };
}

function rectKey(rect: PhysicalRect): string {
  return `${rect.x}:${rect.y}:${rect.width}:${rect.height}`;
}

export function renderLayers(
  ctx: CanvasRenderingContext2D,
  layers: readonly Layer[],
  transform: RenderTransform,
  source?: BaseSource,
  backgroundRect?: PhysicalRect,
): void {
  ctx.save();
  ctx.transform(
    transform.scale,
    0,
    0,
    transform.scale,
    -transform.originX * transform.scale,
    -transform.originY * transform.scale,
  );
  if (source && backgroundRect) source.draw(ctx, backgroundRect);
  ctx.lineCap = "round";
  ctx.lineJoin = "round";

  for (const layer of layers) {
    ctx.save();
    switch (layer.type) {
      case "line":
        ctx.strokeStyle = layer.color;
        ctx.lineWidth = layer.strokeWidth;
        ctx.beginPath();
        ctx.moveTo(layer.start.x, layer.start.y);
        ctx.lineTo(layer.end.x, layer.end.y);
        ctx.stroke();
        break;
      case "arrow":
        ctx.strokeStyle = layer.color;
        ctx.fillStyle = layer.color;
        ctx.lineWidth = layer.strokeWidth;
        drawArrow(ctx, layer);
        break;
      case "rect":
        ctx.strokeStyle = layer.color;
        ctx.lineWidth = layer.strokeWidth;
        ctx.strokeRect(layer.x, layer.y, layer.width, layer.height);
        break;
      case "ellipse":
        ctx.strokeStyle = layer.color;
        ctx.lineWidth = layer.strokeWidth;
        ctx.beginPath();
        ctx.ellipse(
          layer.x + layer.width / 2,
          layer.y + layer.height / 2,
          Math.max(0.5, layer.width / 2),
          Math.max(0.5, layer.height / 2),
          0,
          0,
          Math.PI * 2,
        );
        ctx.stroke();
        break;
      case "text":
        ctx.fillStyle = layer.color;
        ctx.font = `600 ${layer.fontSize}px Inter, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif`;
        ctx.textBaseline = "top";
        layer.text.split("\n").forEach((line, index) => {
          ctx.fillText(line, layer.x, layer.y + index * layer.fontSize * 1.25);
        });
        break;
      case "marker":
        ctx.strokeStyle = layer.color;
        ctx.lineWidth = layer.strokeWidth;
        ctx.globalAlpha = 0.45;
        // White ink uses screen so it remains visible over the photograph.
        ctx.globalCompositeOperation = layer.color.toUpperCase() === "#FFFFFF"
          ? "screen"
          : "multiply";
        drawPolyline(ctx, layer.points);
        break;
      case "redact":
        if (source) drawRedaction(ctx, layer, source);
        break;
      case "step":
        drawStep(ctx, layer.x, layer.y, layer.number, layer.radius, layer.color);
        break;
      default:
        assertNever(layer);
    }
    ctx.restore();
  }

  ctx.restore();
}

function drawArrow(ctx: CanvasRenderingContext2D, layer: ArrowLayer): void {
  const control = layer.control ?? {
    x: (layer.start.x + layer.end.x) / 2,
    y: (layer.start.y + layer.end.y) / 2,
  };
  const tangent = { x: layer.end.x - control.x, y: layer.end.y - control.y };
  const length = Math.hypot(tangent.x, tangent.y) || 1;
  const direction = { x: tangent.x / length, y: tangent.y / length };
  const samples = Array.from({ length: 25 }, (_, index) => quadraticPoint(layer, index / 24));
  const pathLength = samples.slice(1).reduce(
    (sum, point, index) => sum + distance(point, samples[index] ?? point),
    0,
  );
  const headLength = Math.min(
    Math.max(layer.strokeWidth * 3.5, 10),
    Math.max(1, pathLength * 0.8),
  );
  let remaining = headLength;
  let cutIndex = 0;
  let base = layer.start;
  for (let index = samples.length - 1; index > 0; index -= 1) {
    const current = samples[index];
    const previous = samples[index - 1];
    if (!current || !previous) continue;
    const segment = distance(current, previous);
    if (remaining <= segment) {
      cutIndex = index - 1;
      const ratio = segment === 0 ? 0 : remaining / segment;
      base = {
        x: current.x + (previous.x - current.x) * ratio,
        y: current.y + (previous.y - current.y) * ratio,
      };
      break;
    }
    remaining -= segment;
  }
  const half = headLength * 0.42;
  ctx.beginPath();
  ctx.moveTo(layer.start.x, layer.start.y);
  for (let index = 1; index <= cutIndex; index += 1) {
    const point = samples[index];
    if (point) ctx.lineTo(point.x, point.y);
  }
  ctx.lineTo(base.x, base.y);
  ctx.stroke();

  ctx.beginPath();
  ctx.moveTo(layer.end.x, layer.end.y);
  ctx.lineTo(base.x - direction.y * half, base.y + direction.x * half);
  ctx.lineTo(base.x + direction.y * half, base.y - direction.x * half);
  ctx.closePath();
  ctx.fill();
}

function drawPolyline(ctx: CanvasRenderingContext2D, points: PhysicalPoint[]): void {
  if (points.length === 0) return;
  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (let index = 1; index < points.length; index += 1) {
    const point = points[index];
    if (point) ctx.lineTo(point.x, point.y);
  }
  if (points.length === 1) ctx.lineTo(points[0].x + 0.01, points[0].y + 0.01);
  ctx.stroke();
}

function drawRedaction(
  ctx: CanvasRenderingContext2D,
  layer: RedactLayer,
  source: BaseSource,
): void {
  const cached = redactionCanvases.get(source)?.get(JSON.stringify(layer));
  if (cached) {
    ctx.beginPath();
    ctx.rect(layer.x, layer.y, layer.width, layer.height);
    ctx.clip();
    ctx.drawImage(cached, Math.floor(layer.x), Math.floor(layer.y));
    return;
  }
  const left = Math.floor(layer.x);
  const top = Math.floor(layer.y);
  const right = Math.ceil(layer.x + layer.width);
  const bottom = Math.ceil(layer.y + layer.height);
  const rect = { x: left, y: top, width: right - left, height: bottom - top };
  const margin = layer.mode === "pixelate"
    ? Math.ceil(layer.strength)
    : Math.ceil(layer.strength * 3);
  const expandedRect = {
    x: rect.x - margin,
    y: rect.y - margin,
    width: rect.width + margin * 2,
    height: rect.height + margin * 2,
  };
  const original = source.canvasFor(expandedRect);
  const output = document.createElement("canvas");
  output.width = rect.width;
  output.height = rect.height;
  const out = output.getContext("2d");
  if (!out) return;
  if (layer.mode === "pixelate") {
    pixelate(out, original, layer, expandedRect, rect, source);
  } else {
    blur(out, original, layer.strength, margin, rect.width, rect.height);
  }
  let cache = redactionCanvases.get(source);
  if (!cache) {
    cache = new Map();
    redactionCanvases.set(source, cache);
  }
  cache.set(JSON.stringify(layer), output);
  if (cache.size > 32) {
    const oldest = cache.keys().next().value;
    if (oldest) cache.delete(oldest);
  }
  ctx.beginPath();
  ctx.rect(layer.x, layer.y, layer.width, layer.height);
  ctx.clip();
  ctx.drawImage(output, rect.x, rect.y);
}

const redactionCanvases = new WeakMap<BaseSource, Map<string, HTMLCanvasElement>>();

function pixelate(
  out: CanvasRenderingContext2D,
  original: HTMLCanvasElement,
  layer: RedactLayer,
  sampleRect: PhysicalRect,
  targetRect: PhysicalRect,
  source: BaseSource,
): void {
  const input = original.getContext("2d")?.getImageData(0, 0, original.width, original.height);
  if (!input) return;
  const output = out.createImageData(out.canvas.width, out.canvas.height);
  const block = Math.max(1, Math.round(layer.strength));
  const gridX = Math.floor((targetRect.x - source.originX) / block) * block + source.originX;
  const gridY = Math.floor((targetRect.y - source.originY) / block) * block + source.originY;
  for (let blockY = gridY; blockY < targetRect.y + targetRect.height; blockY += block) {
    for (let blockX = gridX; blockX < targetRect.x + targetRect.width; blockX += block) {
      const sampleLeft = blockX - sampleRect.x;
      const sampleTop = blockY - sampleRect.y;
      const sampleRight = sampleLeft + block;
      const sampleBottom = sampleTop + block;
      const average = [0, 0, 0];
      let count = 0;
      for (let by = sampleTop; by < sampleBottom; by += 1) {
        for (let bx = sampleLeft; bx < sampleRight; bx += 1) {
          if (bx < 0 || by < 0 || bx >= original.width || by >= original.height) continue;
          const offset = (by * original.width + bx) * 4;
          average[0] += input.data[offset] ?? 0;
          average[1] += input.data[offset + 1] ?? 0;
          average[2] += input.data[offset + 2] ?? 0;
          count += 1;
        }
      }
      const left = Math.max(blockX, targetRect.x);
      const top = Math.max(blockY, targetRect.y);
      const right = Math.min(blockX + block, targetRect.x + targetRect.width);
      const bottom = Math.min(blockY + block, targetRect.y + targetRect.height);
      for (let by = top; by < bottom; by += 1) {
        for (let bx = left; bx < right; bx += 1) {
          const targetX = bx - targetRect.x;
          const targetY = by - targetRect.y;
          const target = (targetY * output.width + targetX) * 4;
          output.data[target] = Math.round(average[0] / Math.max(1, count));
          output.data[target + 1] = Math.round(average[1] / Math.max(1, count));
          output.data[target + 2] = Math.round(average[2] / Math.max(1, count));
          output.data[target + 3] = 255;
        }
      }
    }
  }
  out.putImageData(output, 0, 0);
}

function blur(
  out: CanvasRenderingContext2D,
  original: HTMLCanvasElement,
  strength: number,
  margin: number,
  width: number,
  height: number,
): void {
  const expanded = document.createElement("canvas");
  expanded.width = original.width;
  expanded.height = original.height;
  const expandedContext = expanded.getContext("2d");
  if (!expandedContext) return;
  expandedContext.drawImage(original, 0, 0);
  const blurred = document.createElement("canvas");
  blurred.width = expanded.width;
  blurred.height = expanded.height;
  const blurredContext = blurred.getContext("2d");
  if (!blurredContext) return;
  blurredContext.filter = `blur(${strength}px)`;
  blurredContext.drawImage(expanded, 0, 0);
  out.drawImage(blurred, margin, margin, width, height, 0, 0, width, height);
}

function drawStep(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  number: number,
  radius: number,
  color: string,
): void {
  ctx.fillStyle = color;
  ctx.strokeStyle = "rgba(255,255,255,.72)";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.arc(x, y, radius, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();
  ctx.fillStyle = color.toUpperCase() === "#FFFFFF" || color.toUpperCase() === "#F59E0B"
    ? "#18181B"
    : "#FFFFFF";
  ctx.font = `700 ${radius * 1.1}px Inter, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(String(number), x, y);
}

function distance(start: PhysicalPoint, end: PhysicalPoint): number {
  return Math.hypot(start.x - end.x, start.y - end.y);
}

function assertNever(value: never): never {
  throw new Error(`Unsupported annotation layer: ${JSON.stringify(value)}`);
}
