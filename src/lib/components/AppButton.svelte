<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    type?: "primary" | "warning" | "danger" | "plain" | "ghost";
    size?: "md" | "sm";
    onclick?: (e: MouseEvent) => void;
    disabled?: boolean;
    title?: string;
    children: Snippet;
  }

  let { type = "primary", size = "md", onclick, disabled = false, title, children }: Props = $props();
</script>

<button type="button" class="app-button {type} {size}" {onclick} {disabled} {title}>
  {@render children()}
</button>

<style>
  .app-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 36px;
    padding: 0 14px;
    border-radius: var(--radius);
    border: 1px solid transparent;
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    transition: background-color 0.15s, border-color 0.15s, opacity 0.15s;
  }

  .app-button.sm {
    height: 30px;
    padding: 0 12px;
    font-size: 12px;
    border-radius: 7px;
  }

  .app-button:disabled {
    opacity: 0.5;
  }

  .primary {
    background: var(--accent);
    color: var(--accent-text);
  }
  .primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .warning {
    background: var(--warning);
    color: #1a1305;
  }
  .warning:hover:not(:disabled) {
    background: #fcd34d;
  }

  .danger {
    background: var(--danger-text);
    color: #2a0d10;
  }
  .danger:hover:not(:disabled) {
    background: var(--danger-strong);
  }

  .plain {
    background: transparent;
    border-color: var(--border-button);
    color: var(--text-strong);
    font-weight: 500;
  }
  .plain:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .ghost {
    background: transparent;
    color: var(--text-strong);
    font-weight: 500;
  }
  .ghost:hover:not(:disabled) {
    background: var(--bg-hover);
  }
</style>
