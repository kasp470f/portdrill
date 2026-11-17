<script lang="ts">
  import { open as openFile } from "@tauri-apps/plugin-dialog";
  import type { ForwardType, Rule } from "../types";
  import { emptyForward, emptyRule, toCleanRule } from "../types";
  import { rules, createRule, createRules, updateRuleRestarting } from "../stores/rules";
  import { showMessage } from "../stores/ui";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import GroupSelect from "./GroupSelect.svelte";
  import PasteCommands from "./PasteCommands.svelte";

  interface Props {
    rule: Rule | null;
    tab: "fill" | "paste";
    onClose: () => void;
  }

  let { rule, tab: initialTab, onClose }: Props = $props();

  // svelte-ignore state_referenced_locally
  const isEdit = rule !== null && rule.id !== "";
  // svelte-ignore state_referenced_locally
  let tab = $state<"fill" | "paste">(isEdit ? "fill" : initialTab);
  // svelte-ignore state_referenced_locally
  let form = $state<Rule>(rule ? toCleanRule(rule) : emptyRule());
  let saving = $state(false);
  let error = $state("");

  // Paste tab
  let pasteGroup = $state("");
  let pasted = $state<Rule[]>([]);

  let groups = $derived([...new Set($rules.map((r) => r.group).filter((g) => g))]);
  let wasConnected = $derived(
    isEdit && $rules.find((r) => r.id === form.id)?.tunnelStatus.status === "connected",
  );

  const types: { id: ForwardType; label: string }[] = [
    { id: "local", label: "Local" },
    { id: "remote", label: "Remote" },
    { id: "dynamic", label: "SOCKS" },
  ];

  function setType(i: number, type: ForwardType) {
    const f = form.forwards[i];
    if (type === "remote" && f.forwardType !== "remote") {
      f.remotePort = f.localPort;
      if (!f.destinationHost) f.destinationHost = "localhost";
    } else if (type !== "remote" && f.forwardType === "remote") {
      f.localPort = f.remotePort;
    }
    f.forwardType = type;
  }

  async function pickKeyFile() {
    const selected = await openFile({ multiple: false, directory: false, title: "Select SSH private key" });
    if (selected) form.sshKeyPath = selected;
  }

  function validate(r: Rule): string {
    if (!r.name.trim()) return "Give the rule a name.";
    if (!r.sshHost.trim()) return "Enter the SSH server's host name or IP.";
    if (!r.sshPort || r.sshPort < 1 || r.sshPort > 65535) return "The SSH port must be between 1 and 65535.";
    if (r.forwards.length === 0) return "Add at least one forward.";
    for (const [i, f] of r.forwards.entries()) {
      const n = r.forwards.length > 1 ? ` (forward ${i + 1})` : "";
      const port = f.forwardType === "remote" ? f.remotePort : f.localPort;
      if (!port || port < 1 || port > 65535) return `Enter a valid port${n}.`;
      if (f.forwardType !== "dynamic") {
        if (!f.destinationHost?.trim()) return `Enter the destination host${n}.`;
        if (!f.destinationPort || f.destinationPort < 1 || f.destinationPort > 65535)
          return `Enter a valid destination port${n}.`;
      }
    }
    return "";
  }

  async function submitFill() {
    error = validate(form);
    if (error) return;
    saving = true;
    try {
      if (isEdit) {
        await updateRuleRestarting(form);
      } else {
        await createRule(form);
      }
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function submitPaste() {
    if (pasted.length === 0) return;
    saving = true;
    try {
      const created = await createRules(pasted);
      showMessage(created.length === 1 ? "Created 1 rule" : `Created ${created.length} rules`);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function review(r: Rule) {
    form = toCleanRule(r);
    tab = "fill";
  }
</script>

<Modal title={isEdit ? "Edit rule" : "New rule"} {onClose}>
  {#snippet headerExtra()}
    {#if !isEdit}
      <div class="tabs" role="tablist" aria-label="How to create the rule">
        <button type="button" role="tab" aria-selected={tab === "fill"} class:on={tab === "fill"} onclick={() => (tab = "fill")}>
          Fill in
        </button>
        <button type="button" role="tab" aria-selected={tab === "paste"} class:on={tab === "paste"} onclick={() => (tab = "paste")}>
          Paste SSH command
        </button>
      </div>
    {/if}
  {/snippet}

  {#if error}
    <div class="error-banner" role="alert">{error}</div>
  {/if}

  {#if tab === "paste"}
    <PasteCommands {groups} bind:group={pasteGroup} bind:parsed={pasted} onReview={review} />
  {:else}
    <form
      id="rule-form"
      class="form"
      onsubmit={(e) => {
        e.preventDefault();
        void submitFill();
      }}
    >
      <div class="name-row">
        <label class="field-label">
          <span>Name</span>
          <input class="field-input" type="text" bind:value={form.name} placeholder="e.g. Postgres" />
        </label>
        <GroupSelect {groups} bind:value={form.group} />
      </div>

      <fieldset>
        <legend>
          <span class="legend-title">SSH server</span>
          <span class="legend-sub">The machine PortDrill signs in to. All forwards below go through it.</span>
        </legend>
        <div class="server-row">
          <label class="field-label">
            <span>User</span>
            <input class="field-input mono" type="text" bind:value={form.sshUser} placeholder="root" />
          </label>
          <span class="sep mono" aria-hidden="true">@</span>
          <label class="field-label">
            <span>Host</span>
            <input class="field-input mono" type="text" bind:value={form.sshHost} placeholder="hostname or IP" />
          </label>
          <span class="sep mono" aria-hidden="true">:</span>
          <label class="field-label">
            <span>Port</span>
            <input class="field-input mono" type="number" min="1" max="65535" bind:value={form.sshPort} />
          </label>
        </div>
        <label class="field-label">
          <span>Key file <span class="field-hint">optional</span></span>
          <span class="key-row">
            <input
              class="field-input mono"
              type="text"
              bind:value={form.sshKeyPath}
              placeholder="Leave empty to use your default key"
            />
            <button type="button" class="browse" onclick={pickKeyFile}>Browse…</button>
          </span>
        </label>
      </fieldset>

      <fieldset>
        <legend>
          <span class="legend-title">Forwards</span>
          <span class="legend-sub">Each one opens a port and says where its traffic goes.</span>
        </legend>

        {#each form.forwards as fwd, i (i)}
          <div class="forward">
            <div class="forward-head">
              <div class="types" role="radiogroup" aria-label="Forward type">
                {#each types as t (t.id)}
                  <button
                    type="button"
                    role="radio"
                    aria-checked={fwd.forwardType === t.id}
                    class:on={fwd.forwardType === t.id}
                    onclick={() => setType(i, t.id)}
                  >
                    {t.label}
                  </button>
                {/each}
              </div>
              <span class="grow"></span>
              {#if form.forwards.length > 1}
                <button
                  type="button"
                  class="remove"
                  aria-label="Remove this forward"
                  title="Remove this forward"
                  onclick={() => (form.forwards = form.forwards.filter((_, j) => j !== i))}
                >
                  <Icon name="trash" size={14} />
                </button>
              {/if}
            </div>

            {#if fwd.forwardType === "dynamic"}
              <label class="field-label socks">
                <span>Local port</span>
                <input class="field-input mono" type="number" min="1" max="65535" placeholder="1080" bind:value={fwd.localPort} />
              </label>
            {:else}
              <div class="route-row">
                {#if fwd.forwardType === "local"}
                  <label class="field-label">
                    <span>Local port</span>
                    <input class="field-input mono" type="number" min="1" max="65535" bind:value={fwd.localPort} />
                  </label>
                {:else}
                  <label class="field-label">
                    <span>Server port</span>
                    <input class="field-input mono" type="number" min="1" max="65535" bind:value={fwd.remotePort} />
                  </label>
                {/if}
                <span class="sep" aria-hidden="true"><Icon name="arrowRight" size={14} /></span>
                <label class="field-label">
                  <span>{fwd.forwardType === "remote" ? "To host (your side)" : "Destination host"}</span>
                  <input
                    class="field-input mono"
                    type="text"
                    bind:value={fwd.destinationHost}
                    placeholder={fwd.forwardType === "remote" ? "localhost" : "host as the server sees it"}
                  />
                </label>
                <span class="sep mono" aria-hidden="true">:</span>
                <label class="field-label">
                  <span>Port</span>
                  <input class="field-input mono" type="number" min="1" max="65535" bind:value={fwd.destinationPort} />
                </label>
              </div>
            {/if}
          </div>
        {/each}

        <button type="button" class="add" onclick={() => (form.forwards = [...form.forwards, emptyForward()])}>
          + Add another forward
        </button>
      </fieldset>
    </form>
  {/if}

  {#snippet footer()}
    {#if tab === "paste"}
      <span class="foot-note">PortDrill only reads the text. Nothing is run.</span>
      <button type="button" class="btn plain" onclick={onClose}>Cancel</button>
      <button type="button" class="btn primary" disabled={saving || pasted.length === 0} onclick={() => void submitPaste()}>
        {saving ? "Creating…" : pasted.length === 1 ? "Create 1 rule" : `Create ${pasted.length} rules`}
      </button>
    {:else}
      {#if isEdit}
        <span class="foot-note">{wasConnected ? "The tunnel restarts when you save." : ""}</span>
      {:else}
        <span class="foot-note"></span>
      {/if}
      <button type="button" class="btn plain" onclick={onClose}>Cancel</button>
      <button type="submit" form="rule-form" class="btn primary" disabled={saving}>
        {saving ? "Saving…" : isEdit ? "Save changes" : "Create rule"}
      </button>
    {/if}
  {/snippet}
</Modal>

<style>
  .tabs {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--radius);
    background: var(--bg-input);
    border: 1px solid var(--border-strong);
  }

  .tabs button {
    height: 28px;
    padding: 0 10px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .tabs button.on {
    background: var(--bg-badge);
    color: var(--text-primary);
  }

  .error-banner {
    padding: 8px 12px;
    border-radius: 7px;
    background: var(--danger-bg);
    border: 1px solid var(--danger-border);
    color: var(--danger-text);
    font-size: 13px;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  .name-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
  }

  fieldset {
    border: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  legend {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 4px;
  }

  .legend-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .legend-sub {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .server-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 14px minmax(0, 2fr) 14px 80px;
    gap: 6px;
    align-items: end;
    padding-top: 8px;
  }

  .route-row {
    display: grid;
    grid-template-columns: 96px 18px minmax(0, 1fr) 14px 80px;
    gap: 6px;
    align-items: end;
  }

  .sep {
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }

  .key-row {
    display: flex;
    gap: 8px;
  }

  .browse {
    height: 36px;
    padding: 0 12px;
    border-radius: 7px;
    border: 1px solid var(--border-button);
    background: #232327;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 500;
    flex-shrink: 0;
  }

  .browse:hover {
    background: var(--bg-active);
  }

  .forward {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-radius: var(--radius-card);
    border: 1px solid var(--border-strong);
    background: #161618;
  }

  .forward-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .types {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 7px;
    background: var(--bg-input);
    border: 1px solid var(--border-strong);
  }

  .types button {
    height: 26px;
    padding: 0 10px;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .types button.on {
    background: var(--bg-badge);
    color: var(--text-primary);
  }

  .grow {
    flex-grow: 1;
  }

  .remove {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .remove:hover {
    background: var(--bg-active);
    color: var(--danger-text);
  }

  .socks {
    width: 140px;
  }

  .add {
    height: 36px;
    border-radius: var(--radius);
    border: 1px dashed var(--border-button);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
  }

  .add:hover {
    border-color: var(--accent);
    color: #b3c4ff;
  }

  .foot-note {
    flex-grow: 1;
    font-size: 12px;
    color: var(--text-muted);
  }

  .btn {
    height: 36px;
    padding: 0 14px;
    border-radius: var(--radius);
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
  }

  .btn.plain {
    border: 1px solid var(--border-button);
    color: var(--text-strong);
    font-weight: 500;
  }

  .btn.plain:hover {
    background: var(--bg-hover);
  }

  .btn.primary {
    background: var(--accent);
    color: var(--accent-text);
    padding: 0 16px;
  }

  .btn.primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .btn:disabled {
    opacity: 0.5;
  }

  input[type="number"]::-webkit-inner-spin-button,
  input[type="number"]::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
</style>
