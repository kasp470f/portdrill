<script lang="ts">
  import type { RuleWithStatus } from "../types";
  import { forwardSummary, forwardTypeLabel } from "../types";
  import { toggleRule, deleteRule } from "../stores/rules";
  import StatusDot from "./StatusDot.svelte";
  import AppButton from "./AppButton.svelte";

  interface Props {
    rule: RuleWithStatus;
    onEdit: (rule: RuleWithStatus) => void;
    onDuplicate: (rule: RuleWithStatus) => void;
    onGripDown?: (e: MouseEvent) => void;
  }

  let { rule, onEdit, onDuplicate, onGripDown }: Props = $props();
  let toggling = $state(false);
  let deleting = $state(false);
  let menuOpen = $state(false);
  let deleteDialogOpen = $state(false);
  let showForwards = $state(false);

  let isActive = $derived(
    rule.tunnelStatus.status === "connected" ||
      rule.tunnelStatus.status === "connecting",
  );

  let statusLabel = $derived.by(() => {
    switch (rule.tunnelStatus.status) {
      case "connected": return "Connected";
      case "connecting": return "Connecting...";
      case "error": return "Error";
      default: return "Disconnected";
    }
  });

  async function handleToggle() {
    toggling = true;
    try {
      await toggleRule(rule.id);
    } catch (e) {
      console.error("Toggle failed:", e);
    } finally {
      toggling = false;
    }
  }

  async function handleDelete() {
    deleting = true;
    try {
      await deleteRule(rule.id);
      closeDeleteDialog();
    } catch (e) {
      console.error("Delete failed:", e);
    } finally {
      deleting = false;
    }
  }

  function closeMenu() {
    menuOpen = false;
  }

  function openDeleteDialog() {
    deleteDialogOpen = true;
    menuOpen = false;
  }

  function closeDeleteDialog() {
    deleteDialogOpen = false;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
{#if menuOpen}
  <div class="menu-backdrop" onclick={closeMenu}></div>
{/if}

{#if deleteDialogOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="dialog-backdrop" onclick={closeDeleteDialog}></div>
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="delete-dialog" role="dialog" aria-modal="true" aria-labelledby="delete-rule-title" onclick={(e) => e.stopPropagation()}>
    <h3 id="delete-rule-title">Delete rule?</h3>
    <p>Are you sure you want to delete <strong>{rule.name}</strong>?</p>
    <div class="dialog-actions">
      <AppButton type="plain" onclick={closeDeleteDialog}>Cancel</AppButton>
      <AppButton type="danger" onclick={() => { void handleDelete(); }} disabled={deleting}>
        {deleting ? "Deleting..." : "Delete rule"}
      </AppButton>
    </div>
  </div>
{/if}

<div class="card" class:active={isActive} class:error={rule.tunnelStatus.status === "error"}>
  <div class="card-row">
    {#if onGripDown}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="drag-grip" onmousedown={(e) => { e.preventDefault(); onGripDown(e); }}>
        <span>⠿</span>
      </div>
    {/if}
    <div class="card-left">
      <div class="badges">
        {#each rule.forwards as fwd}
          <span class="type-badge" class:local={fwd.forwardType === "local"} class:remote={fwd.forwardType === "remote"} class:dynamic={fwd.forwardType === "dynamic"}>
            {forwardTypeLabel(fwd.forwardType)}
          </span>
        {/each}
      </div>
      <div class="info">
        <span class="name">{rule.name}</span>
        <span class="meta">
          <span class="ssh-target">{rule.sshUser}@{rule.sshHost}:{rule.sshPort}</span>
          <span class="separator">·</span>
          <StatusDot status={rule.tunnelStatus} />
          <span class="status-label" class:connected={isActive} class:errored={rule.tunnelStatus.status === "error"}>{statusLabel}</span>
        </span>
      </div>
    </div>

    <div class="card-right">
      <label class="toggle-switch" title={isActive ? "Disconnect" : "Connect"}>
        <input type="checkbox" checked={isActive} onchange={handleToggle} disabled={toggling} />
        <span class="toggle-track">
          <span class="toggle-thumb"></span>
        </span>
      </label>

      <div class="menu-anchor">
        <button class="menu-trigger" onclick={() => menuOpen = !menuOpen} title="More actions">
          <span class="dots">⋮</span>
        </button>
        {#if menuOpen}
          <div class="menu-dropdown">
            <button class="menu-item" onclick={() => { showForwards = !showForwards; closeMenu(); }}>
              {showForwards ? "Hide" : "Show"} forwards
            </button>
            <button class="menu-item" onclick={() => { onEdit(rule); closeMenu(); }} disabled={isActive}>
              Edit rule
            </button>
            <button class="menu-item" onclick={() => { onDuplicate(rule); closeMenu(); }}>
              Duplicate rule
            </button>
            <button class="menu-item danger" onclick={openDeleteDialog} disabled={isActive}>
              Delete rule
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>

  {#if showForwards}
    <div class="forwards">
      {#each rule.forwards as fwd}
        <span class="forward-line">{forwardSummary(fwd)}</span>
      {/each}
    </div>
  {/if}
</div>

<style>
  :global(.card) {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    min-width: 480px;
    transition: border-color 0.2s, box-shadow 0.2s;
  }

  :global(.card:hover) {
    border-color: color-mix(in srgb, var(--accent) 20%, var(--border) 80%);
  }

  :global(.card.active) {
    border-color: var(--success);
  }

  :global(.card.error) {
    border-color: var(--danger);
  }

  :global(.card .card-row) {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  :global(.card .drag-grip) {
    display: flex;
    align-items: center;
    cursor: grab;
    color: var(--text-secondary);
    opacity: 0.25;
    transition: opacity 0.15s;
    user-select: none;
    font-size: 1rem;
    line-height: 1;
    margin-left: -6px;
  }

  :global(.card .drag-grip:hover) {
    opacity: 0.7;
  }

  :global(.card .drag-grip:active) {
    cursor: grabbing;
    opacity: 1;
  }

  :global(.card .card-left) {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    min-width: 0;
  }

  :global(.card .card-right) {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  :global(.card .badges) {
    display: grid;
    grid-template-rows: repeat(2, auto);
    grid-auto-flow: column;
    grid-auto-columns: auto;
    gap: 3px;
    user-select: none;
  }

  :global(.card .type-badge) {
    width: 26px;
    height: 20px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 0.7rem;
    flex-shrink: 0;
  }

  :global(.card .type-badge.local) {
    background: rgba(108, 140, 255, 0.15);
    color: var(--accent);
  }

  :global(.card .type-badge.remote) {
    background: rgba(74, 222, 128, 0.15);
    color: var(--success);
  }

  :global(.card .type-badge.dynamic) {
    background: rgba(251, 191, 36, 0.15);
    color: var(--warning);
  }

  :global(.card .info) {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  :global(.card .name) {
    font-weight: 600;
    font-size: 0.9375rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  :global(.card .meta) {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  :global(.card .separator) {
    opacity: 0.4;
    user-select: none;
  }

  :global(.card .status-label) {
    font-size: 0.7rem;
    font-weight: 500;
  }

  :global(.card .status-label.connected) {
    color: var(--success);
  }

  :global(.card .status-label.errored) {
    color: var(--danger);
  }

  /* Toggle switch */
  :global(.card .toggle-switch) {
    position: relative;
    display: inline-flex;
    cursor: pointer;
  }

  :global(.card .toggle-switch input) {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  :global(.card .toggle-track) {
    width: 36px;
    height: 20px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 10px;
    position: relative;
    transition: background 0.2s, border-color 0.2s;
  }

  :global(.card .toggle-switch input:checked + .toggle-track) {
    background: var(--success);
    border-color: var(--success);
  }

  :global(.card .toggle-thumb) {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    background: white;
    border-radius: 50%;
    transition: transform 0.2s;
  }

  :global(.card .toggle-switch input:checked + .toggle-track .toggle-thumb) {
    transform: translateX(16px);
  }

  :global(.card .toggle-switch input:disabled + .toggle-track) {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Kebab menu */
  :global(.card .menu-anchor) {
    position: relative;
  }

  :global(.card .menu-trigger) {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    transition: background 0.15s;
  }

  :global(.card .menu-trigger:hover) {
    background: var(--bg-input);
    color: var(--text-primary);
  }

  :global(.card .dots) {
    font-size: 1.125rem;
    line-height: 1;
  }

  :global(.card .menu-dropdown) {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px;
    min-width: 160px;
    z-index: 50;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  :global(.card .menu-item) {
    display: block;
    width: 100%;
    padding: 8px 12px;
    text-align: left;
    font-size: 0.8125rem;
    color: var(--text-primary);
    border-radius: 5px;
    transition: background 0.1s;
  }

  :global(.card .menu-item:hover:not(:disabled)) {
    background: var(--bg-input);
  }

  :global(.card .menu-item:disabled) {
    opacity: 0.4;
    cursor: not-allowed;
  }

  :global(.card .menu-item.danger) {
    color: var(--danger);
  }

  :global(.menu-backdrop) {
    position: fixed;
    inset: 0;
    z-index: 40;
  }

  :global(.dialog-backdrop) {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgba(0, 0, 0, 0.55);
  }

  :global(.delete-dialog) {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 70;
    width: min(420px, calc(100vw - 24px));
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 20px;
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.35);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  :global(.delete-dialog h3) {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  :global(.delete-dialog p) {
    color: var(--text-secondary);
    line-height: 1.5;
  }

  :global(.dialog-actions) {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }

  :global(.dialog-btn) {
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
  }

  :global(.dialog-btn.secondary) {
    background: var(--bg-input);
    color: var(--text-primary);
  }

  :global(.dialog-btn.danger) {
    background: var(--danger);
    color: white;
  }

  :global(.dialog-btn:disabled) {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* Forwards (collapsible) */
  :global(.card .forwards) {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-top: 10px;
    padding: 8px 10px;
    background: var(--bg-primary);
    border-radius: 6px;
  }

  :global(.card .forward-line) {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    font-family: "SF Mono", "Cascadia Code", "Fira Code", monospace;
  }
</style>
