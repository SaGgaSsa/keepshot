import type { PhysicalPoint } from "$lib/overlay/geometry";
import type { Layer } from "./model";

export type RenderTransform = { originX: number; originY: number; scale: number };

export function renderLayers(
  ctx: CanvasRenderingContext2D,
  layers: readonly Layer[],
  transform: RenderTransform,
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
  ctx.lineCap = "round";
  ctx.lineJoin = "round";

  for (const layer of layers) {
    ctx.save();
    ctx.strokeStyle = layer.color;
    ctx.fillStyle = layer.color;
    ctx.lineWidth = layer.strokeWidth;

    switch (layer.type) {
      case "line":
        ctx.beginPath();
        ctx.moveTo(layer.start.x, layer.start.y);
        ctx.lineTo(layer.end.x, layer.end.y);
        ctx.stroke();
        break;
      case "arrow":
        drawArrow(ctx, layer.start, layer.end, layer.strokeWidth);
        break;
      case "rect":
        ctx.strokeRect(layer.x, layer.y, layer.width, layer.height);
        break;
      case "ellipse":
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
        ctx.font = `600 ${layer.fontSize}px Inter, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif`;
        ctx.textBaseline = "top";
        layer.text.split("\n").forEach((line, index) => {
          ctx.fillText(line, layer.x, layer.y + index * layer.fontSize * 1.25);
        });
        break;
      default:
        assertNever(layer);
    }

    ctx.restore();
  }

  ctx.restore();
}

function drawArrow(
  ctx: CanvasRenderingContext2D,
  start: PhysicalPoint,
  end: PhysicalPoint,
  width: number,
): void {
  const angle = Math.atan2(end.y - start.y, end.x - start.x);
  const headLength = Math.max(width * 3.5, 10);
  const baseX = end.x - Math.cos(angle) * headLength;
  const baseY = end.y - Math.sin(angle) * headLength;
  const half = headLength * 0.42;

  ctx.beginPath();
  ctx.moveTo(start.x, start.y);
  ctx.lineTo(baseX, baseY);
  ctx.stroke();

  ctx.beginPath();
  ctx.moveTo(end.x, end.y);
  ctx.lineTo(baseX - Math.sin(angle) * half, baseY + Math.cos(angle) * half);
  ctx.lineTo(baseX + Math.sin(angle) * half, baseY - Math.cos(angle) * half);
  ctx.closePath();
  ctx.fill();
}

function assertNever(value: never): never {
  throw new Error(`Unsupported annotation layer: ${JSON.stringify(value)}`);
}
