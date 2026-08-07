<script lang="ts">
  import type { RuleWithStatus } from "../types";
  import { forwardSummary, forwardTypeLabel } from "../types";
  import { toggleRule, deleteRule } from "../stores/rules";
  import StatusDot from "./StatusDot.svelte";

  interface Props {
    rule: RuleWithStatus;
    onEdit: (rule: RuleWithStatus) => void;
  }

  let { rule, onEdit }: Props = $props();
  let toggling = $state(false);
  let deleting = $state(false);
  let confirmingDelete = $state(false);

  let isActive = $derived(
    rule.tunnelStatus.status === "connected" ||
      rule.tunnelStatus.status === "connecting",
  );

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
    } catch (e) {
      console.error("Delete failed:", e);
      deleting = false;
    }
  }
</script>

<div class="card" class:active={isActive} class:error={rule.tunnelStatus.status === "error"}>
  <div class="card-header">
    <div class="badges">
      {#each rule.forwards as fwd}
        <span class="type-badge" class:local={fwd.forwardType === "local"} class:remote={fwd.forwardType === "remote"} class:dynamic={fwd.forwardType === "dynamic"}>
          {forwardTypeLabel(fwd.forwardType)}
        </span>
      {/each}
    </div>
    <div class="info">
      <span class="name">{rule.name}</span>
      <span class="ssh-target">{rule.sshUser}@{rule.sshHost}:{rule.sshPort}</span>
    </div>
    <StatusDot status={rule.tunnelStatus} />
  </div>

  <div class="forwards">
    {#each rule.forwards as fwd}
      <span class="forward-line">{forwardSummary(fwd)}</span>
    {/each}
  </div>

  <div class="card-actions">
    <button class="btn toggle" class:on={isActive} onclick={handleToggle} disabled={toggling}>
      {isActive ? "Disconnect" : "Connect"}
    </button>
    <button class="btn edit" onclick={() => onEdit(rule)} disabled={isActive}>
      Edit
    </button>
    {#if confirmingDelete}
      <button class="btn confirm-delete" onclick={handleDelete} disabled={deleting}>
        Confirm
      </button>
      <button class="btn cancel-delete" onclick={() => confirmingDelete = false}>
        Cancel
      </button>
    {:else}
      <button class="btn delete" onclick={() => confirmingDelete = true} disabled={isActive}>
        Delete
      </button>
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
    transition: border-color 0.2s;
  }

  .card.active {
    border-color: var(--success);
  }

  .card.error {
    border-color: var(--danger);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
  }

  .badges {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .type-badge {
    width: 28px;
    height: 22px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .type-badge.local {
    background: rgba(108, 140, 255, 0.15);
    color: var(--accent);
  }

  .type-badge.remote {
    background: rgba(74, 222, 128, 0.15);
    color: var(--success);
  }

  .type-badge.dynamic {
    background: rgba(251, 191, 36, 0.15);
    color: var(--warning);
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    font-weight: 600;
    font-size: 0.9375rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ssh-target {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .forwards {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-bottom: 12px;
    padding: 8px 10px;
    background: var(--bg-primary);
    border-radius: 6px;
  }

  .forward-line {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    font-family: "SF Mono", "Cascadia Code", "Fira Code", monospace;
  }

  .card-actions {
    display: flex;
    gap: 8px;
  }

  .btn {
    padding: 6px 14px;
    border-radius: 6px;
    font-size: 0.8125rem;
    font-weight: 500;
    transition: background 0.15s, opacity 0.15s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.toggle {
    background: var(--accent);
    color: white;
  }

  .btn.toggle:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .btn.toggle.on {
    background: var(--danger);
  }

  .btn.toggle.on:hover:not(:disabled) {
    background: #ef4444;
  }

  .btn.edit {
    background: var(--bg-input);
    color: var(--text-primary);
  }

  .btn.edit:hover:not(:disabled) {
    background: var(--border);
  }

  .btn.delete {
    background: transparent;
    color: var(--danger);
    margin-left: auto;
  }

  .btn.delete:hover:not(:disabled) {
    background: rgba(248, 113, 113, 0.1);
  }

  .btn.confirm-delete {
    background: var(--danger);
    color: white;
    margin-left: auto;
  }

  .btn.confirm-delete:hover:not(:disabled) {
    background: #ef4444;
  }

  .btn.cancel-delete {
    background: var(--bg-input);
    color: var(--text-secondary);
  }

  .btn.cancel-delete:hover {
    background: var(--border);
  }
</style>
