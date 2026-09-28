<script lang="ts">
  import type { AnnotationTool, RedactMode } from "$lib/annotations/model";
  let {
    position,
    tool,
    color,
    strokeWidth,
    redactMode,
    showRedactMode,
    canUndo,
    canRedo,
    busy,
    notice,
    onTool,
    onColor,
    onWidth,
    onRedactMode,
    onUndo,
    onRedo,
    onCopy,
    onSave,
    onClose,
  }: {
    position: { x: number; y: number; width: number; height: number };
    tool: AnnotationTool;
    color: string;
    strokeWidth: number;
    redactMode: RedactMode;
    showRedactMode: boolean;
    canUndo: boolean;
    canRedo: boolean;
    busy: boolean;
    notice: string;
    onTool: (tool: AnnotationTool) => void;
    onColor: (color: string) => void;
    onWidth: (width: number) => void;
    onRedactMode: (mode: RedactMode) => void;
    onUndo: () => void;
    onRedo: () => void;
    onCopy: () => void;
    onSave: () => void;
    onClose: () => void;
  } = $props();
  let paletteOpen = $state(false);
  const colors = ["#EF4444", "#F59E0B", "#10B981", "#0EA5E9", "#8B5CF6", "#FFFFFF", "#18181B"];
  const tools: { id: AnnotationTool; title: string; path: string }[] = [
    { id: "select", title: "Select (V)", path: "M5 3 16 12 11 13 14 18 11 19 8 14 5 18Z" },
    { id: "arrow", title: "Arrow (A)", path: "M4 15 15 4M7 4h8v8" },
    { id: "line", title: "Line (L)", path: "M4 16 16 4" },
    { id: "rect", title: "Rectangle (R)", path: "M4 5h12v10H4z" },
    { id: "ellipse", title: "Ellipse (E)", path: "M10 4a6 5 0 1 0 0 10 6 5 0 1 0 0-10Z" },
    { id: "text", title: "Text (T)", path: "M4 5h12M10 5v11m-3 0h6" },
    { id: "marker", title: "Marker (M)", path: "M4 15 14 5l2 2L6 17H4zM11 8l2 2" },
    { id: "redact", title: "Redact (B)", path: "M4 4h12v12H4zM7 7l6 6m0-6-6 6" },
    {
      id: "step",
      title: "Step (N)",
      path: "M10 3a7 7 0 1 0 0 14 7 7 0 1 0 0-14ZM10 6v8m-2-6 2-2",
    },
  ];
</script>

<div
  class="toolbar"
  role="toolbar"
  aria-label="Annotation tools"
  tabindex="-1"
  class:working={busy}
  style={`left:${position.x}px;top:${position.y}px;width:${position.width}px;height:${position.height}px`}
  onpointerdown={(event) => event.stopPropagation()}
  onpointerup={(event) => event.stopPropagation()}
>
  {#if notice}<div class="notice" role="status">{notice}</div>{/if}
  <div class="group tools">
    {#each tools as item (item.id)}
      <button
        class:active={tool === item.id}
        class="tool"
        title={item.title}
        aria-label={item.title}
        aria-pressed={tool === item.id}
        onclick={() => onTool(item.id)}
      >
        <svg viewBox="0 0 20 20" aria-hidden="true"><path d={item.path} /></svg>
      </button>
    {/each}
  </div>
  <span class="divider"></span>
  <div class="group properties">
    <div class="color-wrap">
      <button
        class="color-button"
        title="Annotation color"
        aria-label="Choose annotation color"
        onclick={() => paletteOpen = !paletteOpen}
      >
        <span class="swatch" style={`background:${color}`}></span>
        <svg viewBox="0 0 12 12"><path d="m3 4 3 3 3-3" /></svg>
      </button>
      {#if paletteOpen}
        <div
          class="palette"
          role="group"
          aria-label="Annotation colors"
          onpointerdown={(event) => event.stopPropagation()}
        >
          {#each colors as value (value)}
            <button
              class:selected={value === color}
              class="palette-swatch"
              style={`--swatch:${value}`}
              title={value}
              aria-label={value}
              onclick={() => { onColor(value); paletteOpen = false; }}
            ></button>
          {/each}
        </div>
      {/if}
    </div>
    <div class="sizes" aria-label="Stroke width">
      {#each [
        { label: "S", value: 3 },
        { label: "M", value: 5 },
        { label: "L", value: 8 },
      ] as size (size.value)}
        <button
          class:selected={strokeWidth === size.value}
          title={`Size ${size.label}`}
          aria-pressed={strokeWidth === size.value}
          onclick={() => onWidth(size.value)}
        >{size.label}</button>
      {/each}
    </div>
    {#if showRedactMode}
      <div class="group redact-modes" aria-label="Redaction mode">
        <button
          class:chosen={redactMode === "pixelate"}
          onclick={() => onRedactMode("pixelate")}
        >Pixelate</button>
        <button class:chosen={redactMode === "blur"} onclick={() => onRedactMode("blur")}>Blur</button>
      </div>
    {/if}
  </div>
  <span class="divider"></span>
  <div class="group">
    <button class="tool" title="Undo (Ctrl+Z)" aria-label="Undo" disabled={!canUndo} onclick={onUndo}>
      <svg viewBox="0 0 20 20"><path d="M7 6 3.5 9.5 7 13M4 9.5h7a5 5 0 0 1 5 5" /></svg>
    </button>
    <button class="tool" title="Redo (Ctrl+Y)" aria-label="Redo" disabled={!canRedo} onclick={onRedo}>
      <svg viewBox="0 0 20 20"><path d="m13 6 3.5 3.5L13 13m3-3.5h-7a5 5 0 0 0-5 5" /></svg>
    </button>
  </div>
  <span class="divider"></span>
  <div class="group actions">
    <button class="action copy" title="Copy (Ctrl+C / Enter)" disabled={busy} onclick={onCopy}>
      <svg viewBox="0 0 20 20">
        <rect x="7" y="7" width="9" height="10" rx="1.5" />
        <path d="M12 4H5.5A1.5 1.5 0 0 0 4 5.5V13" />
      </svg>
      <span>Copy</span>
    </button>
    <button class="action" title="Save (Ctrl+S)" disabled={busy} onclick={onSave}>
      <svg viewBox="0 0 20 20">
        <path d="M4 3.5h10l2 2V16a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1zM7 3.5v5h6v-5M7 17v-5h6v5" />
      </svg>
    </button>
    <button class="action close" title="Close (Esc)" disabled={busy} onclick={onClose}>
      <svg viewBox="0 0 20 20"><path d="m5 5 10 10M15 5 5 15" /></svg>
    </button>
  </div>
</div>

<style>
  .toolbar {
    position: absolute;
    z-index: 5;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    box-sizing: border-box;
    min-height: 52px;
    padding: 6px;
    border: 1px solid rgba(255,255,255,.1);
    border-radius: var(--radius-xl);
    background: var(--color-glass);
    backdrop-filter: blur(16px) saturate(180%);
    box-shadow: var(--shadow-l1);
    cursor: default;
    animation: toolbar-enter 140ms cubic-bezier(0.2, 0, 0, 1) both;
  }
  .toolbar.working {
    opacity: .72;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .tools {
    gap: 1px;
  }
  .divider {
    width: 1px;
    height: 20px;
    margin: 0 3px;
    background: var(--color-divider);
  }
  button {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    border: 1px solid transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color var(--motion-fast), color var(--motion-fast),
      border-color var(--motion-fast);
  }
  button:hover:not(:disabled) {
    color: #fff;
    background: rgba(255,255,255,.08);
  }
  button:disabled {
    opacity: .35;
    cursor: default;
  }
  .tool {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: transparent;
  }
  .tool.active {
    border-color: rgba(99,102,241,.5);
    background: var(--color-primary-soft);
    color: white;
    box-shadow: inset 0 0 8px rgba(99,102,241,.16);
  }
  svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .properties {
    gap: 6px;
  }
  .color-wrap {
    position: relative;
  }
  .color-button {
    height: 36px;
    gap: 4px;
    padding: 0 7px;
    border-radius: var(--radius-md);
    background: transparent;
  }
  .color-button svg {
    width: 12px;
    height: 12px;
  }
  .swatch {
    width: 17px;
    height: 17px;
    border: 1px solid rgba(255,255,255,.32);
    border-radius: 50%;
  }
  .palette {
    position: absolute;
    z-index: 6;
    left: 0;
    bottom: calc(100% + 8px);
    display: grid;
    grid-template-columns: repeat(4, 20px);
    gap: 9px;
    padding: 12px;
    border: 1px solid rgba(255,255,255,.14);
    border-radius: 12px;
    background: var(--color-glass-level-2);
    backdrop-filter: blur(24px) saturate(200%);
    box-shadow: var(--shadow-l2);
    transform-origin: bottom left;
    animation: palette-enter 120ms cubic-bezier(0.2, 0, 0, 1) both;
  }
  .palette-swatch {
    width: 20px;
    height: 20px;
    border: 2px solid rgba(255,255,255,.24);
    border-radius: 50%;
    background: var(--swatch);
  }
  .palette-swatch.selected {
    outline: 2px solid white;
    outline-offset: 2px;
  }
  .sizes {
    display: flex;
    padding: 2px;
    border-radius: 7px;
    background: rgba(255,255,255,.07);
  }
  .sizes button {
    width: 25px;
    height: 27px;
    border-radius: 5px;
    background: transparent;
    font: 600 11px var(--font-sans);
  }
  .sizes button.selected {
    background: rgba(99,102,241,.34);
    color: white;
  }
  .redact-modes {
    gap: 2px;
    padding: 2px;
    border-radius: 7px;
    background: rgba(255,255,255,.07);
  }
  .redact-modes button {
    height: 27px;
    padding: 0 7px;
    border-radius: 5px;
    background: transparent;
    font: 600 10px var(--font-sans);
  }
  .redact-modes button.chosen {
    background: rgba(99,102,241,.34);
    color: white;
  }
  .actions {
    gap: 3px;
  }
  .action {
    height: 36px;
    min-width: 36px;
    gap: 5px;
    padding: 0 9px;
    border-radius: var(--radius-md);
    background: rgba(255,255,255,.07);
    color: var(--color-text);
  }
  .action svg {
    width: 16px;
    height: 16px;
  }
  .action.copy {
    border-color: rgba(99,102,241,.5);
    background: var(--color-primary);
    color: white;
    font: 600 12px var(--font-sans);
  }
  .action.copy:hover:not(:disabled) {
    background: var(--color-primary-hover);
  }
  .action.close:hover:not(:disabled) {
    background: rgba(239,68,68,.15);
    color: #fca5a5;
  }
  .notice {
    position: absolute;
    right: 0;
    bottom: calc(100% + 7px);
    max-width: 320px;
    overflow: hidden;
    padding: 7px 10px;
    border: 1px solid rgba(239,68,68,.4);
    border-radius: var(--radius-md);
    background: rgba(24,24,27,.94);
    color: #fecaca;
    font: 11px/1.4 var(--font-sans);
    text-overflow: ellipsis;
    white-space: nowrap;
    box-shadow: var(--shadow-l1);
  }

  @keyframes toolbar-enter {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes palette-enter {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
