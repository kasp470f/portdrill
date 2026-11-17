<script lang="ts">
  import type { Forward } from "../types";
  import { forwardTypeLabel, sortedForwards } from "../types";

  /**
   * A rule's forwards as chips hanging off the status dot on thin tree lines.
   * Display only: the chips aren't clickable. `reach` is how far the first
   * line climbs back up towards the dot.
   */
  interface Props {
    forwards: Forward[];
    reach?: number;
  }

  let { forwards, reach = 11 }: Props = $props();

  const LEAF = 30;

  let leaves = $derived(
    sortedForwards(forwards).map((fwd, i, all) => {
      const first = i === 0;
      const extra = first ? reach : 0;
      return {
        fwd,
        last: i === all.length - 1,
        top: first ? -reach : 0,
        lineHeight: LEAF + extra,
        elbowHeight: LEAF / 2 + 1 + extra,
        badge: forwardTypeLabel(fwd.forwardType),
        port: fwd.forwardType === "remote" ? fwd.remotePort : fwd.localPort,
        arrow: fwd.forwardType === "dynamic" ? "·" : "→",
        dest:
          fwd.forwardType === "dynamic"
            ? "SOCKS5"
            : `${fwd.destinationHost ?? "?"}:${fwd.destinationPort ?? "?"}`,
      };
    }),
  );
</script>

<div class="tree">
  {#each leaves as leaf, i (i)}
    <div class="leaf">
      {#if leaf.last}
        <span class="elbow" style="top: {leaf.top}px; height: {leaf.elbowHeight}px" aria-hidden="true"></span>
      {:else}
        <span class="stem" style="top: {leaf.top}px; height: {leaf.lineHeight}px" aria-hidden="true"></span>
        <span class="tick" aria-hidden="true"></span>
      {/if}
      <span class="chip">
        <span class="badge">{leaf.badge}</span>
        <span class="port">{leaf.port ?? "?"}</span>
        <span class="arrow">{leaf.arrow}</span>
        <span class="dest">{leaf.dest}</span>
      </span>
    </div>
  {/each}
</div>

<style>
  .tree {
    display: flex;
    flex-direction: column;
    /* 3px puts the 1px line under the middle of the 8px status dot as it
       actually renders (3.5px landed a pixel to the right on screen). */
    margin: -4px 0 0 3px;
    user-select: none;
  }

  .leaf {
    position: relative;
    height: 30px;
    padding-left: 22px;
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .stem {
    position: absolute;
    left: 0;
    width: 1px;
    background: var(--tree-line);
  }

  .tick {
    position: absolute;
    left: 0;
    top: 15px;
    width: 16px;
    height: 1px;
    background: var(--tree-line);
  }

  .elbow {
    position: absolute;
    left: 0;
    width: 16px;
    border-left: 1px solid var(--tree-line);
    border-bottom: 1px solid var(--tree-line);
    border-bottom-left-radius: 7px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    max-width: 100%;
    padding: 0 8px 0 4px;
    border-radius: 6px;
    border: 1px solid var(--border-strong);
    background: var(--bg-chip);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: default;
  }

  .badge {
    min-width: 16px;
    height: 16px;
    border-radius: 4px;
    background: var(--bg-badge);
    color: var(--text-strong);
    font-size: 10px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .port {
    font-weight: 500;
  }

  .arrow {
    color: var(--text-muted);
  }

  .dest {
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
