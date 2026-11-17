<script lang="ts">
  import { onMount } from "svelte";
  import type { RuleWithStatus } from "../types";
  import { rules, loading, fetchRules, listenForStatusChanges, reorderRules } from "../stores/rules";
  import { pendingDeletes } from "../stores/ui";
  import Icon from "./Icon.svelte";
  import RuleRow from "./RuleRow.svelte";
  import EmptyState from "./EmptyState.svelte";

  type Filter = "all" | "active" | "issues";

  interface Props {
    query: string;
    filter: Filter;
  }

  let { query, filter }: Props = $props();

  const COLLAPSED_KEY = "portdrill.collapsedGroups";
  const UNGROUPED = "";

  function loadCollapsed(): Record<string, boolean> {
    try {
      return JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? "{}");
    } catch {
      return {};
    }
  }

  let collapsed = $state<Record<string, boolean>>(loadCollapsed());

  function toggleGroup(name: string) {
    collapsed = { ...collapsed, [name]: !collapsed[name] };
    try {
      localStorage.setItem(COLLAPSED_KEY, JSON.stringify(collapsed));
    } catch {
      /* preference only */
    }
  }

  onMount(() => {
    fetchRules();
    listenForStatusChanges();
  });

  let visible = $derived($rules.filter((r) => !$pendingDeletes.has(r.id)));
  let groupNames = $derived([...new Set(visible.map((r) => r.group).filter((g) => g))]);
  let hasGroups = $derived(groupNames.length > 0);

  const isActive = (r: RuleWithStatus) =>
    r.tunnelStatus.status === "connected" || r.tunnelStatus.status === "connecting";

  function matches(r: RuleWithStatus): boolean {
    if (filter === "active" && !isActive(r)) return false;
    if (filter === "issues" && r.tunnelStatus.status !== "error") return false;
    const q = query.trim().toLowerCase();
    if (!q) return true;
    const hay = [
      r.name,
      r.sshHost,
      r.sshUser,
      r.group,
      String(r.sshPort),
      ...r.forwards.flatMap((f) => [f.localPort, f.remotePort, f.destinationHost, f.destinationPort]),
    ]
      .filter((v) => v !== undefined && v !== null && v !== "")
      .join(" ")
      .toLowerCase();
    return hay.includes(q);
  }

  let filtering = $derived(filter !== "all" || query.trim() !== "");

  let sections = $derived.by(() => {
    const names = hasGroups ? [...groupNames, UNGROUPED] : [UNGROUPED];
    return names
      .map((name) => {
        const all = visible.filter((r) => (r.group || UNGROUPED) === name);
        return {
          name,
          label: name || (hasGroups ? "No group" : ""),
          all,
          rows: all.filter(matches),
          connected: all.filter((r) => r.tunnelStatus.status === "connected").length,
          issues: all.filter((r) => r.tunnelStatus.status === "error").length,
          dots: all.map((r) =>
            r.tunnelStatus.status === "connected"
              ? "var(--success)"
              : r.tunnelStatus.status === "connecting"
                ? "var(--warning)"
                : r.tunnelStatus.status === "error"
                  ? "var(--danger)"
                  : "var(--off)",
          ),
        };
      })
      .filter((s) => s.rows.length > 0 || (!filtering && s.all.length > 0));
  });

  // ---------- drag to reorder (within a section) ----------
  // `moved` stays false until the pointer travels a few pixels, so a plain
  // click on the handle (press and release in place) never reorders anything.
  let drag = $state<{ id: string; group: string; beforeId: string | null; startY: number; moved: boolean } | null>(
    null,
  );
  let listEl: HTMLElement | undefined = $state();
  const DRAG_THRESHOLD = 4;

  function startDrag(rule: RuleWithStatus, e: MouseEvent) {
    if (filtering) return;
    drag = { id: rule.id, group: rule.group || UNGROUPED, beforeId: null, startY: e.clientY, moved: false };
  }

  function onMove(e: MouseEvent) {
    if (!drag || !listEl) return;
    if (!drag.moved && Math.abs(e.clientY - drag.startY) < DRAG_THRESHOLD) return;
    const items = [...listEl.querySelectorAll<HTMLElement>("[data-rule-id]")].filter(
      (el) => el.dataset.group === drag!.group,
    );
    let beforeId: string | null = null;
    for (const el of items) {
      const box = el.getBoundingClientRect();
      if (e.clientY < box.top + box.height / 2) {
        beforeId = el.dataset.ruleId!;
        break;
      }
    }
    drag = { ...drag, beforeId, moved: true };
  }

  async function endDrag() {
    if (!drag) return;
    const { id, group, beforeId, moved } = drag;
    drag = null;
    if (!moved) return;
    const ids = $rules.map((r) => r.id).filter((x) => x !== id);
    let at: number;
    if (beforeId && beforeId !== id) {
      at = ids.indexOf(beforeId);
    } else {
      const lastInGroup = [...$rules]
        .reverse()
        .find((r) => r.id !== id && (r.group || UNGROUPED) === group);
      at = lastInGroup ? ids.indexOf(lastInGroup.id) + 1 : ids.length;
    }
    ids.splice(at, 0, id);
    if (ids.join() === $rules.map((r) => r.id).join()) return;
    const byId = new Map($rules.map((r) => [r.id, r]));
    rules.set(ids.map((x) => byId.get(x)!));
    try {
      await reorderRules(ids);
    } catch (err) {
      console.error("Reorder failed:", err);
      await fetchRules();
    }
  }
</script>

<svelte:window onmousemove={drag ? onMove : undefined} onmouseup={drag ? () => void endDrag() : undefined} />

<div class="list" bind:this={listEl}>
  {#if $loading && $rules.length === 0}
    <div class="empty">Loading…</div>
  {:else if visible.length === 0}
    <EmptyState />
  {:else if sections.length === 0}
    <div class="empty">No rules match this search or filter.</div>
  {:else}
    {#each sections as section (section.name)}
      {@const isCollapsed = !!collapsed[section.name] && !filtering}
      <section class="group">
        {#if section.label}
          <button
            type="button"
            class="group-header"
            aria-expanded={!isCollapsed}
            onclick={() => toggleGroup(section.name)}
          >
            <span class="chev" class:open={!isCollapsed}><Icon name="chevronRight" size={12} strokeWidth={2.5} /></span>
            <span class="group-name">{section.label}</span>
            {#if isCollapsed}
              <span class="dots" aria-hidden="true">
                {#each section.dots as color, i (i)}<span style="background: {color}"></span>{/each}
              </span>
            {/if}
            <span class="spacer"></span>
            {#if isCollapsed && section.issues > 0}
              <span class="issues">{section.issues} {section.issues === 1 ? "issue" : "issues"}</span>
            {/if}
            <span class="summary">{section.connected}/{section.all.length}</span>
          </button>
        {/if}
        {#if !isCollapsed}
          <div class="card">
            {#each section.rows as rule, i (rule.id)}
              <div
                class="drag-item"
                class:drop-before={drag?.moved && drag.beforeId === rule.id && drag.id !== rule.id}
                class:drop-after={drag?.moved && drag.beforeId === null && drag.group === section.name && i === section.rows.length - 1 && drag.id !== rule.id}
                data-rule-id={rule.id}
                data-group={rule.group || UNGROUPED}
              >
                <RuleRow
                  {rule}
                  allRules={$rules}
                  groups={groupNames}
                  first={i === 0}
                  last={i === section.rows.length - 1}
                  dragging={!!drag?.moved && drag.id === rule.id}
                  onGripDown={filtering ? undefined : (e) => startDrag(rule, e)}
                />
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {/each}
  {/if}
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-bottom: 16px;
  }

  .empty {
    padding: 48px 24px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 14px;
  }

  .group {
    display: flex;
    flex-direction: column;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 8px;
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    text-align: left;
  }

  .group-header:hover {
    color: var(--text-strong);
  }

  .chev {
    display: flex;
    transition: transform 0.15s;
  }

  .chev.open {
    transform: rotate(90deg);
  }

  .dots {
    display: flex;
    gap: 4px;
    margin-left: 4px;
  }

  .dots span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .spacer {
    flex-grow: 1;
  }

  .issues {
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
    color: var(--danger-text);
  }

  .summary {
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }

  .card {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--bg-card);
  }

  .drag-item.drop-before {
    box-shadow: inset 0 2px 0 var(--accent);
  }

  .drag-item.drop-after {
    box-shadow: inset 0 -2px 0 var(--accent);
  }
</style>
