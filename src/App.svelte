<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";
  import RuleList from "./lib/components/RuleList.svelte";
  import UpdateBanner from "./lib/components/UpdateBanner.svelte";
  import appIcon from "../src-tauri/icons/64x64.png";

  let version = $state("");

  onMount(async () => {
    for (let i = 0; i < 3; i++) {
      try {
        version = await getVersion();
        return;
      } catch {
        await new Promise((r) => setTimeout(r, 500));
      }
    }
    version = "dev";
  });
</script>

<main>
  <header>
    <div class="title-row">
      <img class="app-icon" src={appIcon} alt="PortDrill icon" />
      <div>
        <h1>PortDrill</h1>
        <p>SSH Port Forwarding Manager</p>
      </div>
      {#if version}
        <span class="version">v{version}</span>
      {/if}
    </div>
  </header>
  <UpdateBanner />
  <RuleList />
</main>

<style>
  :global(main) {
    max-width: 960px;
    margin: 0 auto;
    padding: 28px 48px 40px;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  :global(main header) {
    margin-bottom: 32px;
  }

  :global(.title-row) {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  :global(.app-icon) {
    height: 52px;
    flex-shrink: 0;
  }

  :global(main header h1) {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  :global(main header p) {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  :global(.version) {
    font-size: 0.7rem;
    color: var(--text-secondary);
    opacity: 0.6;
    margin-left: auto;
    align-self: flex-start;
    padding-top: 4px;
  }
</style>
