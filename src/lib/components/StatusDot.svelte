<script lang="ts">
  import type { TunnelStatus } from "../types";

  interface Props {
    status: TunnelStatus;
  }

  let { status }: Props = $props();

  let color = $derived.by(() => {
    switch (status.status) {
      case "connected":
        return "var(--success)";
      case "error":
        return "var(--danger)";
      default:
        return "var(--text-secondary)";
    }
  });

  let label = $derived.by(() => {
    switch (status.status) {
      case "connected":
        return "Connected";
      case "connecting":
        return "Connecting...";
      case "error":
        return `Error: ${status.message}`;
      default:
        return "Disconnected";
    }
  });
</script>

<span class="status-dot" style="--dot-color: {color}" title={label}>
  {#if status.status === "connecting"}
    <span class="pulse"></span>
  {/if}
</span>

<style>
  :global(.status-dot) {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background-color: var(--dot-color);
    position: relative;
    flex-shrink: 0;
  }

  :global(.status-dot .pulse) {
    position: absolute;
    inset: -3px;
    border-radius: 50%;
    background-color: var(--dot-color);
    opacity: 0.4;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes -global-pulse {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.4;
    }
    50% {
      transform: scale(1.6);
      opacity: 0;
    }
  }
</style>
