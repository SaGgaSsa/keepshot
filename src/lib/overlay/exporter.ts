import { invoke } from "@tauri-apps/api/core";
import { renderLayers, type BaseSource } from "$lib/annotations/render";
import {
  isRedact,
  moveLayer,
  type AnnotationDocument,
  type Layer,
} from "$lib/annotations/model";
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
  historyId: string | null,
): Promise<void> {
  const composite = layers.length ? rasterizeAnnotations(rect, layers, baseSource) : null;
  const redactions = layers.filter(isRedact);
  const redactBase = redactions.length
    ? rasterizeAnnotations(rect, redactions, baseSource)
    : null;
  const document: AnnotationDocument = {
    version: 2,
    layers: layers
      .filter((layer) => !isRedact(layer))
      .map((layer) => moveLayer(layer, -rect.x, -rect.y)),
  };
  const metadata = new TextEncoder().encode(JSON.stringify({
    rect,
    document,
    hasComposite: composite !== null,
    hasRedactBase: redactBase !== null,
    historyId,
  }));
  if (metadata.byteLength > 0xffffffff) throw new Error("Export metadata is too large");
  const compositeLength = composite?.byteLength ?? 0;
  const redactLength = redactBase?.byteLength ?? 0;
  const body = new Uint8Array(4 + metadata.byteLength + compositeLength + redactLength);
  new DataView(body.buffer).setUint32(0, metadata.byteLength, true);
  body.set(metadata, 4);
  let offset = 4 + metadata.byteLength;
  if (composite) {
    body.set(composite, offset);
    offset += composite.byteLength;
  }
  if (redactBase) body.set(redactBase, offset);
  await invoke(command, body);
}
