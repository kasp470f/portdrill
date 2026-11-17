<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    title: string;
    onClose: () => void;
    /** Extra controls next to the title (e.g. tabs). */
    headerExtra?: Snippet;
    footer?: Snippet;
    children: Snippet;
    /** "full" fills the window like the rule dialog; "auto" sizes to content. */
    size?: "full" | "auto";
    icon?: Snippet;
  }

  let { title, onClose, headerExtra, footer, children, size = "full", icon }: Props = $props();

  let panel: HTMLElement | undefined = $state();
  const titleId = `modal-title-${Math.random().toString(36).slice(2)}`;

  onMount(() => {
    const previous = document.activeElement as HTMLElement | null;
    panel?.querySelector<HTMLElement>("input, textarea, select, button:not(.close)")?.focus();
    return () => previous?.focus?.();
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="backdrop" role="presentation" onmousedown={(e) => e.target === e.currentTarget && onClose()}>
  <div class="panel {size}" role="dialog" aria-modal="true" aria-labelledby={titleId} bind:this={panel}>
    <div class="head">
      {#if icon}{@render icon()}{/if}
      <h2 id={titleId}>{title}</h2>
      {#if headerExtra}{@render headerExtra()}{/if}
      <button type="button" class="close" aria-label="Close" onclick={onClose}>
        <Icon name="close" size={14} strokeWidth={2.2} />
      </button>
    </div>
    <div class="content">
      {@render children()}
    </div>
    {#if footer}
      <div class="foot">{@render footer()}</div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    background: rgba(6, 6, 8, 0.66);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px 24px;
  }

  .panel {
    width: 100%;
    max-width: 640px;
    max-height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-dialog);
    border: 1px solid var(--border-strong);
    border-radius: 14px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  .panel.full {
    height: 100%;
  }

  .panel.auto {
    max-width: 570px;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 16px 12px 20px;
    border-bottom: 1px solid var(--border);
  }

  h2 {
    flex-grow: 1;
    font-size: 17px;
    font-weight: 600;
  }

  .close {
    width: 32px;
    height: 32px;
    border-radius: 7px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .close:hover {
    background: var(--bg-active);
    color: var(--text-primary);
  }

  .content {
    flex-grow: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px 12px 20px;
    border-top: 1px solid var(--border);
  }
</style>
