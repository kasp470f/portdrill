<script lang="ts">
  import packageJson from "../package.json";
  import AppHeader from "./lib/components/AppHeader.svelte";
  import UpdateBanner from "./lib/components/UpdateBanner.svelte";
  import SshMissingBanner from "./lib/components/SshMissingBanner.svelte";
  import RuleList from "./lib/components/RuleList.svelte";
  import RuleDialog from "./lib/components/RuleDialog.svelte";
  import HostKeyDialog from "./lib/components/HostKeyDialog.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import { dialog, closeDialog, openNewRule } from "./lib/stores/ui";

  const version = packageJson.version ?? "dev";

  let query = $state("");
  let filter = $state<"all" | "active" | "issues">("all");
  let searchEl: HTMLInputElement | undefined = $state();

  function onKey(e: KeyboardEvent) {
    if ($dialog) return;
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      searchEl?.focus();
      searchEl?.select();
    } else if (mod && e.key.toLowerCase() === "n") {
      e.preventDefault();
      openNewRule("fill");
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="app">
  <AppHeader bind:query bind:filter bind:searchEl />
  <div class="banners">
    <UpdateBanner />
  </div>
  <SshMissingBanner />
  <main class="scroll">
    <RuleList {query} {filter} />
  </main>
  <footer>
    <span>v{version}</span>
  </footer>
</div>

{#if $dialog?.kind === "rule"}
  <RuleDialog rule={$dialog.rule} tab={$dialog.tab} onClose={closeDialog} />
{:else if $dialog?.kind === "hostKey"}
  <HostKeyDialog ruleId={$dialog.ruleId} reconnect={$dialog.reconnect} onClose={closeDialog} />
{/if}

<Toast />

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* Everything below the header lines up with its 20px side margins. */
  .banners {
    padding: 0 20px;
  }

  .banners:empty {
    display: none;
  }

  .scroll {
    flex-grow: 1;
    min-height: 0;
    overflow-y: auto;
    /* 20px on the left; on the right 14px plus the always-reserved 6px
       scrollbar, so cards end exactly where the search and buttons do,
       whether or not the list is long enough to scroll. */
    padding: 0 14px 0 20px;
    scrollbar-gutter: stable;
  }

  .scroll::-webkit-scrollbar {
    width: 6px;
  }

  .scroll::-webkit-scrollbar-track {
    margin: 12px 0;
  }

  footer {
    height: 30px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: 0 16px;
    border-top: 1px solid #222226;
    background: var(--bg-titlebar);
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
