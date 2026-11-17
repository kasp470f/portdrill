<script lang="ts">
  import { onMount } from "svelte";
  import type { HostKeyInfo } from "../types";
  import { rules, clearHostKey, inspectHostKey, toggleRule } from "../stores/rules";
  import { showMessage } from "../stores/ui";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    ruleId: string;
    reconnect: boolean;
    onClose: () => void;
  }

  let { ruleId, reconnect, onClose }: Props = $props();

  let rule = $derived($rules.find((r) => r.id === ruleId));
  let where = $derived(rule ? (rule.sshPort === 22 ? rule.sshHost : `${rule.sshHost}:${rule.sshPort}`) : "");
  let changed = $derived(rule?.tunnelStatus.status === "error" && rule.tunnelStatus.kind === "hostKeyChanged");

  let info = $state<HostKeyInfo | null>(null);
  let scanning = $state(true);
  let confirmed = $state(false);
  let working = $state(false);
  let error = $state("");

  onMount(async () => {
    try {
      info = await inspectHostKey(ruleId);
    } catch (e) {
      info = {
        target: where,
        fingerprints: [],
        scanError: String(e),
        savedFingerprints: [],
      };
    } finally {
      scanning = false;
    }
  });

  async function trust() {
    working = true;
    error = "";
    try {
      await clearHostKey(ruleId);
      onClose();
      if (reconnect) {
        await toggleRule(ruleId).catch(() => {});
      } else {
        showMessage(`Removed the saved host key for ${where}`);
      }
    } catch (e) {
      error = String(e);
    } finally {
      working = false;
    }
  }
</script>

<Modal title={changed ? "Review the new host key" : "Clear the saved host key?"} {onClose} size="auto">
  {#snippet icon()}
    <span class="badge-icon" aria-hidden="true"><Icon name="shieldAlert" size={18} /></span>
  {/snippet}

  <div class="intro">
    {#if changed}
      <p>
        <span class="mono">{where}</span> is presenting a different key than the one you saved. That's expected if the
        server was rebuilt or reinstalled.
      </p>
      <p class="warn">If nothing changed on the server, stop here: someone may be intercepting the connection.</p>
    {:else}
      <p>
        This removes the saved key for <span class="mono">{where}</span> from known_hosts. The next connection will trust
        whichever key the server offers.
      </p>
    {/if}
  </div>

  <div class="keys">
    {#if scanning}
      <div class="key-row"><span class="muted">Checking what the server is presenting…</span></div>
    {:else if info}
      {#if info.savedFingerprints.length > 0}
        <div class="key-row">
          <span class="key-label">Saved key</span>
          <span class="key-values">
            <span class="key-sub">in known_hosts</span>
            {#each info.savedFingerprints as fp (fp)}
              <span class="fp mono old">{fp}</span>
            {/each}
          </span>
        </div>
      {/if}
      <div class="key-row">
        <span class="key-label now">Server now</span>
        <span class="key-values">
          {#if info.fingerprints.length > 0}
            <span class="key-sub">offered just now</span>
            {#each info.fingerprints as fp (fp)}
              <span class="fp mono">{fp}</span>
            {/each}
          {:else}
            <span class="muted">Couldn't read the server's key: {info.scanError ?? "no answer"}</span>
          {/if}
        </span>
      </div>
    {/if}
  </div>

  <p class="how">
    Compare the new fingerprint with what your hosting provider, deploy log or server admin reports. On the server you can
    run <span class="mono">ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub</span>.
  </p>

  <label class="confirm">
    <input type="checkbox" bind:checked={confirmed} />
    I've checked that this fingerprint matches
  </label>

  {#if error}
    <p class="failure" role="alert">{error}</p>
  {/if}

  {#snippet footer()}
    <span class="cmd mono">runs ssh-keygen -R</span>
    <button type="button" class="btn plain" onclick={onClose}>Cancel</button>
    <button
      type="button"
      class="btn warning"
      disabled={!confirmed || working || scanning}
      title={confirmed ? undefined : "Tick the checkbox first"}
      onclick={() => void trust()}
    >
      {working ? "Working…" : reconnect ? "Trust new key & reconnect" : "Clear saved key"}
    </button>
  {/snippet}
</Modal>

<style>
  .badge-icon {
    width: 32px;
    height: 32px;
    border-radius: 9px;
    background: #33270a;
    color: var(--warning);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .intro {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-strong);
  }

  .intro .mono {
    font-size: 12px;
  }

  .warn {
    color: #fcd34d;
  }

  .keys {
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-card);
    background: var(--bg-input);
  }

  .key-row {
    display: grid;
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 12px;
    padding: 12px 14px;
    align-items: baseline;
  }

  .key-row + .key-row {
    border-top: 1px solid var(--border);
  }

  .key-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .key-label.now {
    font-weight: 600;
    color: #fcd34d;
  }

  .key-values {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .key-sub {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .fp {
    font-size: 12px;
    color: var(--text-primary);
    overflow-wrap: anywhere;
  }

  .fp.old {
    color: var(--text-secondary);
    text-decoration: line-through;
  }

  .muted {
    grid-column: 1 / -1;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .how {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  .how .mono {
    color: var(--text-strong);
  }

  .confirm {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--border-button);
    background: var(--bg-hover);
    font-size: 13px;
    cursor: pointer;
  }

  .confirm input {
    width: 16px;
    height: 16px;
    accent-color: var(--warning);
  }

  .failure {
    font-size: 13px;
    color: var(--danger-text);
    overflow-wrap: anywhere;
  }

  .cmd {
    flex-grow: 1;
    font-size: 11px;
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

  .btn.warning {
    background: var(--warning);
    color: #1a1305;
    padding: 0 16px;
  }

  .btn.warning:disabled {
    background: #5c4a1a;
    color: #c9b88a;
    opacity: 1;
  }
</style>
