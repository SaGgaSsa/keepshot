<script lang="ts">
  type Props = {
    value: string;
    onCommit: (shortcut: string) => Promise<void>;
    onRecordingChange: (recording: boolean) => Promise<void>;
  };

  let { value, onCommit, onRecordingChange }: Props = $props();
  let recording = $state(false);
  let busy = $state(false);

  async function startRecording(): Promise<void> {
    busy = true;
    try {
      await onRecordingChange(true);
      recording = true;
    } finally {
      busy = false;
    }
  }

  function stopRecording(): void {
    recording = false;
    onRecordingChange(false);
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (!recording) return;
    if (event.key === "Escape") {
      event.preventDefault();
      stopRecording();
      return;
    }
    if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) return;
    if (event.code === "PrintScreen") return;
    event.preventDefault();
    void submit(event);
  }

  function onKeyUp(event: KeyboardEvent): void {
    if (!recording || event.code !== "PrintScreen") return;
    event.preventDefault();
    void submit(event);
  }

  function onWindowBlur(): void {
    if (recording) stopRecording();
  }

  async function submit(event: KeyboardEvent): Promise<void> {
    const key = keyToken(event.code);
    if (!key || busy) return;
    const parts: string[] = [];
    if (event.ctrlKey) parts.push("Ctrl");
    if (event.altKey) parts.push("Alt");
    if (event.shiftKey) parts.push("Shift");
    if (event.metaKey) parts.push("Super");
    parts.push(key);
    busy = true;
    try {
      await onCommit(parts.join("+"));
    } finally {
      busy = false;
      stopRecording();
    }
  }

  function keyToken(code: string): string | null {
    if (code.startsWith("Key")) return code.slice(3);
    if (code.startsWith("Digit")) return code.slice(5);
    if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
    if (code.startsWith("Numpad")) return code;
    const known: Record<string, string> = {
      PrintScreen: "PrintScreen",
      Space: "Space",
      Enter: "Enter",
      Backspace: "Backspace",
      Delete: "Delete",
      Insert: "Insert",
      Home: "Home",
      End: "End",
      PageUp: "PageUp",
      PageDown: "PageDown",
      ArrowUp: "ArrowUp",
      ArrowDown: "ArrowDown",
      ArrowLeft: "ArrowLeft",
      ArrowRight: "ArrowRight",
      Tab: "Tab",
      Minus: "Minus",
      Equal: "Equal",
      BracketLeft: "BracketLeft",
      BracketRight: "BracketRight",
      Backslash: "Backslash",
      Semicolon: "Semicolon",
      Quote: "Quote",
      Backquote: "Backquote",
      Comma: "Comma",
      Period: "Period",
      Slash: "Slash",
    };
    return known[code] ?? null;
  }

  function displayTokens(shortcut: string): string[] {
    return shortcut.split("+").map((token) => {
      if (token === "PrintScreen") return "Print Screen";
      if (token === "Super") return "Win";
      return token;
    });
  }
</script>

<svelte:window onkeydown={onKeyDown} onkeyup={onKeyUp} onblur={onWindowBlur} />

<button class:recording type="button" onclick={startRecording} disabled={busy}>
  {#if recording}
    <span class="prompt">Press a shortcut…</span>
  {:else}
    {#each displayTokens(value) as token, index (index)}
      {#if index > 0}<span class="join">+</span>{/if}
      <kbd>{token}</kbd>
    {/each}
  {/if}
</button>

<style>
  button {
    display: flex;
    align-items: center;
    gap: 5px;
    min-height: 36px;
    padding: 5px 8px;
    border: 1px solid var(--color-divider);
    border-radius: var(--radius-md);
    background: var(--color-hover);
    color: var(--color-text);
    cursor: pointer;
    transition: background-color var(--motion-fast), color var(--motion-fast),
      border-color var(--motion-fast);
  }
  button:hover, button.recording {
    border-color: rgba(99, 102, 241, 0.6);
    background: var(--color-primary-soft);
  }
  button:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  kbd {
    padding: 4px 6px;
    border: 1px solid var(--color-glass-rim);
    border-radius: 4px;
    background: var(--color-shortcut-bg);
    color: var(--color-shortcut-text);
    font: 600 10px var(--font-sans);
    text-transform: uppercase;
  }
  .join {
    color: var(--color-text-muted);
    font-size: 10px;
  }
  .prompt {
    color: var(--color-text-strong);
    font-size: 12px;
  }
</style>
