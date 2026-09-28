import type { Layer } from "./model";

// Layers are plain JSON data; a JSON round-trip also works on Svelte $state proxies,
// which structuredClone rejects.
function snapshot(layers: readonly Layer[]): Layer[] {
  return JSON.parse(JSON.stringify(layers)) as Layer[];
}

export class LayerHistory {
  private past: Layer[][] = [];
  private future: Layer[][] = [];
  canUndo = false;
  canRedo = false;

  push(layers: readonly Layer[]): void {
    this.past.push(snapshot(layers));
    this.future = [];
    this.updateFlags();
  }

  undo(current: readonly Layer[]): Layer[] | null {
    const previous = this.past.pop();
    if (!previous) return null;
    this.future.push(snapshot(current));
    this.updateFlags();
    return previous;
  }

  redo(current: readonly Layer[]): Layer[] | null {
    const next = this.future.pop();
    if (!next) return null;
    this.past.push(snapshot(current));
    this.updateFlags();
    return next;
  }

  reset(): void {
    this.past = [];
    this.future = [];
    this.updateFlags();
  }

  private updateFlags(): void {
    this.canUndo = this.past.length > 0;
    this.canRedo = this.future.length > 0;
  }
}
