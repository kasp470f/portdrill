<script lang="ts">
  import appIcon from "../../../src-tauri/icons/64x64.png";
  import packageJson from "../../../package.json";

  const fallbackVersion = packageJson.version ?? "dev";
</script>

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

<style lang="scss">
  :global(header) {
    margin-bottom: 32px;
  }

  :global(.title-row) {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  :global(header h1) {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  :global(header p) {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  :global(.app-icon) {
    height: 58px;
    flex-shrink: 0;
    user-select: none;
    pointer-events: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background-color: var(--bg-card);
    padding: 4px;
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
