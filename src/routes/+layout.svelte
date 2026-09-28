<script lang="ts">
  import "../lib/styles/tokens.css";

  let { children } = $props();
</script>

{@render children()}

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(html),
  :global(body) {
    min-width: 320px;
    min-height: 100%;
    margin: 0;
    background: var(--color-surface);
    color: var(--color-text);
    font-family: var(--font-sans);
  }
</style>
