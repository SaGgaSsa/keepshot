<script lang="ts">
  import { reportError } from "$lib/reportError";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Diagnostics from "$lib/settings/Diagnostics.svelte";
  import ShortcutRecorder from "$lib/settings/ShortcutRecorder.svelte";
  import Updates from "$lib/settings/Updates.svelte";

  type Settings = {
    captureShortcut: string;
    historyShortcut: string;
    saveFolder: string | null;
    onboardingDone: boolean;
  };
  type SettingsView = {
    settings: Settings;
    autostart: boolean;
    shortcutErrors: string[];
    printScreenConflict: boolean | null;
    defaultSaveFolder: string;
    windowMaterial: "mica" | "solid";
  };

  let view = $state<SettingsView | null>(null);
  let error = $state("");
  let captureError = $state("");
  let historyError = $state("");
  let printConflict = $state<boolean | null>(null);
  let busy = $state(false);

  async function refresh(): Promise<void> {
    try {
      view = await invoke<SettingsView>("get_settings_view");
      printConflict = view.printScreenConflict;
      document.documentElement.dataset.material = view.windowMaterial;
    } catch (cause) {
      error = message(cause);
    }
  }

  onMount(() => {
    // The window stays hidden until the first paint so it never shows WebView2's white default.
    void refresh().finally(() =>
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          invoke("settings_ready").catch((cause) => reportError("settings ready", cause));
        }),
      ),
    );
    const onFocus = () => void refresh();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  });

  async function recordingChanged(recording: boolean): Promise<void> {
    try {
      await invoke("suspend_shortcuts", { suspend: recording });
    } catch (cause) {
      error = message(cause);
    }
  }

  async function saveShortcut(kind: "capture" | "history", shortcut: string): Promise<void> {
    error = "";
    captureError = "";
    historyError = "";
    try {
      const settings = await invoke<Settings>("set_shortcut", { kind, value: shortcut });
      if (view) view.settings = settings;
      await refresh();
    } catch (cause) {
      if (kind === "capture") captureError = message(cause);
      else historyError = message(cause);
    }
  }

  async function setAutostart(event: Event): Promise<void> {
    const enabled = (event.currentTarget as HTMLInputElement).checked;
    try {
      await invoke("set_autostart", { enabled });
      if (view) view.autostart = enabled;
    } catch (cause) {
      error = message(cause);
      await refresh();
    }
  }

  async function chooseFolder(): Promise<void> {
    try {
      const folder = await invoke<string | null>("pick_save_folder");
      if (folder && view) view.settings.saveFolder = folder;
    } catch (cause) {
      error = message(cause);
    }
  }

  async function resetFolder(): Promise<void> {
    try {
      await invoke("reset_save_folder");
      if (view) view.settings.saveFolder = null;
    } catch (cause) {
      error = message(cause);
    }
  }

  async function openSaveFolder(): Promise<void> {
    try {
      await invoke("open_save_folder");
    } catch (cause) {
      error = message(cause);
    }
  }

  async function completeOnboarding(): Promise<void> {
    busy = true;
    try {
      await invoke("complete_onboarding");
      if (view) view.settings.onboardingDone = true;
    } catch (cause) {
      error = message(cause);
    } finally {
      busy = false;
    }
  }

  async function checkPrintScreen(): Promise<void> {
    try {
      printConflict = await invoke<boolean | null>("check_print_screen");
    } catch (cause) {
      error = message(cause);
    }
  }

  async function openKeyboardSettings(): Promise<void> {
    try {
      await invoke("open_keyboard_settings");
    } catch (cause) {
      error = message(cause);
    }
  }

  function message(cause: unknown): string {
    return reportError("settings", cause);
  }

  function shortPath(path: string): string {
    if (path.length < 54) return path;
    return `…${path.slice(-51)}`;
  }

  function shortcutTokens(shortcut: string): string[] {
    return shortcut.split("+").map((token) =>
      token === "PrintScreen" ? "Print Screen" : token === "Super" ? "Win" : token,
    );
  }

  function usesPrintScreen(shortcut: string): boolean {
    return shortcut.toLowerCase().includes("printscreen");
  }
</script>

<svelte:head>
  <title>KeepShot Settings</title>
  <meta name="description" content="Configure KeepShot shortcuts and saving." />
</svelte:head>

<main>
  <header>
    <img class="brand-mark" src="/app-icon.svg" alt="" aria-hidden="true" draggable="false" />
    <div>
      <p class="eyebrow">KEEPSHOT</p>
      <h1>Settings</h1>
    </div>
    <span class="status"><i></i> Running in tray</span>
  </header>

  {#if error}<p class="error-banner" role="alert">{error}</p>{/if}
  {#if !view}
    <p class="loading">Loading settings…</p>
  {:else}
    {#if !view.settings.onboardingDone}
      <section class="onboarding panel">
        <div class="section-title"><span>01</span><h2>Welcome to KeepShot</h2></div>
        <p>KeepShot stays in your system tray. Use these shortcuts to capture your screen or open History.</p>
        <div class="onboarding-shortcuts">
          <span>Capture</span>
          <span class="keycaps">
            {#each shortcutTokens(view.settings.captureShortcut) as token, index (index)}
              {#if index > 0}<b>+</b>{/if}<kbd>{token}</kbd>
            {/each}
          </span>
          <span>History</span>
          <span class="keycaps">
            {#each shortcutTokens(view.settings.historyShortcut) as token, index (index)}
              {#if index > 0}<b>+</b>{/if}<kbd>{token}</kbd>
            {/each}
          </span>
        </div>
        {#if usesPrintScreen(view.settings.captureShortcut) && printConflict === true}
          <div class="warning">
            <strong>Windows is using Print Screen to open Snipping Tool</strong>
            <p>
              Settings › Accessibility › Keyboard › turn off “Use the Print screen key to open
              screen capture”.
            </p>
            <div class="warning-actions">
              <button class="secondary" onclick={openKeyboardSettings}>Open keyboard settings</button>
              <button class="text-button" onclick={checkPrintScreen}>Check again</button>
            </div>
          </div>
        {:else if usesPrintScreen(view.settings.captureShortcut) && printConflict === false}
          <p class="success">✓ Print Screen is ready for KeepShot.</p>
        {/if}
        <button class="primary got-it" disabled={busy} onclick={completeOnboarding}>Got it</button>
      </section>
    {/if}

    <section class="panel">
      <div class="section-title"><span>01</span><h2>Shortcuts</h2></div>
      {#each view.shortcutErrors as shortcutError (shortcutError)}
        <p class="inline-error">{shortcutError}</p>
      {/each}
      <div class="shortcut-row">
        <div><strong>Capture</strong><small>Start a screen capture</small></div>
        <ShortcutRecorder
          value={view.settings.captureShortcut}
          onCommit={(value) => saveShortcut("capture", value)}
          onRecordingChange={recordingChanged}
        />
      </div>
      {#if captureError}<p class="inline-error">{captureError}</p>{/if}
      <div class="shortcut-row">
        <div><strong>History</strong><small>Open your recent captures</small></div>
        <ShortcutRecorder
          value={view.settings.historyShortcut}
          onCommit={(value) => saveShortcut("history", value)}
          onRecordingChange={recordingChanged}
        />
      </div>
      {#if historyError}<p class="inline-error">{historyError}</p>{/if}
      {#if usesPrintScreen(view.settings.captureShortcut) && printConflict === true}
        <div class="compact-warning">
          <span>Windows is using Print Screen to open Snipping Tool.</span>
          <button class="text-button" onclick={openKeyboardSettings}>Open keyboard settings</button>
        </div>
      {/if}
    </section>

    <section class="panel">
      <div class="section-title"><span>02</span><h2>Saving</h2></div>
      <div class="folder-row">
        <div class="folder-copy">
          <strong>Save folder</strong>
          <span title={view.settings.saveFolder ?? view.defaultSaveFolder}>
            {shortPath(view.settings.saveFolder ?? view.defaultSaveFolder)}
          </span>
        </div>
        <button class="secondary" onclick={chooseFolder}>Change…</button>
        <button class="secondary" onclick={openSaveFolder}>Open</button>
      </div>
      {#if view.settings.saveFolder}
        <button class="text-button reset" onclick={resetFolder}>Reset to default</button>
      {/if}
    </section>

    <section class="panel startup-panel">
      <div class="section-title"><span>03</span><h2>Startup</h2></div>
      <label class="toggle-row">
        <span>
          <strong>Start KeepShot with Windows</strong>
          <small>Keep shortcuts available after sign-in</small>
        </span>
        <input type="checkbox" checked={view.autostart} onchange={setAutostart} />
        <i class="switch"></i>
      </label>
    </section>

    <Updates />

    <details class="panel diagnostics">
      <summary>
        <span class="section-title"><span>05</span><h2>Diagnostics</h2></span>
        <span class="chevron">⌄</span>
      </summary>
      <Diagnostics />
    </details>
  {/if}
  <footer>Frames and history stay on this device.</footer>
</main>

<style>
  :global(html), :global(body) {
    min-width: 480px;
    min-height: 100%;
    margin: 0;
    background: var(--color-surface);
  }
  :global(html[data-material="mica"]),
  :global(html[data-material="mica"] body) {
    background: transparent;
  }
  main {
    width: min(520px, calc(100% - 36px));
    margin: 0 auto;
    padding: 24px 0 20px;
    color: var(--color-text);
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 20px;
  }
  .brand-mark {
    display: block;
    width: 40px;
    height: 40px;
    filter: drop-shadow(0 6px 14px #6366f140);
  }
  .eyebrow {
    margin: 0 0 3px;
    color: var(--color-text-muted);
    font: 600 9px/1.2 var(--font-mono);
    letter-spacing: 0.12em;
  }
  h1 {
    margin: 0;
    color: var(--color-text-strong);
    font-size: 18px;
    line-height: 24px;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-left: auto;
    color: var(--color-text-muted);
    font-size: 11px;
  }
  .status i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-success);
    box-shadow: 0 0 9px #10b98188;
  }
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
  .onboarding > p {
    margin: 0 0 12px;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .onboarding-shortcuts {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px 12px;
    align-items: center;
    color: var(--color-text-variant);
    font-size: 12px;
  }
  .keycaps {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .keycaps b {
    color: var(--color-text-muted);
    font-size: 9px;
    font-weight: 400;
  }
  kbd {
    padding: 4px 7px;
    border: 1px solid var(--color-glass-rim);
    border-radius: 4px;
    background: var(--color-shortcut-bg);
    color: var(--color-shortcut-text);
    font: 600 10px var(--font-mono);
  }
  button {
    border: 0;
    font: inherit;
    cursor: pointer;
    transition: background-color var(--motion-fast), color var(--motion-fast),
      border-color var(--motion-fast);
  }
  .primary, .secondary {
    min-height: 32px;
    padding: 0 11px;
    border-radius: 7px;
    font-size: 11px;
    font-weight: 600;
  }
  .primary {
    background: var(--color-primary);
    color: white;
  }
  .primary:hover {
    background: var(--color-primary-hover);
  }
  .primary:disabled {
    opacity: 0.6;
  }
  .secondary {
    border: 1px solid var(--color-divider);
    background: var(--color-hover);
    color: var(--color-text);
  }
  .secondary:hover {
    background: var(--color-surface-high);
  }
  .got-it {
    margin-top: 14px;
  }
  .shortcut-row, .folder-row, .toggle-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 44px;
  }
  .shortcut-row + .shortcut-row {
    border-top: 1px solid var(--color-divider);
  }
  .shortcut-row > div:first-child {
    display: grid;
    gap: 3px;
    flex: 1;
  }
  strong {
    color: var(--color-text);
    font-size: 12px;
    font-weight: 500;
  }
  small {
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .folder-row {
    align-items: center;
  }
  .folder-copy {
    display: grid;
    flex: 1;
    min-width: 0;
    gap: 4px;
  }
  .folder-copy span {
    overflow: hidden;
    color: var(--color-text-muted);
    font: 10px var(--font-mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .text-button {
    padding: 5px 2px;
    background: transparent;
    color: var(--color-primary-hover);
    font-size: 11px;
  }
  .text-button:hover {
    color: var(--color-primary);
  }
  .reset {
    margin-top: 5px;
  }
  .toggle-row {
    position: relative;
    justify-content: space-between;
    cursor: pointer;
  }
  .toggle-row > span {
    display: grid;
    gap: 4px;
  }
  .toggle-row input {
    position: absolute;
    right: 0;
    width: 38px;
    height: 22px;
    opacity: 0;
    cursor: pointer;
  }
  .switch {
    width: 36px;
    height: 20px;
    border: 1px solid var(--color-glass-rim);
    border-radius: 99px;
    background: var(--color-surface-high);
    transition: background-color var(--motion-fast), border-color var(--motion-fast);
  }
  .switch::after {
    display: block;
    width: 14px;
    height: 14px;
    margin: 2px;
    border-radius: 50%;
    background: #a1a1aa;
    content: "";
    transition: transform var(--motion-fast), background-color var(--motion-fast);
  }
  .toggle-row input:checked + .switch {
    border-color: var(--color-primary);
    background: var(--color-primary);
  }
  .toggle-row input:checked + .switch::after {
    transform: translateX(16px);
    background: white;
  }
  .toggle-row input:focus-visible + .switch {
    outline: 2px solid #a5b4fc;
    outline-offset: 2px;
  }
  .diagnostics {
    padding-bottom: 12px;
    interpolate-size: allow-keywords;
  }
  .diagnostics summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
    list-style: none;
  }
  .diagnostics summary::-webkit-details-marker {
    display: none;
  }
  .diagnostics summary .section-title {
    margin-bottom: 0;
  }
  .chevron {
    color: var(--color-text-muted);
    transition: transform 120ms ease;
  }
  .diagnostics[open] .chevron {
    transform: rotate(180deg);
  }
  .diagnostics::details-content {
    height: 0;
    overflow: clip;
    opacity: 0;
    content-visibility: hidden;
    transition: content-visibility var(--motion-normal) allow-discrete,
      height var(--motion-normal), opacity var(--motion-normal);
  }
  .diagnostics[open]::details-content {
    height: auto;
    opacity: 1;
    content-visibility: visible;
  }
  .warning, .compact-warning {
    margin-top: 12px;
    padding: 10px;
    border: 1px solid var(--color-warning-border);
    border-radius: 8px;
    background: var(--color-warning-bg);
    color: var(--color-warning-text);
    font-size: 11px;
  }
  .warning p {
    margin: 6px 0;
    color: var(--color-text-variant);
    line-height: 1.45;
  }
  .warning-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .compact-warning {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .success {
    color: var(--color-success) !important;
  }
  .inline-error, .error-banner {
    margin: 5px 0;
    color: var(--color-danger);
    font-size: 11px;
  }
  .error-banner {
    padding: 9px 12px;
    border: 1px solid var(--color-danger-border);
    border-radius: 8px;
    background: var(--color-danger-bg);
  }
  .loading {
    color: var(--color-text-muted);
    font-size: 12px;
  }
  footer {
    padding: 5px 2px;
    color: var(--color-text-muted);
    font-size: 10px;
  }
</style>
