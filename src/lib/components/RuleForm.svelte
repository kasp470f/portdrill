<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { Rule, Forward, ForwardType } from "../types";
  import { emptyRule, emptyForward, toCleanRule } from "../types";
  import { createRule, updateRule } from "../stores/rules";
  import { parseSshCommand } from "../sshParser";
  import AppButton from "./AppButton.svelte";

  interface Props {
    editRule?: Rule | null;
    onClose: () => void;
  }

  let { editRule = null, onClose }: Props = $props();

  let isEdit = $derived(editRule != null && editRule.id !== "");
  // svelte-ignore state_referenced_locally
  let form: Rule = $state(editRule != null ? toCleanRule(editRule) : emptyRule());
  let saving = $state(false);
  let error = $state("");

  let importMode = $state(false);
  let sshCommand = $state("");
  let importErrors = $state<string[]>([]);

  function addForward() {
    form.forwards = [...form.forwards, emptyForward()];
  }

  function removeForward(index: number) {
    form.forwards = form.forwards.filter((_, i) => i !== index);
  }

  function setForwardType(index: number, type: ForwardType) {
    form.forwards[index].forwardType = type;
  }

  async function pickKeyFile() {
    const selected = await open({
      multiple: false,
      directory: false,
      title: "Select SSH Private Key",
    });
    if (selected) {
      form.sshKeyPath = selected;
    }
  }

  function handleImport() {
    const result = parseSshCommand(sshCommand);
    if (result.errors.length > 0) {
      importErrors = result.errors;
      return;
    }
    form.sshHost = result.rule.sshHost;
    form.sshPort = result.rule.sshPort;
    form.sshUser = result.rule.sshUser;
    form.sshKeyPath = result.rule.sshKeyPath;
    form.forwards = result.rule.forwards;
    importErrors = [];
    importMode = false;
  }

  async function handleSubmit() {
    error = "";
    saving = true;

    try {
      if (isEdit) {
        await updateRule(form);
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
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="form-overlay" role="presentation" onclick={onClose} onkeydown={(e) => { if (e.key === "Escape") onClose(); }}>
  <!-- svelte-ignore a11y_no_static_element_interactions, a11y_interactive_supports_focus, a11y_click_events_have_key_events -->
  <div class="form-modal" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h2>{importMode ? "Import SSH Command" : isEdit ? "Edit Rule" : "New Rule"}</h2>
      {#if !isEdit}
        <button type="button" class="import-toggle" onclick={() => { importMode = !importMode; importErrors = []; }}>
          {importMode ? "Manual entry" : "Import from command"}
        </button>
      {/if}
    </div>

    {#if importMode}
      <div class="import-view">
        <p class="import-hint">Paste an SSH command to auto-fill the rule fields</p>
        <textarea
          class="import-textarea"
          bind:value={sshCommand}
          placeholder="ssh -N -p 30022 -L 3004:host:80 -L 2222:host:22 user@server"
          rows="4"
        ></textarea>
        {#if importErrors.length > 0}
          <div class="error-banner">
            {#each importErrors as err}
              <div>{err}</div>
            {/each}
          </div>
        {/if}
        <div class="form-actions">
          <AppButton type="plain" onclick={() => { importMode = false; importErrors = []; }}>
            Cancel
          </AppButton>
          <AppButton type="primary" onclick={handleImport}>
            Convert
          </AppButton>
        </div>
      </div>
    {:else}

    {#if error}
      <div class="error-banner">{error}</div>
    {/if}

    <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
      <div class="field">
        <label for="name">Name</label>
        <input id="name" type="text" bind:value={form.name} placeholder="My Tunnel" required />
      </div>

      <fieldset>
        <legend>SSH Connection</legend>
        <div class="row">
          <div class="field flex-2">
            <label for="ssh-host">Host</label>
            <input id="ssh-host" type="text" bind:value={form.sshHost} placeholder="hostname or IP" required />
          </div>
          <div class="field flex-1">
            <label for="ssh-port">Port</label>
            <input id="ssh-port" type="number" bind:value={form.sshPort} min="1" max="65535" />
          </div>
        </div>
        <div class="row">
          <div class="field flex-1">
            <label for="ssh-user">User</label>
            <input id="ssh-user" type="text" bind:value={form.sshUser} placeholder="root" required />
          </div>
          <div class="field flex-2">
            <label for="ssh-key">Key File <span class="optional">(optional — uses default if empty)</span></label>
            <div class="key-picker">
              <input id="ssh-key" type="text" bind:value={form.sshKeyPath} placeholder="~/.ssh/id_rsa" />
              <button type="button" class="browse-btn" onclick={pickKeyFile}>Browse</button>
            </div>
          </div>
        </div>
      </fieldset>

      <fieldset>
        <legend>Forwards</legend>

        {#each form.forwards as fwd, i}
          <div class="forward-entry">
            <div class="forward-header">
              <span class="forward-num">#{i + 1}</span>
              <div class="type-selector">
                <button type="button" class="type-btn" class:selected={fwd.forwardType === "local"} onclick={() => setForwardType(i, "local")}>
                  <span class="badge">L</span> Local
                </button>
                <button type="button" class="type-btn" class:selected={fwd.forwardType === "remote"} onclick={() => setForwardType(i, "remote")}>
                  <span class="badge">R</span> Remote
                </button>
                <button type="button" class="type-btn" class:selected={fwd.forwardType === "dynamic"} onclick={() => setForwardType(i, "dynamic")}>
                  <span class="badge">D</span> Dynamic
                </button>
              </div>
              {#if form.forwards.length > 1}
                <button type="button" class="remove-btn" onclick={() => removeForward(i)} title="Remove forward">✕</button>
              {/if}
            </div>

            {#if fwd.forwardType === "local"}
              <div class="row">
                <div class="field flex-1">
                  <label for="local-port-{i}">Local Port</label>
                  <input id="local-port-{i}" type="number" bind:value={fwd.localPort} min="1" max="65535" placeholder="" required />
                </div>
                <div class="field flex-2">
                  <label for="dest-host-{i}">Destination Host</label>
                  <input id="dest-host-{i}" type="text" bind:value={fwd.destinationHost} placeholder="" required />
                </div>
                <div class="field flex-1">
                  <label for="dest-port-{i}">Dest Port</label>
                  <input id="dest-port-{i}" type="number" bind:value={fwd.destinationPort} min="1" max="65535" placeholder="" required />
                </div>
              </div>
            {:else if fwd.forwardType === "remote"}
              <div class="row">
                <div class="field flex-1">
                  <label for="remote-port-{i}">Remote Port</label>
                  <input id="remote-port-{i}" type="number" bind:value={fwd.remotePort} min="1" max="65535" placeholder="" required />
                </div>
                <div class="field flex-2">
                  <label for="dest-host-r-{i}">Destination Host</label>
                  <input id="dest-host-r-{i}" type="text" bind:value={fwd.destinationHost} placeholder="" required />
                </div>
                <div class="field flex-1">
                  <label for="dest-port-r-{i}">Dest Port</label>
                  <input id="dest-port-r-{i}" type="number" bind:value={fwd.destinationPort} min="1" max="65535" placeholder="" required />
                </div>
              </div>
            {:else}
              <div class="field">
                <label for="socks-port-{i}">SOCKS Proxy Port</label>
                <input id="socks-port-{i}" type="number" bind:value={fwd.localPort} min="1" max="65535" placeholder="" required />
              </div>
            {/if}
          </div>
        {/each}

        <button type="button" class="add-forward-btn" onclick={addForward}>+ Add Forward</button>
      </fieldset>

      <div class="form-actions">
        <AppButton type="plain" onclick={onClose}>
          Cancel
        </AppButton>
        <AppButton type="primary" onclick={handleSubmit} disabled={saving}>
          {saving ? "Saving..." : isEdit ? "Save Changes" : "Create Rule"}
        </AppButton>
      </div>
    </form>
    {/if}
  </div>
</div>

<style>
  :global(.form-overlay) {
    position: fixed !important;
    inset: 0 !important;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    z-index: 100;
    overflow-y: auto;
    padding: 40px 16px;
  }

  :global(.form-modal) {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px;
    width: 100%;
    max-width: 620px;
    margin: auto 0;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  h2 {
    font-size: 1.25rem;
  }

  .import-toggle {
    font-size: 0.75rem;
    color: var(--accent);
    padding: 4px 10px;
    border-radius: 4px;
    transition: background 0.15s;
  }

  .import-toggle:hover {
    background: rgba(108, 140, 255, 0.1);
  }

  .import-view {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .import-hint {
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }

  .import-textarea {
    width: 100%;
    padding: 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 0.8125rem;
    font-family: "SF Mono", "Cascadia Code", "Fira Code", monospace;
    resize: vertical;
    line-height: 1.5;
  }

  .import-textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  .import-textarea::placeholder {
    color: var(--text-secondary);
    opacity: 0.5;
  }

  .error-banner {
    background: rgba(248, 113, 113, 0.1);
    border: 1px solid var(--danger);
    border-radius: 6px;
    padding: 8px 12px;
    margin-bottom: 16px;
    font-size: 0.875rem;
    color: var(--danger);
  }

  fieldset {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 16px 16px;
    margin-bottom: 16px;
  }

  legend {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    padding: 0 6px;
  }

  .field {
    margin-bottom: 12px;
  }

  label {
    display: block;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .optional {
    font-weight: 400;
    opacity: 0.6;
  }

  input {
    width: 100%;
    padding: 8px 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 0.875rem;
    transition: border-color 0.15s;
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  input::placeholder {
    color: var(--text-secondary);
    opacity: 0.5;
  }

  .row {
    display: flex;
    gap: 12px;
  }

  .flex-1 {
    flex: 1;
  }

  .flex-2 {
    flex: 2;
  }

  .forward-entry {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 10px;
  }

  .forward-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
  }

  .forward-num {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-secondary);
    min-width: 20px;
  }

  .type-selector {
    display: flex;
    gap: 6px;
    flex: 1;
  }

  .type-btn {
    flex: 1;
    padding: 5px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--text-secondary);
    font-size: 0.75rem;
    display: flex;
    align-items: center;
    gap: 4px;
    justify-content: center;
    transition: all 0.15s;
  }

  .type-btn.selected {
    border-color: var(--accent);
    color: var(--text-primary);
    background: rgba(108, 140, 255, 0.1);
  }

  .type-btn:hover:not(.selected) {
    background: var(--border)
  }

  .type-btn .badge {
    font-weight: 700;
  }

  .remove-btn {
    width: 26px;
    height: 26px;
    border-radius: 4px;
    background: transparent;
    color: var(--danger);
    font-size: 0.875rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .remove-btn:hover {
    background: rgba(248, 113, 113, 0.1);
  }

  .add-forward-btn {
    width: 100%;
    padding: 8px;
    background: transparent;
    border: 1px dashed var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 0.8125rem;
    transition: all 0.15s;
  }

  .add-forward-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .key-picker {
    display: flex;
    gap: 8px;
  }

  .key-picker input {
    flex: 1;
  }

  .browse-btn {
    padding: 8px 14px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 0.8125rem;
    white-space: nowrap;
  }

  .browse-btn:hover {
    background: var(--border);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }
</style>
