<script lang="ts">
  import RuleList from "./lib/components/RuleList.svelte";
  import UpdateBanner from "./lib/components/UpdateBanner.svelte";
  import AppHeader from "./lib/components/AppHeader.svelte";
</script>

<main>
  <AppHeader />
  <UpdateBanner />
  <RuleList />
</main>

<style>
  :global(main) {
    max-width: 960px;
    margin: 0 auto;
    padding: 28px 48px 40px;
    min-height: 100vh;
    max-height: calc(100vh - (28px + 40px));
    display: flex;
    flex-direction: column;
  }
</style>
