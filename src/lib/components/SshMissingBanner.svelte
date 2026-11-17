<script lang="ts">
  import { onMount } from "svelte";
  import { rules, sshAvailable } from "../stores/rules";
  import { sshMissing } from "../stores/ui";

  let checking = $state(false);

  async function check() {
    checking = true;
    try {
      sshMissing.set(!(await sshAvailable()));
    } catch {
      /* leave the current state */
    } finally {
      checking = false;
    }
  }

  onMount(() => {
    void check();
  });

  // A connect that fails because ssh can't start also flips the banner on.
  $effect(() => {
    if ($rules.some((r) => r.tunnelStatus.status === "error" && r.tunnelStatus.kind === "sshMissing")) {
      sshMissing.set(true);
    }
  });

  const isWindows = navigator.userAgent.includes("Windows");
</script>

{#if $sshMissing}
  <div class="banner" role="alert">
    <strong>OpenSSH isn't installed</strong>
    <p>
      PortDrill uses your computer's <span class="mono">ssh</span> program and couldn't find it.
      {#if isWindows}
        Add it under Settings › System › Optional features › OpenSSH Client, then click Check again.
      {:else}
        Install OpenSSH with your system's package manager, then click Check again.
      {/if}
    </p>
    <div class="actions">
      <button type="button" class="primary" disabled={checking} onclick={check}>
        {checking ? "Checking…" : "Check again"}
      </button>
    </div>
  </div>
{/if}

<style>
  .banner {
    margin: 0 12px 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    border-radius: var(--radius-card);
    background: var(--danger-bg);
    border: 1px solid var(--danger-border);
  }

  strong {
    font-size: 14px;
    font-weight: 600;
    color: var(--danger-text);
  }

  p {
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-strong);
  }

  .actions button {
    height: 30px;
    padding: 0 12px;
    border-radius: 7px;
    font-size: 12px;
    font-weight: 600;
    background: var(--danger-text);
    color: #2a0d10;
  }

  .actions button:hover:not(:disabled) {
    background: var(--danger-strong);
  }
</style>
