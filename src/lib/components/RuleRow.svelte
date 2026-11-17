<script lang="ts">
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { open as openFile } from "@tauri-apps/plugin-dialog";
  import type { RuleWithStatus } from "../types";
  import { toCleanRule } from "../types";
  import { formatSshCommand } from "../sshParser";
  import { describeError, type ErrorAction } from "../errorCopy";
  import {
    duplicateRule,
    setForwardsHidden,
    setRuleGroup,
    toggleRule,
    updateRule,
  } from "../stores/rules";
  import { openEditRule, openHostKey, requestDelete, showMessage } from "../stores/ui";
  import Icon from "./Icon.svelte";
  import StatusDot from "./StatusDot.svelte";
  import Toggle from "./Toggle.svelte";
  import ForwardTree from "./ForwardTree.svelte";
  import RuleMenu, { type MenuAction } from "./RuleMenu.svelte";

  interface Props {
    rule: RuleWithStatus;
    allRules: RuleWithStatus[];
    groups: string[];
    first: boolean;
    last: boolean;
    dragging?: boolean;
    onGripDown?: (e: MouseEvent) => void;
  }

  let { rule, allRules, groups, first, last, dragging = false, onGripDown }: Props = $props();

  let menu = $state<{ x: number; y: number; flipY: number } | null>(null);
  let rowEl: HTMLDivElement | undefined = $state();

  let status = $derived(rule.tunnelStatus.status);
  let active = $derived(status === "connected" || status === "connecting");
  let toggleState = $derived<"on" | "off" | "connecting">(
    status === "connecting" ? "connecting" : status === "connected" ? "on" : "off",
  );
  let host = $derived(
    `${rule.sshUser ? rule.sshUser + "@" : ""}${rule.sshHost}${rule.sshPort !== 22 ? ":" + rule.sshPort : ""}`,
  );
  let error = $derived(
    rule.tunnelStatus.status === "error"
      ? {
          raw: rule.tunnelStatus.message,
          ...describeError(rule.tunnelStatus.kind ?? "other", rule.tunnelStatus.message, rule, allRules),
        }
      : null,
  );
  let hiddenText = $derived(
    rule.forwards.length === 1 ? "1 forward hidden" : `${rule.forwards.length} forwards hidden`,
  );

  async function connectOrDisconnect(id = rule.id) {
    try {
      await toggleRule(id);
    } catch (e) {
      // Connect failures arrive as an error status on the row; only surface
      // the ones that wouldn't (e.g. a refused double click).
      const msg = String(e);
      if (!msg.includes("already")) console.error("Toggle failed:", msg);
    }
  }

  async function copy(text: string, what = text) {
    try {
      await writeText(text);
      showMessage(`Copied ${what}`);
    } catch (e) {
      showMessage(`Couldn't copy: ${e}`, "error");
    }
  }

  // Menu coordinates are window coordinates; RuleMenu keeps itself on screen.
  function openMenuAt(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    menu = { x: e.clientX, y: e.clientY, flipY: e.clientY };
  }

  function openMenuFromButton(e: MouseEvent) {
    e.stopPropagation();
    if (menu) {
      menu = null;
      return;
    }
    const btn = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menu = { x: btn.right - 252, y: btn.bottom + 4, flipY: btn.top - 4 };
  }

  async function onMenu(action: MenuAction) {
    switch (action) {
      case "toggleForwards":
        try {
          await setForwardsHidden(rule.id, !rule.forwardsHidden);
        } catch (e) {
          showMessage(String(e), "error");
        }
        break;
      case "edit":
        openEditRule(toCleanRule(rule));
        break;
      case "duplicate":
        await duplicateRule(rule);
        break;
      case "copyCommand":
        await copy(formatSshCommand(toCleanRule(rule)), "SSH command");
        break;
      case "clearHostKey":
        if (active) await connectOrDisconnect();
        openHostKey(rule.id, false);
        break;
      case "delete":
        requestDelete(toCleanRule(rule));
        break;
    }
  }

  async function onMove(group: string) {
    try {
      await setRuleGroup(rule.id, group);
    } catch (e) {
      showMessage(String(e), "error");
    }
  }

  async function chooseKeyAndRetry(path: string | null) {
    try {
      await updateRule({ ...toCleanRule(rule), sshKeyPath: path ?? "" });
      await connectOrDisconnect();
    } catch (e) {
      showMessage(String(e), "error");
    }
  }

  async function onErrorAction(id: ErrorAction) {
    switch (id) {
      case "reviewKey":
        openHostKey(rule.id, true);
        break;
      case "edit":
        openEditRule(toCleanRule(rule));
        break;
      case "retry":
      case "reconnect":
        await connectOrDisconnect();
        break;
      case "chooseKey": {
        const selected = await openFile({ multiple: false, directory: false, title: "Select SSH private key" });
        if (selected) await chooseKeyAndRetry(selected);
        break;
      }
      case "defaultKey":
        await chooseKeyAndRetry(null);
        break;
      case "stopOther":
        if (error?.otherRuleId) {
          await connectOrDisconnect(error.otherRuleId);
          await connectOrDisconnect();
        }
        break;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="row"
  class:first
  class:last
  class:menu-open={menu !== null}
  class:dragging
  bind:this={rowEl}
  oncontextmenu={openMenuAt}
>
  <div class="top">
    <StatusDot status={rule.tunnelStatus} />
    <div class="title">
      <span class="name">{rule.name}</span>
      <span class="host mono">{host}</span>
      {#if status === "connecting"}
        <span class="status connecting">Connecting…</span>
      {:else if status === "error"}
        <span class="status failed">Failed</span>
      {/if}
    </div>
    <div class="actions">
      {#if onGripDown}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span
          class="icon-btn grip"
          title="Drag to reorder"
          aria-hidden="true"
          onmousedown={(e) => {
            e.preventDefault();
            onGripDown(e);
          }}
        >
          <Icon name="grip" />
        </span>
      {/if}
      <button
        type="button"
        class="icon-btn"
        class:pressed={menu !== null}
        aria-haspopup="menu"
        aria-expanded={menu !== null}
        aria-label="More actions"
        title="More actions (or right-click the rule)"
        onclick={openMenuFromButton}
      >
        <Icon name="more" />
      </button>
    </div>
    <Toggle
      state={toggleState}
      label={status === "connecting" ? `Connecting to ${rule.name}…` : active ? `Disconnect ${rule.name}` : `Connect ${rule.name}`}
      onToggle={() => void connectOrDisconnect()}
    />
  </div>

  {#if rule.forwards.length > 0}
    {#if rule.forwardsHidden}
      <button type="button" class="hidden-line" onclick={() => onMenu("toggleForwards")}>
        <Icon name="eyeOff" size={14} />
        {hiddenText} · <span class="link">Show</span>
      </button>
    {:else}
      <ForwardTree forwards={rule.forwards} />
    {/if}
  {/if}

  {#if error}
    <div class="error" role="alert">
      <strong>{error.title}</strong>
      <p>{error.explain}</p>
      {#if error.meta}<span class="meta">{error.meta}</span>{/if}
      <div class="raw">
        <span class="raw-label">ssh said</span>
        <code>{error.raw}</code>
      </div>
      <div class="error-actions">
        {#each error.actions as action, i (action.id)}
          <button type="button" class="err-btn" class:primary={i === 0} onclick={() => void onErrorAction(action.id)}>
            {action.label}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if menu}
    <RuleMenu
      {rule}
      {groups}
      x={menu.x}
      y={menu.y}
      flipY={menu.flipY}
      onAction={(a) => void onMenu(a)}
      onMove={(g) => void onMove(g)}
      onClose={() => (menu = null)}
    />
  {/if}
</div>

<style>
  .row {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 12px 12px 14px;
    border-top: 1px solid var(--border);
  }

  .row.first {
    border-top: none;
    border-top-left-radius: var(--radius-card);
    border-top-right-radius: var(--radius-card);
  }

  .row.last {
    border-bottom-left-radius: var(--radius-card);
    border-bottom-right-radius: var(--radius-card);
  }

  .row:hover,
  .row.menu-open {
    background: var(--bg-hover);
  }

  .row.dragging {
    opacity: 0.35;
  }

  .grip {
    cursor: grab;
    user-select: none;
  }

  .grip:active {
    cursor: grabbing;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 30px;
  }

  .title {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-grow: 1;
    min-width: 0;
  }

  .name {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
  }

  .host {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .status {
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }

  .status.connecting {
    color: var(--warning);
  }

  .status.failed {
    color: var(--danger-text);
  }

  .actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.12s;
  }

  .row:hover .actions,
  .row:focus-within .actions,
  .row.menu-open .actions {
    opacity: 1;
  }

  .icon-btn {
    width: 30px;
    height: 30px;
    border-radius: 6px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon-btn:hover,
  .icon-btn.pressed {
    background: var(--bg-active);
    color: var(--text-primary);
  }

  .hidden-line {
    margin-left: 14px;
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px;
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .hidden-line:hover {
    color: var(--text-primary);
    background: var(--bg-active);
  }

  .link {
    color: var(--link);
  }

  .error {
    margin-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--radius-card);
    background: var(--danger-bg);
    border: 1px solid var(--danger-border);
  }

  .error strong {
    font-size: 13px;
    font-weight: 600;
    color: var(--danger-text);
  }

  .error p {
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-strong);
  }

  .meta {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .raw {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .raw-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .raw code {
    display: block;
    padding: 7px 9px;
    border-radius: 6px;
    background: #1a0f11;
    border: 1px solid #3d2226;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-strong);
    overflow-wrap: anywhere;
  }

  .error-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .err-btn {
    height: 30px;
    padding: 0 12px;
    border-radius: 7px;
    border: 1px solid #7a3a3f;
    color: var(--danger-strong);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }

  .err-btn:hover {
    background: #3a1d20;
  }

  .err-btn.primary {
    background: var(--danger-text);
    border-color: var(--danger-text);
    color: #2a0d10;
  }

  .err-btn.primary:hover {
    background: var(--danger-strong);
  }
</style>
