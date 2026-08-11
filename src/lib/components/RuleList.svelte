<script lang="ts">
  import { onMount } from "svelte";
  import type { RuleWithStatus } from "../types";
  import { rules, loading, fetchRules, listenForStatusChanges } from "../stores/rules";
  import RuleCard from "./RuleCard.svelte";
  import RuleForm from "./RuleForm.svelte";
  import AppButton from "./AppButton.svelte";

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
    <AppButton onclick={openNew}>+ Add Rule</AppButton>
  </div>

  <div class="rules-wrapper">
    <div class="rules-container">
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
  </div>
</div>

{#if showForm}
  <RuleForm editRule={editTarget} onClose={closeForm} />
{/if}

<style lang="scss">
  :global(.rule-list) {
    display: flex;
    flex-direction: column;
    gap: 16px;
    flex: 1;
    overflow: hidden;
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

  :global(.rule-list .rules-wrapper) {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    padding: 12px 8px 12px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background-color: color-mix(in srgb, var(--bg-card) 20%, transparent 80%);
  }

  :global(.rule-list .rules-container) {
    height: 100%;
    overflow-y: auto;
    padding-right: 8px;

    &::-webkit-scrollbar {
      width: 6px;
    }

    &::-webkit-scrollbar-track {
      border-radius: var(--radius);
      overflow: hidden;
    }

    &::-webkit-scrollbar-thumb {
      border-radius: var(--radius);
    }
  }

  :global(.rule-list .grid) {
    display: flex;
    flex-direction: column;
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
