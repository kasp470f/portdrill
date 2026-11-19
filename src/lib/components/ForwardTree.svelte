<script lang="ts">
  import type { Forward } from "../types";
  import { forwardTypeLabel, sortedForwards } from "../types";

  /**
   * A rule's forwards hanging off the status dot on thin tree lines.
   *
   * Open: one chip per forward (badge, port, destination). Display only.
   * Hidden: a single branch carrying just the L / R / D badges, each in its
   * own small chip, wrapping when there are many. Clicking them calls
   * `onShow`.
   *
   * `reach` is how far the first line climbs back up towards the dot.
   */
  interface Props {
    forwards: Forward[];
    hidden?: boolean;
    onShow?: () => void;
    reach?: number;
  }

  let { forwards, hidden = false, onShow, reach = 11 }: Props = $props();

  const LEAF = 30;

  let sorted = $derived(sortedForwards(forwards));

  let leaves = $derived(
    sorted.map((fwd, i, all) => {
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

  let showLabel = $derived(
    forwards.length === 1 ? "Show 1 hidden forward" : `Show ${forwards.length} hidden forwards`,
  );
</script>

<div class="tree">
  {#if hidden}
    <div class="leaf wrap">
      <span class="elbow" style="top: {-reach}px; height: {LEAF / 2 + 1 + reach}px" aria-hidden="true"></span>
      <button type="button" class="badges" aria-label={showLabel} title={showLabel} onclick={onShow}>
        {#each sorted as fwd, i (i)}
          <span class="badge-chip" aria-hidden="true">
            <span class="type-badge {fwd.forwardType}">{forwardTypeLabel(fwd.forwardType)}</span>
          </span>
        {/each}
      </button>
    </div>
  {:else}
    {#each leaves as leaf, i (i)}
      <div class="leaf">
        {#if leaf.last}
          <span class="elbow" style="top: {leaf.top}px; height: {leaf.elbowHeight}px" aria-hidden="true"></span>
        {:else}
          <span class="stem" style="top: {leaf.top}px; height: {leaf.lineHeight}px" aria-hidden="true"></span>
          <span class="tick" aria-hidden="true"></span>
        {/if}
        <span class="chip">
          <span class="type-badge {leaf.fwd.forwardType}">{leaf.badge}</span>
          <span class="port">{leaf.port ?? "?"}</span>
          <span class="arrow">{leaf.arrow}</span>
          <span class="dest">{leaf.dest}</span>
        </span>
      </div>
    {/each}
  {/if}
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

  /* The hidden branch grows when its badges wrap; the elbow stays on the
     first row (3px padding + half of a 24px chip = the 15px midline). */
  .leaf.wrap {
    height: auto;
    min-height: 30px;
    padding-top: 3px;
    padding-bottom: 3px;
    align-items: flex-start;
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

  .type-badge {
    min-width: 16px;
    height: 16px;
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

  .badges {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    min-width: 0;
    border-radius: 6px;
  }

  /* 4px left padding matches the open chip's, so the first badge sits on the
     exact same spot whether the forwards are open or hidden. */
  .badge-chip {
    display: flex;
    align-items: center;
    height: 24px;
    padding: 0 4px;
    border-radius: 6px;
    border: 1px solid var(--border-strong);
    background: var(--bg-chip);
  }
</style>
