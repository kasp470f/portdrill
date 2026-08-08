<script lang="ts">
  import RuleList from "./lib/components/RuleList.svelte";
  import appIcon from "../src-tauri/icons/64x64.png";
</script>

<main>
  <header>
    <div class="title-row">
      <img class="app-icon" src={appIcon} alt="TunnelApp icon" />
      <div>
        <h1>TunnelApp</h1>
        <p>SSH Port Forwarding Manager</p>
      </div>
    </div>
  </header>
  <RuleList />
</main>

<style>
  :global(main) {
    max-width: 960px;
    margin: 0 auto;
    padding: 40px 48px;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  :global(main header) {
    margin-bottom: 32px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .app-icon {
    width: 40px;
    height: 40px;
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
    margin-top: 2px;
  }
</style>
