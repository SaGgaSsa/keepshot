<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  type HistoryItem = {
    id: string;
    createdAt: string;
    updatedAt: string;
    width: number;
    height: number;
    thumbUrl: string;
  };

  let items = $state<HistoryItem[]>([]);
  let selectedIndex = $state(0);
  let errorMessage = $state("");
  const relativeTime = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

  async function refresh(): Promise<void> {
    try {
      items = await invoke<HistoryItem[]>("history_list");
      selectedIndex = Math.min(selectedIndex, Math.max(0, items.length - 1));
      errorMessage = "";
    } catch (error) {
      errorMessage = String(error);
    }
  }

  function updatedLabel(value: string): string {
    const seconds = Math.round((Date.parse(value) - Date.now()) / 1000);
    const absolute = Math.abs(seconds);
    if (absolute < 60) return relativeTime.format(seconds, "second");
    const minutes = Math.round(seconds / 60);
    if (absolute < 3600) return relativeTime.format(minutes, "minute");
    const hours = Math.round(seconds / 3600);
    if (absolute < 86400) return relativeTime.format(hours, "hour");
    const days = Math.round(seconds / 86400);
    if (absolute < 604800) return relativeTime.format(days, "day");
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    }).format(new Date(value));
  }

  async function copy(item: HistoryItem, closeAfter = false): Promise<void> {
    try {
      await invoke("history_copy", { id: item.id });
      if (closeAfter) await invoke("history_close");
    } catch (error) {
      errorMessage = String(error);
    }
  }

  async function save(item: HistoryItem): Promise<void> {
    try {
      await invoke<string>("history_save", { id: item.id });
      errorMessage = "";
    } catch (error) {
      errorMessage = String(error);
    }
  }

  async function edit(item: HistoryItem): Promise<void> {
    try {
      await invoke("history_edit", { id: item.id });
    } catch (error) {
      errorMessage = String(error);
    }
  }

  async function remove(item: HistoryItem): Promise<void> {
    try {
      await invoke("history_delete", { id: item.id });
      await refresh();
    } catch (error) {
      errorMessage = String(error);
    }
  }


  function scrollSelectedIntoView() {
    void tick().then(() => {
      document.querySelectorAll("article")[selectedIndex]?.scrollIntoView({ block: "nearest" });
    });
  }
  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === "Enter" && event.target instanceof HTMLElement
      && event.target.closest("button")) return;
    if (event.key === "Escape") {
      event.preventDefault();
      void invoke("history_close");
      return;
    }
    if (items.length === 0) return;
    const current = items[selectedIndex];
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex = Math.min(items.length - 1, selectedIndex + 1);
      scrollSelectedIntoView();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex = Math.max(0, selectedIndex - 1);
      scrollSelectedIntoView();
    } else if (event.key === "Enter" && current) {
      event.preventDefault();
      void copy(current, true);
    } else if (event.key.toLowerCase() === "e" && current) {
      event.preventDefault();
      void edit(current);
    } else if (event.key === "Delete" && current) {
      event.preventDefault();
      void remove(current);
    }
  }

  onMount(() => {
    const currentWindow = getCurrentWebviewWindow();
    let unlistenShown: (() => void) | undefined;
    let disposed = false;
    void currentWindow.listen("history:shown", () => void refresh()).then((unlisten) => {
      if (disposed) unlisten();
      else unlistenShown = unlisten;
    });
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("focus", refresh);
    void refresh();
    return () => {
      disposed = true;
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("focus", refresh);
      unlistenShown?.();
    };
  });
</script>

<svelte:head>
  <title>KeepShot History</title>
</svelte:head>

<main class="panel" aria-label="Capture history">
  <header>
    <div>
      <h1>History</h1>
      <p>{items.length} captures</p>
    </div>
    <button
      class="close"
      title="Close (Esc)"
      aria-label="Close history"
      onclick={() => void invoke("history_close")}
    >
      <svg viewBox="0 0 20 20" aria-hidden="true">
        <path d="m5 5 10 10M15 5 5 15" />
      </svg>
    </button>
  </header>

  {#if errorMessage}
    <div class="error" role="status">{errorMessage}</div>
  {/if}

  {#if items.length === 0}
    <section class="empty">
      <div class="empty-icon" aria-hidden="true">⌑</div>
      <h2>No captures yet</h2>
      <p>Confirmed captures will appear here.</p>
    </section>
  {:else}
    <section class="grid" aria-label="Recent captures">
      {#each items as item, index (item.id)}
        <article
          class:active={selectedIndex === index}
          onmouseenter={() => selectedIndex = index}
          onfocusin={() => selectedIndex = index}
        >
          <button
            class="preview"
            aria-label="Copy capture from {updatedLabel(item.updatedAt)}"
            onclick={() => copy(item, true)}
          >
            <img src={item.thumbUrl} alt="" draggable="false" />
          </button>
          <div class="details">
            <span>{updatedLabel(item.updatedAt)}</span>
            <span class="dimensions">{item.width}×{item.height}</span>
          </div>
          <div class="actions" role="group" aria-label="Capture actions">
            <button title="Edit" aria-label="Edit capture" onclick={() => edit(item)}>
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M4 13.5V16h2.5L15 7.5 12.5 5 4 13.5ZM14 3.5l2.5 2.5" />
              </svg>
            </button>
            <button title="Copy" aria-label="Copy capture" onclick={() => copy(item)}>
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <rect x="7" y="7" width="9" height="10" rx="1.5" />
                <path d="M13 7V4.5A1.5 1.5 0 0 0 11.5 3h-7A1.5 1.5 0 0 0 3 4.5v8A1.5 1.5 0 0 0 4.5 14H7" />
              </svg>
            </button>
            <button title="Save" aria-label="Save capture" onclick={() => save(item)}>
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M10 3v9m0 0 3-3m-3 3L7 9m-3 5v2.5A.5.5 0 0 0 4.5 17h11a.5.5 0 0 0 .5-.5V14" />
              </svg>
            </button>
            <button class="delete" title="Delete" aria-label="Delete capture" onclick={() => remove(item)}>
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M4 6h12m-10 0 .7 10h6.6L14 6m-6-2h4l1 2M8 9v4m4-4v4" />
              </svg>
            </button>
          </div>
        </article>
      {/each}
    </section>
  {/if}
</main>

<style>
  :global(html), :global(body) {
    width: 100%;
    height: 100%;
    margin: 0;
    overflow: hidden;
    background: transparent;
  }

  /* The panel itself has no background: only the title and the capture cards float over the desktop. */
  .panel {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    overflow: hidden;
    padding: 8px 10px;
    color: var(--color-text);
    outline: none;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
    padding: 0 2px;
  }

  h1 {
    margin: 0;
    color: #fff;
    font-size: 20px;
    font-weight: 600;
    line-height: 28px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8), 0 0 12px rgba(0, 0, 0, 0.6);
  }

  header p {
    margin: 1px 0 0;
    color: #e4e4e7;
    font-size: 12px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
  }

  .close {
    border-color: var(--color-glass-rim);
    background: var(--color-glass);
    backdrop-filter: blur(16px) saturate(180%);
  }

  button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  button:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }

  button svg {
    width: 17px;
    height: 17px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 1.6;
  }

  .grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: calc(100% - 50px);
    overflow-y: auto;
    padding: 2px 2px 12px;
    scrollbar-width: none;
  }

  article {
    position: relative;
    flex: none;
    min-width: 0;
    padding: 8px;
    border: 1px solid var(--color-glass-rim);
    border-radius: 12px;
    background: var(--color-glass);
    backdrop-filter: blur(16px) saturate(180%);
    box-shadow: var(--shadow-l1);
  }

  .preview {
    display: flex;
    width: 100%;
    height: 180px;
    overflow: hidden;
    padding: 0;
    border: 1px solid var(--color-divider);
    border-radius: 8px;
    background: var(--color-surface-lowest);
  }

  article.active .preview, .preview:hover {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 1px rgba(99, 102, 241, 0.45);
  }

  .preview img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .details {
    display: flex;
    justify-content: space-between;
    gap: 6px;
    margin-top: 6px;
    color: var(--color-text-muted);
    font-size: 11px;
  }

  .dimensions {
    color: var(--color-text-variant);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .actions {
    position: absolute;
    top: 14px;
    right: 14px;
    display: flex;
    gap: 2px;
    padding: 3px;
    border: 1px solid var(--color-glass-rim);
    border-radius: 8px;
    background: rgba(24, 24, 27, 0.94);
    opacity: 0;
    transition: opacity 100ms ease;
  }

  article:hover .actions, article:focus-within .actions, article.active .actions {
    opacity: 1;
  }

  .actions button {
    width: 27px;
    height: 27px;
  }

  .actions svg {
    width: 15px;
    height: 15px;
  }

  .actions .delete:hover {
    background: rgba(239, 68, 68, 0.18);
    color: #ef4444;
  }

  .empty {
    display: grid;
    justify-items: center;
    align-content: center;
    padding: 28px 16px;
    border: 1px solid var(--color-glass-rim);
    border-radius: 12px;
    background: var(--color-glass);
    backdrop-filter: blur(16px) saturate(180%);
    box-shadow: var(--shadow-l1);
    color: var(--color-text-muted);
    text-align: center;
  }

  .empty-icon {
    display: grid;
    place-items: center;
    width: 58px;
    height: 58px;
    margin-bottom: 12px;
    border: 1px solid var(--color-divider);
    border-radius: 16px;
    background: var(--color-surface-low);
    color: var(--color-primary);
    font-size: 28px;
  }

  h2 {
    margin: 0 0 5px;
    color: var(--color-text);
    font-size: 14px;
    font-weight: 600;
  }

  .empty p, .error {
    margin: 0;
    font-size: 12px;
  }

  .error {
    margin: -8px 0 12px;
    color: var(--color-danger);
  }
</style>
