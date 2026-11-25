<script lang="ts" module>
  export type MenuAction =
    | "toggleForwards"
    | "toggleAutoReconnect"
    | "edit"
    | "duplicate"
    | "copyCommand"
    | "clearHostKey"
    | "delete";
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import type { RuleWithStatus } from "../types";
  import { isActiveStatus } from "../types";
  import Icon from "./Icon.svelte";

  interface Props {
    rule: RuleWithStatus;
    groups: string[];
    /** Where to open, in window coordinates (the click point or below the ⋯ button). */
    x: number;
    y: number;
    /**
     * Where the menu's bottom edge goes if it has to open upwards instead
     * (the click point, or the top of the ⋯ button).
     */
    flipY: number;
    onAction: (action: MenuAction) => void;
    onMove: (group: string) => void;
    onClose: () => void;
  }

  let { rule, groups, x, y, flipY, onAction, onMove, onClose }: Props = $props();

  let page = $state<"main" | "groups">("main");
  let newGroup = $state("");
  let el: HTMLDivElement | undefined = $state();

  const MARGIN = 8;
  let pos = $state<{ left: number; top: number } | null>(null);

  /**
   * Keeps the whole menu inside the window: opens upwards when there's no
   * room below, and slides left when it would run off the right edge. The
   * menu is `position: fixed`, so it never makes the list scroll.
   */
  function place() {
    if (!el) return;
    const w = el.offsetWidth;
    const h = el.offsetHeight;
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    let top = y;
    if (top + h > vh - MARGIN) {
      const above = flipY - h;
      top = above >= MARGIN ? above : Math.max(MARGIN, vh - h - MARGIN);
    }
    const left = Math.min(Math.max(MARGIN, x), vw - w - MARGIN);
    pos = { left, top };
  }

  // Re-place on open, on a new right-click while open, and when switching to
  // the (shorter or taller) "Move to group" page.
  $effect(() => {
    void [x, y, flipY, page, groups.length];
    place();
  });

  let active = $derived(isActiveStatus(rule.tunnelStatus));
  let hasForwards = $derived(rule.forwards.length > 0);

  function run(action: MenuAction) {
    onAction(action);
    onClose();
  }

  function move(group: string) {
    onMove(group);
    onClose();
  }

  onMount(() => {
    el?.querySelector<HTMLButtonElement>("button")?.focus();

    function onPointer(e: MouseEvent) {
      if (el && !el.contains(e.target as Node)) onClose();
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }
      if (page !== "main" || e.ctrlKey || e.metaKey || e.altKey) return;
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT") return;
      if (e.key === "h" || e.key === "H") {
        e.preventDefault();
        if (hasForwards) run("toggleForwards");
      } else if (e.key === "e" || e.key === "E") {
        e.preventDefault();
        run("edit");
      } else if (e.key === "Delete") {
        e.preventDefault();
        run("delete");
      }
    }
    // A fixed menu would float away from its rule, so close it instead.
    function onScrollOrResize(e: Event) {
      if (e.type === "scroll" && el && el.contains(e.target as Node)) return;
      onClose();
    }
    window.addEventListener("mousedown", onPointer, true);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("scroll", onScrollOrResize, true);
    window.addEventListener("resize", onScrollOrResize);
    return () => {
      window.removeEventListener("mousedown", onPointer, true);
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("scroll", onScrollOrResize, true);
      window.removeEventListener("resize", onScrollOrResize);
    };
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_click_events_have_key_events -->
<div
  class="menu"
  role="menu"
  tabindex="-1"
  aria-label="Actions for {rule.name}"
  bind:this={el}
  style="left: {pos?.left ?? x}px; top: {pos?.top ?? y}px; visibility: {pos ? 'visible' : 'hidden'}"
  onclick={(e) => e.stopPropagation()}
  oncontextmenu={(e) => e.preventDefault()}
>
  {#if page === "main"}
    <button type="button" role="menuitem" class="item" disabled={!hasForwards} onclick={() => run("toggleForwards")}>
      <Icon name={rule.forwardsHidden ? "eye" : "eyeOff"} size={15} />
      <span class="label">{rule.forwardsHidden ? "Show forwards" : "Hide forwards"}</span>
      <kbd>H</kbd>
    </button>
    {#if rule.autoReconnect}
      <button type="button" role="menuitem" class="item indent" onclick={() => run("toggleAutoReconnect")}>
        <span class="label">
          {rule.autoReconnectPaused ? "Resume auto-reconnect" : "Pause auto-reconnect"}
          {#if !rule.autoReconnectPaused}<span class="sub">Until PortDrill restarts</span>{/if}
        </span>
      </button>
    {/if}
    <div class="sep" role="separator"></div>
    <button type="button" role="menuitem" class="item indent" onclick={() => run("edit")}>
      <span class="label">
        Edit…
        {#if active}<span class="sub">The tunnel restarts when you save</span>{/if}
      </span>
      <kbd>E</kbd>
    </button>
    <button type="button" role="menuitem" class="item indent" onclick={() => run("duplicate")}>
      <span class="label">Duplicate</span>
    </button>
    <button type="button" role="menuitem" class="item indent" onclick={() => run("copyCommand")}>
      <span class="label">Copy as SSH command</span>
    </button>
    <button type="button" role="menuitem" class="item indent" onclick={() => (page = "groups")}>
      <span class="label">Move to group</span>
      <Icon name="chevronRight" size={12} strokeWidth={2.5} />
    </button>
    <div class="sep" role="separator"></div>
    <button type="button" role="menuitem" class="item indent" onclick={() => run("clearHostKey")}>
      <span class="label">
        Clear saved host key…
        {#if active}<span class="sub">Disconnects first</span>{/if}
      </span>
    </button>
    <button type="button" role="menuitem" class="item indent danger" onclick={() => run("delete")}>
      <span class="label">
        Delete
        <span class="sub">{active ? "Disconnects first · you can undo" : "You can undo"}</span>
      </span>
      <kbd>Del</kbd>
    </button>
  {:else}
    <button type="button" role="menuitem" class="item back" onclick={() => (page = "main")}>
      <span class="back-chevron"><Icon name="chevronRight" size={12} strokeWidth={2.5} /></span>
      <span class="label">Move to group</span>
    </button>
    <div class="sep" role="separator"></div>
    {#each groups as g (g)}
      <button type="button" role="menuitemradio" aria-checked={rule.group === g} class="item indent" onclick={() => move(g)}>
        <span class="label">{g}</span>
        {#if rule.group === g}<span class="check">✓</span>{/if}
      </button>
    {/each}
    <button type="button" role="menuitemradio" aria-checked={!rule.group} class="item indent" onclick={() => move("")}>
      <span class="label muted">No group</span>
      {#if !rule.group}<span class="check">✓</span>{/if}
    </button>
    <div class="sep" role="separator"></div>
    <form
      class="new-group"
      onsubmit={(e) => {
        e.preventDefault();
        if (newGroup.trim()) move(newGroup.trim());
      }}
    >
      <label class="sr-only" for="new-group-{rule.id}">New group name</label>
      <input id="new-group-{rule.id}" class="field-input" placeholder="New group…" bind:value={newGroup} />
    </form>
  {/if}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 70;
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    width: 252px;
    padding: 5px;
    border-radius: 10px;
    background: var(--bg-hover);
    border: 1px solid var(--border-input);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    cursor: default;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    text-align: left;
  }

  .item.indent {
    padding-left: 35px;
  }

  .item:hover:not(:disabled),
  .item:focus-visible {
    background: var(--bg-active);
    outline: none;
  }

  .item:disabled {
    opacity: 0.45;
  }

  .item.danger {
    color: var(--danger-text);
  }

  .label {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .label.muted {
    color: var(--text-secondary);
  }

  .sub {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .check {
    color: var(--text-secondary);
  }

  .back {
    color: var(--text-secondary);
    font-weight: 500;
  }

  .back-chevron {
    transform: rotate(180deg);
    display: flex;
  }

  .sep {
    height: 1px;
    background: var(--border-strong);
    margin: 4px 6px;
  }

  .new-group {
    padding: 4px;
  }

  .new-group .field-input {
    height: 32px;
  }
</style>
