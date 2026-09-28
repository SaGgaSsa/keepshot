import { invoke } from "@tauri-apps/api/core";
import { renderLayers, type BaseSource } from "$lib/annotations/render";
import type { Layer } from "$lib/annotations/model";
import type { PhysicalRect } from "./geometry";

export function rasterizeAnnotations(
  rect: PhysicalRect,
  layers: readonly Layer[],
  baseSource: BaseSource | null,
): Uint8Array {
  const canvas = document.createElement("canvas");
  canvas.width = rect.width;
  canvas.height = rect.height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Canvas 2D is unavailable");
  renderLayers(ctx, [...layers], {
    originX: rect.x,
    originY: rect.y,
    scale: 1,
  }, baseSource ?? undefined, rect);
  const pixels = ctx.getImageData(0, 0, rect.width, rect.height).data;
  return new Uint8Array(pixels.buffer);
}

export async function invokeSelectionAction(
  command: "copy_selection" | "save_selection",
  rect: PhysicalRect,
  layers: readonly Layer[],
  baseSource: BaseSource | null,
): Promise<void> {
  const body = layers.length
    ? rasterizeAnnotations(rect, layers, baseSource)
    : new Uint8Array(0);
  await invoke(command, body, {
    headers: { "x-keepshot-rect": JSON.stringify(rect) },
  });
}
