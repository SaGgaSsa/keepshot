<script lang="ts">
  import { reportError } from "$lib/reportError";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  type UpdateStatus =
    | { state: "idle" | "checking" | "upToDate" }
    | { state: "available"; version: string; notes: string | null; date: string | null }
    | { state: "downloading"; downloaded: number; total: number | null }
    | { state: "error"; message: string };
  type UpdateStatusView = {
    status: UpdateStatus;
    currentVersion: string;
    lastChecked: string | null;
  };

  let view = $state<UpdateStatusView | null>(null);
  let busy = $state(false);
  let error = $state("");

  async function refresh(): Promise<void> {
    try {
      view = await invoke<UpdateStatusView>("get_update_status");
    } catch (cause) {
      error = reportError("settings", cause);
    }
  }

  onMount(() => {
    void refresh();
    let unlisten: UnlistenFn | undefined;
    void listen<UpdateStatus>("updates:status", (event) => {
      if (view) view.status = event.payload;
      else void refresh();
    }).then((stop) => {
      unlisten = stop;
    }).catch((cause: unknown) => {
      error = reportError("settings", cause);
    });
    return () => unlisten?.();
  });

  async function check(): Promise<void> {
    busy = true;
    error = "";
    try {
      view = await invoke<UpdateStatusView>("check_for_updates");
    } catch (cause) {
      error = reportError("settings", cause);
      await refresh();
    } finally {
      busy = false;
    }
  }

  async function install(): Promise<void> {
    busy = true;
    error = "";
    try {
      await invoke("install_update");
    } catch (cause) {
      error = reportError("settings", cause);
      await refresh();
    } finally {
      busy = false;
    }
  }

  function checkedAgo(value: string | null): string {
    if (!value) return "not checked yet";
    const elapsed = Math.max(0, Date.now() - Date.parse(value));
    const minutes = Math.floor(elapsed / 60_000);
    if (minutes < 1) return "checked just now";
    if (minutes < 60) return `checked ${minutes} min ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `checked ${hours} hr ago`;
    return `checked ${Math.floor(hours / 24)} days ago`;
  }

  function megabytes(bytes: number): string {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<section class="panel updates">
  <div class="section-title"><span>04</span><h2>Updates</h2></div>
  {#if view}
    <p class="version">KeepShot v{view.currentVersion} <span>· {checkedAgo(view.lastChecked)}</span></p>
    {#if view.status.state === "checking"}
      <p class="status">Checking for updates…</p>
    {:else if view.status.state === "available"}
      <p class="status">Version {view.status.version} is available</p>
      {#if view.status.notes}
        <details class="notes"><summary>Release notes</summary><p>{view.status.notes}</p></details>
      {/if}
    {:else if view.status.state === "downloading"}
      <p class="status">Downloading update…</p>
      <progress
        max={view.status.total ?? 0}
        value={view.status.downloaded}
        aria-label="Update download progress"
      ></progress>
      <p class="progress-label">
        {megabytes(view.status.downloaded)}{view.status.total
          ? ` / ${megabytes(view.status.total)}`
          : ""}
      </p>
    {:else if view.status.state === "upToDate"}
      <p class="status">Up to date</p>
    {:else if view.status.state === "error"}
      <p class="error">{view.status.message}</p>
    {/if}
    <div class="actions">
      <button class="secondary" disabled={busy} onclick={check}>Check for updates</button>
      {#if view.status.state === "available"}
        <button class="primary" disabled={busy} onclick={install}>Install and restart</button>
      {/if}
    </div>
  {:else}
    <p class="status">Loading update status…</p>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
  .panel {
    margin: 0 0 12px;
    padding: 15px 16px;
    border: 1px solid var(--color-glass-ambient);
    border-radius: 12px;
    background: var(--color-settings-panel);
    box-shadow: var(--shadow-l1);
  }
  :global(html[data-material="mica"]) .panel {
    border-color: var(--color-glass-rim);
    background: var(--color-settings-panel-mica);
    backdrop-filter: blur(18px) saturate(150%);
  }
  .section-title {
    display: flex;
    align-items: center;
    gap: 9px;
    margin-bottom: 11px;
  }
  .section-title > span {
    color: #818cf8;
    font: 10px var(--font-mono);
  }
  h2 {
    margin: 0;
    color: var(--color-text-strong);
    font-size: 13px;
    font-weight: 600;
  }
  p {
    margin: 0 0 9px;
    font-size: 11px;
  }
  .version {
    color: var(--color-text);
    font-weight: 600;
  }
  .version span, .status, .progress-label {
    color: var(--color-text-muted);
    font-weight: 400;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
    margin-top: 12px;
  }
  button {
    min-height: 30px;
    padding: 0 10px;
    border: 1px solid var(--color-divider);
    border-radius: 7px;
    font-family: inherit;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background-color var(--motion-fast), color var(--motion-fast);
  }
  button:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .secondary {
    background: var(--color-hover);
    color: var(--color-text);
  }
  .secondary:hover:not(:disabled) {
    background: var(--color-surface-high);
  }
  .primary {
    border-color: var(--color-primary);
    background: var(--color-primary);
    color: #fff;
  }
  .primary:hover:not(:disabled) {
    background: var(--color-primary-hover);
  }
  .error {
    color: var(--color-danger);
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .notes {
    margin: 7px 0;
    color: var(--color-text-variant);
    font-size: 11px;
  }
  .notes summary {
    color: var(--color-primary-hover);
    cursor: pointer;
  }
  .notes p {
    margin: 7px 0 0;
    white-space: pre-wrap;
  }
  progress {
    width: 100%;
    height: 6px;
    accent-color: var(--color-primary);
  }
  .progress-label {
    margin-top: 4px;
    font: 10px var(--font-mono);
  }
</style>
