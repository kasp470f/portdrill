<script lang="ts">
  import RuleList from "./lib/components/RuleList.svelte";
  import UpdateBanner from "./lib/components/UpdateBanner.svelte";
  import appIcon from "../src-tauri/icons/64x64.png";
  import packageJson from "../package.json";

  const fallbackVersion = packageJson.version ?? "dev";
</script>

<main>
  <header>
    <div class="title-row">
      <img class="app-icon" src={appIcon} alt="PortDrill icon" />
      <div>
        <h1>PortDrill</h1>
        <p>SSH Port Forwarding Manager</p>
      </div>
      {#if fallbackVersion}
        <span class="version">v{fallbackVersion}</span>
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
    user-select: none;
    pointer-events: none;
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
    user-select: none;
  }
</style>
