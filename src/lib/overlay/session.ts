import type { BaseFrame } from "$lib/annotations/render";
import type { MonitorGeometry, VirtualBounds } from "./geometry";

export type OverlaySession = {
  session: string;
  bounds: VirtualBounds;
  monitors: MonitorGeometry[];
};

export type ImageTiming = { label: string; loadMs: number; paintMs: number };
export type LoadedFrames = { timings: ImageTiming[]; frames: BaseFrame[] };

const FRAME_BAND_BYTES = 2 * 1024 * 1024;

export function frameBands(monitor: MonitorGeometry): [number, number][] {
  const rowsPerBand = Math.max(1, Math.floor(FRAME_BAND_BYTES / (monitor.width * 4)));
  const bands: [number, number][] = [];
  for (let start = 0; start < monitor.height; start += rowsPerBand) {
    bands.push([start, Math.min(monitor.height, start + rowsPerBand)]);
  }
  return bands;
}

export function frameUrl(
  sessionId: string,
  monitor: MonitorGeometry,
  startRow: number,
  endRow: number,
): string {
  const query = `s=${encodeURIComponent(sessionId)}&y0=${startRow}&y1=${endRow}`;
  return `http://frame.localhost/${encodeURIComponent(monitor.label)}?${query}`;
}

export async function loadFrames(
  session: OverlaySession,
  arrivedAt: number,
  dependencies: {
    findCanvas: (label: string) => HTMLCanvasElement | null;
    isCurrent: () => boolean;
    now: () => number;
    fetchFrame: (url: string) => Promise<Response>;
  },
): Promise<LoadedFrames> {
  const results = await Promise.all(
    session.monitors.map(async (monitor) => {
      const canvas = dependencies.findCanvas(monitor.label);
      if (!canvas) throw new Error(`Frame canvas for ${monitor.label} was not mounted`);
      const context = canvas.getContext("2d", { alpha: false });
      if (!context) throw new Error(`Canvas 2D is unavailable for ${monitor.label}`);
      let loadedAt = arrivedAt;
      await Promise.all(frameBands(monitor).map(async ([startRow, endRow]) => {
        const url = frameUrl(session.session, monitor, startRow, endRow);
        const response = await dependencies.fetchFrame(url);
        if (!response.ok) throw new Error(`Frame load failed for ${monitor.label}`);
        const buffer = await response.arrayBuffer();
        loadedAt = Math.max(loadedAt, dependencies.now());
        if (!dependencies.isCurrent()) throw new Error("Stale capture session");
        const rows = endRow - startRow;
        if (buffer.byteLength !== monitor.width * rows * 4) {
          throw new Error(`Frame band for ${monitor.label} has an unexpected size`);
        }
        const imageData = new ImageData(new Uint8ClampedArray(buffer), monitor.width, rows);
        context.putImageData(imageData, 0, startRow);
      }));
      return {
        timing: {
          label: monitor.label,
          loadMs: loadedAt - arrivedAt,
          paintMs: dependencies.now() - arrivedAt,
        } satisfies ImageTiming,
        frame: { ...monitor, canvas } satisfies BaseFrame,
      };
    }),
  );
  return {
    timings: results.map((result) => result.timing),
    frames: results.map((result) => result.frame),
  };
}
