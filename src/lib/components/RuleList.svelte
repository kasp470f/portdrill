<script lang="ts">
  import { onMount } from "svelte";
  import type { RuleWithStatus } from "../types";
  import { rules, loading, fetchRules, listenForStatusChanges } from "../stores/rules";
  import RuleCard from "./RuleCard.svelte";
  import RuleForm from "./RuleForm.svelte";

  let showForm = $state(false);
  let editTarget: RuleWithStatus | null = $state(null);

  onMount(() => {
    fetchRules();
    listenForStatusChanges();
  });

  function openNew() {
    editTarget = null;
    showForm = true;
  }

  function openEdit(rule: RuleWithStatus) {
    editTarget = rule;
    showForm = true;
  }

  function closeForm() {
    showForm = false;
    editTarget = null;
  }
</script>

<div class="rule-list">
  <div class="toolbar">
    <span class="count">
      {$rules.length} rule{$rules.length !== 1 ? "s" : ""}
    </span>
    <button class="btn-add" onclick={openNew}>+ Add Rule</button>
  </div>

  {#if $loading}
    <div class="empty">Loading...</div>
  {:else if $rules.length === 0}
    <div class="empty">
      <p>No forwarding rules yet.</p>
      <p class="hint">Click "Add Rule" to create your first SSH tunnel.</p>
    </div>
  {:else}
    <div class="grid">
      {#each $rules as rule (rule.id)}
        <RuleCard {rule} onEdit={openEdit} />
      {/each}
    </div>
  {/if}
</div>

{#if showForm}
  <RuleForm editRule={editTarget} onClose={closeForm} />
{/if}

<style>
  :global(.rule-list) {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  :global(.rule-list .toolbar) {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  :global(.rule-list .count) {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  :global(.rule-list .btn-add) {
    padding: 8px 18px;
    background: var(--accent);
    color: white;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
  }

  :global(.rule-list .btn-add:hover) {
    background: var(--accent-hover);
  }

  :global(.rule-list .grid) {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 12px;
  }

  :global(.rule-list .empty) {
    text-align: center;
    padding: 48px 24px;
    color: var(--text-secondary);
  }

  :global(.rule-list .empty .hint) {
    font-size: 0.875rem;
    margin-top: 8px;
    opacity: 0.7;
  }
</style>
