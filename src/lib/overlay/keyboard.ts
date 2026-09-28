import type { AnnotationTool } from "$lib/annotations/model";

export type KeyboardActions = {
  editingText: boolean;
  selectedLayer: boolean;
  hasSelection: boolean;
  busy: boolean;
  finishTextEdit: (cancel?: boolean) => void;
  setTool: (tool: AnnotationTool) => void;
  undo: () => void;
  redo: () => void;
  deleteSelection: () => void;
  clearSelection: () => void;
  close: () => void;
  copy: () => void;
  save: () => void;
  selectCursorMonitor: () => void;
};

const tools: Record<string, AnnotationTool> = {
  v: "select",
  a: "arrow",
  l: "line",
  r: "rect",
  e: "ellipse",
  t: "text",
  m: "marker",
  b: "redact",
  n: "step",
};

export function handleOverlayKeyDown(event: KeyboardEvent, actions: KeyboardActions): void {
  if (
    event.key === "Enter"
    && event.target instanceof HTMLElement
    && event.target.closest(".toolbar")
  ) return;

  if (actions.editingText) {
    if (event.key === "Escape") {
      event.preventDefault();
      actions.finishTextEdit(true);
    } else if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      actions.finishTextEdit();
    }
    return;
  }

  const key = event.key.toLowerCase();
  if (!(event.ctrlKey || event.metaKey) && !event.altKey && !event.repeat) {
    const tool = tools[key];
    if (tool) {
      actions.setTool(tool);
      return;
    }
  }
  if ((event.ctrlKey || event.metaKey) && key === "z") {
    event.preventDefault();
    if (event.shiftKey) actions.redo();
    else actions.undo();
    return;
  }
  if ((event.ctrlKey || event.metaKey) && key === "y") {
    event.preventDefault();
    actions.redo();
    return;
  }
  if ((key === "delete" || key === "backspace") && actions.selectedLayer) {
    event.preventDefault();
    actions.deleteSelection();
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    if (actions.selectedLayer) actions.clearSelection();
    else actions.close();
  } else if ((event.ctrlKey || event.metaKey) && key === "c") {
    event.preventDefault();
    actions.copy();
  } else if ((event.ctrlKey || event.metaKey) && key === "s") {
    event.preventDefault();
    actions.save();
  } else if (event.key === "Enter" && !event.repeat && !actions.busy) {
    event.preventDefault();
    if (actions.hasSelection) actions.copy();
    else actions.selectCursorMonitor();
  }
}
