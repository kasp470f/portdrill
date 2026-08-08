<script lang="ts">
  import RuleList from "./lib/components/RuleList.svelte";
</script>

<main>
  <header>
    <h1>TunnelApp</h1>
    <p>SSH Port Forwarding Manager</p>
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
