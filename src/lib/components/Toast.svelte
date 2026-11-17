<script lang="ts">
  import { onDestroy } from "svelte";
  import { toast, undoDelete, UNDO_SECONDS } from "../stores/ui";

  let now = $state(Date.now());
  const tick = setInterval(() => (now = Date.now()), 250);
  onDestroy(() => clearInterval(tick));

  let remaining = $derived(
    $toast?.kind === "undo" ? Math.max(0, Math.ceil(($toast.expiresAt - now) / 1000)) : 0,
  );
  let progress = $derived(
    $toast?.kind === "undo" ? Math.max(0, Math.min(1, ($toast.expiresAt - now) / (UNDO_SECONDS * 1000))) : 0,
  );

  function onKey(e: KeyboardEvent) {
    if ($toast?.kind === "undo" && (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
      e.preventDefault();
      undoDelete();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if $toast}
  <div class="toast" class:error={$toast.kind === "message" && $toast.tone === "error"} role="status">
    {#if $toast.kind === "undo"}
      <div class="body">
        <span>Deleted <strong>{$toast.name}</strong></span>
        <span class="secs mono">{remaining}s</span>
        <button type="button" onclick={undoDelete}>Undo <kbd>Ctrl Z</kbd></button>
      </div>
      <div class="bar" aria-hidden="true"><div style="width: {progress * 100}%"></div></div>
    {:else}
      <div class="body">
        <span>{$toast.text}</span>
      </div>
    {/if}
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    left: 50%;
    bottom: 46px;
    transform: translateX(-50%);
    z-index: 90;
    max-width: calc(100vw - 32px);
    display: flex;
    flex-direction: column;
    border-radius: 10px;
    background: var(--text-primary);
    color: #141416;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  .toast.error {
    background: var(--danger-strong);
    color: #2a0d10;
  }

  .body {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 8px 8px 14px;
    font-size: 13px;
    min-height: 44px;
  }

  .body > span:first-child {
    overflow-wrap: anywhere;
  }

  .body:has(> span:only-child) {
    padding-right: 14px;
  }

  strong {
    font-weight: 600;
  }

  .secs {
    font-size: 11px;
    color: #52525b;
  }

  button {
    height: 28px;
    padding: 0 10px;
    border-radius: 6px;
    background: #141416;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
  }

  button kbd {
    font-size: 10px;
    color: var(--text-secondary);
  }

  .bar {
    height: 3px;
    background: var(--text-strong);
  }

  .bar div {
    height: 3px;
    background: var(--accent);
    transition: width 0.25s linear;
  }
</style>
