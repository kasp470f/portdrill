<script lang="ts">
  import type { TunnelStatus } from "../types";

  interface Props {
    status: TunnelStatus;
  }

  let { status }: Props = $props();

  let look = $derived.by(() => {
    switch (status.status) {
      case "connected":
        return { color: "var(--success)", halo: "var(--success-halo)", label: "Connected" };
      case "connecting":
        return { color: "var(--warning)", halo: "var(--warning-halo)", label: "Connecting…" };
      case "reconnecting":
        return {
          color: "var(--warning)",
          halo: "var(--warning-halo)",
          label: `Reconnecting (attempt ${status.attempt} of ${status.maxAttempts}): ${status.message}`,
        };
      case "error":
        return { color: "var(--danger)", halo: "var(--danger-halo)", label: `Failed: ${status.message}` };
      default:
        return { color: "var(--off)", halo: "transparent", label: "Off" };
    }
  });
</script>

<span class="status-dot" style="--dot: {look.color}; --halo: {look.halo}" title={look.label} aria-hidden="true"></span>

<style>
  .status-dot {
    position: relative;
    z-index: 1;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--dot);
    box-shadow: 0 0 0 3px var(--halo);
  }
</style>
