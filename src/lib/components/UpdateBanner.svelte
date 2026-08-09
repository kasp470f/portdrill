<script lang="ts">
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import AppButton from "./AppButton.svelte";

  let updateAvailable = $state(false);
  let newVersion = $state("");
  let downloading = $state(false);
  let checking = $state(false);
  let progress = $state("");
  let dismissed = $state(false);
  let noUpdate = $state(false);
  let errorMsg = $state("");

  async function checkForUpdate() {
    checking = true;
    noUpdate = false;
    dismissed = false;
    errorMsg = "";

    try {
      const update = await check();
      if (update) {
        updateAvailable = true;
        newVersion = update.version;
        (window as any).__pendingUpdate = update;
      } else {
        noUpdate = true;
        setTimeout(() => { noUpdate = false; }, 4000);
      }
    } catch (e: any) {
      errorMsg = String(e?.message ?? e);
      setTimeout(() => { errorMsg = ""; }, 8000);
    } finally {
      checking = false;
    }
  }

  onMount(() => {
    checkForUpdate();
    listen("check-for-update", () => checkForUpdate());
  });

  async function installUpdate() {
    const update = (window as any).__pendingUpdate;
    if (!update) return;

    downloading = true;
    progress = "Downloading...";

    try {
      await update.downloadAndInstall((event: any) => {
        if (event.event === "Started" && event.data.contentLength) {
          progress = `Downloading (${Math.round(event.data.contentLength / 1024)}KB)...`;
        } else if (event.event === "Finished") {
          progress = "Installing...";
        }
      });
      await relaunch();
    } catch (e) {
      console.error("Update failed:", e);
      progress = "Update failed";
      downloading = false;
    }
  }
</script>

{#if checking}
  <div class="update-banner checking">
    <span class="update-text">Checking for updates...</span>
  </div>
{:else if updateAvailable && !dismissed}
  <div class="update-banner">
    <span class="update-text">
      v{newVersion} is available
    </span>
    {#if downloading}
      <span class="update-progress">{progress}</span>
    {:else}
      <AppButton type="primary" onclick={installUpdate}>Update now</AppButton>
      <AppButton type="plain" onclick={() => dismissed = true}>Later</AppButton>
    {/if}
  </div>
{:else if noUpdate}
  <div class="update-banner uptodate">
    <span class="update-text">You're on the latest version</span>
  </div>
{:else if errorMsg}
  <div class="update-banner error">
    <span class="update-text">Update check failed: {errorMsg}</span>
  </div>
{/if}

<style lang="scss">
  :global(.update-banner) {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-card) 90%);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--border) 70%);
    border-radius: var(--radius);
    margin-bottom: 16px;
  }

  :global(.update-banner.uptodate) {
    background: color-mix(in srgb, var(--success) 8%, var(--bg-card) 92%);
    border-color: color-mix(in srgb, var(--success) 25%, var(--border) 75%);
  }

  :global(.update-banner.checking) {
    opacity: 0.7;
  }

  :global(.update-banner.error) {
    background: color-mix(in srgb, var(--danger) 8%, var(--bg-card) 92%);
    border-color: color-mix(in srgb, var(--danger) 25%, var(--border) 75%);
  }

  :global(.update-banner .update-text) {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary);
    flex: 1;
  }

  :global(.update-banner .update-progress) {
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }
</style>
