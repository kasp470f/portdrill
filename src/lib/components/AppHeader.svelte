<script lang="ts">
  import appIcon from "../../../src-tauri/icons/64x64.png";
  import { rules } from "../stores/rules";
  import { openNewRule } from "../stores/ui";
  import Icon from "./Icon.svelte";

  type Filter = "all" | "active" | "issues";

  interface Props {
    query: string;
    filter: Filter;
    searchEl?: HTMLInputElement;
  }

  let { query = $bindable(), filter = $bindable(), searchEl = $bindable() }: Props = $props();

  let connected = $derived($rules.filter((r) => r.tunnelStatus.status === "connected").length);
  let live = $derived(
    $rules.filter((r) => r.tunnelStatus.status === "connected" || r.tunnelStatus.status === "connecting").length,
  );
  let issues = $derived($rules.filter((r) => r.tunnelStatus.status === "error").length);

  let filters = $derived([
    { id: "all" as const, label: "All", count: $rules.length },
    { id: "active" as const, label: "Active", count: live },
    { id: "issues" as const, label: "Issues", count: issues },
  ]);
</script>

<header>
  <div class="title-row">
    <img class="app-icon" src={appIcon} alt="PortDrill" />
    <div class="title">
      <h1>PortDrill</h1>
      <span class="counts"
        ><span class="active">{connected} active</span> · {$rules.length}
        {$rules.length === 1 ? "rule" : "rules"}</span
      >
    </div>
    <button type="button" class="new-rule" onclick={() => openNewRule("fill")}>
      <Icon name="plus" size={14} strokeWidth={2.5} />
      New rule
    </button>
  </div>

  {#if $rules.length > 0}
    <div class="tools">
      <label class="search">
        <span class="sr-only">Search rules</span>
        <span class="search-icon"><Icon name="search" /></span>
        <input
          bind:this={searchEl}
          bind:value={query}
          type="text"
          placeholder="Search name, host or port"
          onkeydown={(e) => {
            if (e.key === "Escape") {
              query = "";
              (e.target as HTMLInputElement).blur();
            }
          }}
        />
        <kbd>Ctrl K</kbd>
      </label>
      <div class="filters" role="group" aria-label="Filter">
        {#each filters as f (f.id)}
          <button type="button" class:on={filter === f.id} aria-pressed={filter === f.id} onclick={() => (filter = f.id)}>
            {f.label}
            <span class="count" class:issue={f.id === "issues" && f.count > 0}>{f.count}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</header>

<style>
  header {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px 20px 14px;
    flex-shrink: 0;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .app-icon {
    width: 46px;
    height: 46px;
    flex-shrink: 0;
    padding: 4px;
    border-radius: var(--radius);
    border: 1px solid #3f3f46;
    background: #27272a;
    user-select: none;
    pointer-events: none;
  }

  .title {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex-grow: 1;
    min-width: 0;
  }

  h1 {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .counts {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .counts .active {
    color: var(--success);
  }

  .new-rule {
    height: 36px;
    padding: 0 14px;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-text);
    font-size: 13px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .new-rule:hover {
    background: var(--accent-hover);
  }

  .tools {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .search {
    position: relative;
    flex-grow: 1;
    display: block;
  }

  .search-icon {
    position: absolute;
    left: 11px;
    top: 11px;
    color: var(--text-muted);
  }

  .search input {
    width: 100%;
    height: 38px;
    padding: 0 64px 0 34px;
    border-radius: var(--radius);
    border: 1px solid var(--border-strong);
    background: var(--bg-raised);
    color: var(--text-primary);
    font-size: 13px;
  }

  .search input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .search input::placeholder {
    color: var(--text-muted);
  }

  .search kbd {
    position: absolute;
    right: 9px;
    top: 9px;
    color: var(--text-secondary);
    background: #26262b;
    border: 1px solid var(--border-input);
    border-radius: 4px;
    padding: 1px 6px;
  }

  .filters {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--radius);
    border: 1px solid var(--border-strong);
    background: var(--bg-raised);
  }

  .filters button {
    height: 30px;
    padding: 0 10px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .filters button.on {
    background: var(--bg-badge);
    color: var(--text-primary);
  }

  .count {
    /* Same font as the label (a mono font sits on a different baseline and
       looked low next to it); tabular figures keep the width steady. */
    font-size: 11px;
    font-weight: 500;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }

  .filters button {
    line-height: 1;
  }

  .count.issue {
    color: var(--danger-text);
  }
</style>
