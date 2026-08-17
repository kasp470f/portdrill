<script lang="ts">
  import { onMount } from "svelte";
  import type { RuleWithStatus } from "../types";
  import { rules, loading, fetchRules, listenForStatusChanges, reorderRules, duplicateRule } from "../stores/rules";
  import RuleCard from "./RuleCard.svelte";
  import RuleForm from "./RuleForm.svelte";
  import AppButton from "./AppButton.svelte";

  let showForm = $state(false);
  let editTarget: RuleWithStatus | null = $state(null);

  let dragState = $state<{
    fromIndex: number;
    dropIndex: number | null;
  } | null>(null);

  let gridEl: HTMLElement | null = $state(null);

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

  function startDrag(index: number) {
    dragState = { fromIndex: index, dropIndex: null };
  }

  // Dropping right before or right after the dragged item's own position
  // is a no-op (it lands back where it started), so don't highlight those slots.
  function isTrivialDrop(index: number, fromIndex: number): boolean {
    return index === fromIndex || index === fromIndex + 1;
  }

  function updateDrop(e: MouseEvent) {
    if (!dragState || !gridEl) return;
    const items = gridEl.querySelectorAll(".drag-item");
    let best: number = dragState.fromIndex;
    let bestDist = Infinity;

    items.forEach((item, i) => {
      const rect = item.getBoundingClientRect();
      const midY = rect.top + rect.height / 2;
      const dist = Math.abs(e.clientY - midY);
      if (dist < bestDist) {
        bestDist = dist;
        best = e.clientY < midY ? i : i + 1;
      }
    });

    dragState = { ...dragState, dropIndex: best };
  }

  async function endDrag() {
    if (!dragState || dragState.dropIndex === null) {
      dragState = null;
      return;
    }

    let toIndex = dragState.dropIndex;
    const fromIndex = dragState.fromIndex;
    if (fromIndex < toIndex) toIndex -= 1;

    dragState = null;

    if (fromIndex === toIndex) return;

    const currentRules = [...$rules];
    const [moved] = currentRules.splice(fromIndex, 1);
    currentRules.splice(toIndex, 0, moved);
    rules.set(currentRules);

    try {
      await reorderRules(currentRules.map((r) => r.id));
    } catch (err) {
      console.error("Reorder failed:", err);
      await fetchRules();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="rule-list"
  onmousemove={dragState ? updateDrop : undefined}
  onmouseup={dragState ? endDrag : undefined}
  onmouseleave={dragState ? endDrag : undefined}
>
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
        <div class="grid" bind:this={gridEl}>
          {#each $rules as rule, i (rule.id)}
            <div
              class="drag-slot"
              class:visible={dragState !== null &&
                dragState.dropIndex === i &&
                !isTrivialDrop(i, dragState.fromIndex)}
            >
              <div class="drag-line"></div>
            </div>
            <div
              class="drag-item"
              style={dragState?.fromIndex === i ? "opacity: 0.3;" : ""}
            >
              <RuleCard
                {rule}
                onEdit={openEdit}
                onDuplicate={(r) => duplicateRule(r)}
                onGripDown={() => startDrag(i)}
              />
            </div>
          {/each}
          <div
            class="drag-slot"
            class:visible={dragState !== null &&
              dragState.dropIndex === $rules.length &&
              !isTrivialDrop($rules.length, dragState.fromIndex)}
          >
            <div class="drag-line"></div>
          </div>
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
    padding: 4px 8px 4px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background-color: color-mix(in srgb, var(--bg-card) 20%, transparent 80%);
  }

  :global(.rule-list .rules-container) {
    flex: 1;
    height: 100%;
    overflow-y: auto;
    padding-right: 8px;
    position: relative;

    &::-webkit-scrollbar {
      width: 6px;
    }

    &::-webkit-scrollbar-track {
      border-radius: var(--radius);
      overflow: hidden;
      margin: 12px 0;
    }

    &::-webkit-scrollbar-thumb {
      border-radius: var(--radius);
    }
  }

  :global(.rule-list .grid) {
    display: flex;
    flex-direction: column;
  }

  :global(.rule-list .drag-item) {
    transition: opacity 0.1s;
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

  :global(.rule-list .drag-slot) {
    height: 12px;
    display: flex;
    align-items: center;
    pointer-events: none;
  }

  :global(.rule-list .drag-slot .drag-line) {
    width: 100%;
    height: 3px;
    border-radius: 2px;
    background: transparent;
    transition: background-color 0.1s;
  }

  :global(.rule-list .drag-slot.visible .drag-line) {
    background: var(--accent);
  }
</style>
