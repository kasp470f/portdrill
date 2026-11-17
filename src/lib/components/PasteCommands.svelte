<script lang="ts">
  import type { Rule } from "../types";
  import { forwardTypeLabel, sortedForwards } from "../types";
  import { parseSshCommand } from "../sshParser";
  import GroupSelect from "./GroupSelect.svelte";

  interface Props {
    groups: string[];
    group: string;
    /** Rules the current text would create (after the split option). */
    parsed: Rule[];
    /** Non-empty when exactly one rule would be created. */
    onReview: (rule: Rule) => void;
  }

  let { groups, group = $bindable(), parsed = $bindable(), onReview }: Props = $props();

  let text = $state("");
  let split = $state(false);

  type Line = { line: number; src: string; rule?: Rule; error?: string };

  function autoName(rule: Rule): string {
    const short = rule.sshHost.split(".")[0] || rule.sshHost;
    const ports = rule.forwards
      .map((f) => (f.forwardType === "remote" ? f.remotePort : f.localPort))
      .filter((p) => p !== undefined)
      .map((p) => `:${p}`)
      .join(" ");
    return `${short} ${ports}`.trim();
  }

  let lines = $derived.by((): Line[] => {
    const out: Line[] = [];
    // Join "\" line continuations so a multi-line command counts as one.
    text
      .replace(/\\\r?\n/g, " ")
      .split(/\r?\n/)
      .forEach((raw, i) => {
        const src = raw.trim();
        if (!src || src.startsWith("#")) return;
        const result = parseSshCommand(src);
        if (result.errors.length > 0) {
          out.push({ line: i + 1, src, error: result.errors[0] });
        } else {
          out.push({ line: i + 1, src, rule: result.rule });
        }
      });
    return out;
  });

  let skipped = $derived(lines.filter((l) => l.error));

  let unsplitCount = $derived(lines.filter((l) => l.rule).length);
  let splitCount = $derived(lines.reduce((n, l) => n + (l.rule ? l.rule.forwards.length : 0), 0));

  $effect(() => {
    const result: Rule[] = [];
    for (const l of lines) {
      if (!l.rule) continue;
      const parts = split ? l.rule.forwards.map((f) => [f]) : [l.rule.forwards];
      for (const forwards of parts) {
        const rule: Rule = { ...l.rule, forwards, group };
        rule.name = autoName(rule);
        result.push(rule);
      }
    }
    parsed = result;
  });
</script>

<label class="field-label">
  <span>SSH command <span class="field-hint">· paste several, one per line, to create several rules</span></span>
  <textarea
    class="field-input mono paste"
    spellcheck="false"
    bind:value={text}
    placeholder="ssh -N -p 2222 -L 6443:10.0.4.12:6443 ops@staging-gw.example.com"
  ></textarea>
</label>

<GroupSelect {groups} bind:value={group} label="Group" hint="all new rules go here" />

{#if lines.length > 0}
  <div class="preview">
    <div class="preview-head">
      <span class="count">
        {parsed.length === 0
          ? "Nothing to create yet"
          : parsed.length === 1
            ? "This will create 1 rule"
            : `This will create ${parsed.length} rules`}
      </span>
      <span class="hint">Names can be changed after</span>
    </div>
    <div class="preview-list">
      {#each parsed as rule, i (i)}
        <div class="item">
          <div class="item-title">
            <span class="name">{rule.name}</span>
            <span class="target mono"
              >{rule.sshUser ? rule.sshUser + "@" : ""}{rule.sshHost}{rule.sshPort !== 22 ? ":" + rule.sshPort : ""}</span
            >
          </div>
          <div class="chips">
            {#each sortedForwards(rule.forwards) as f, j (j)}
              <span class="chip mono">
                <span class="badge">{forwardTypeLabel(f.forwardType)}</span>
                <span class="port">{f.forwardType === "remote" ? f.remotePort : f.localPort}</span>
                {#if f.forwardType === "dynamic"}
                  <span class="dest">SOCKS5</span>
                {:else}
                  <span class="arrow">→</span>
                  <span class="dest">{f.destinationHost}:{f.destinationPort}</span>
                {/if}
              </span>
            {/each}
          </div>
        </div>
      {/each}
      {#each skipped as s (s.line)}
        <div class="item skipped">
          <span class="skip-title">Line {s.line} skipped: {s.error}</span>
          <span class="skip-src mono">{s.src}</span>
        </div>
      {/each}
    </div>
  </div>

  {#if unsplitCount > 0 && splitCount > unsplitCount}
    <label class="check">
      <input type="checkbox" bind:checked={split} />
      Make a separate rule for each forward
      <span class="field-hint">({splitCount} rules instead of {unsplitCount})</span>
    </label>
  {/if}

  {#if parsed.length === 1}
    <button type="button" class="review" onclick={() => onReview(parsed[0])}>Review in the form instead</button>
  {/if}
{:else}
  <p class="help">
    Understands <span class="mono">-L -R -D -p -i</span> and <span class="mono">user@host</span>. PortDrill only reads
    the text; nothing is run.
  </p>
{/if}

<style>
  .paste {
    height: 104px;
    padding: 10px 12px;
    line-height: 1.6;
    resize: none;
    white-space: pre;
    overflow-x: auto;
  }

  .preview {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .preview-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .count {
    font-size: 13px;
    font-weight: 600;
  }

  .hint {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .preview-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-card);
    background: #161618;
    overflow: hidden;
  }

  .item {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 11px 14px;
    border-top: 1px solid var(--border);
  }

  .item:first-child {
    border-top: none;
  }

  .item-title {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .name {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
  }

  .target {
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chips {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 8px 0 4px;
    border-radius: 6px;
    border: 1px solid var(--border-strong);
    background: var(--bg-chip);
    font-size: 11px;
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
  }

  .skipped {
    background: #1d1415;
  }

  .skip-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--danger-text);
  }

  .skip-src {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-strong);
    cursor: pointer;
  }

  .check input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }

  .review {
    align-self: flex-start;
    font-size: 12px;
    color: var(--link);
  }

  .review:hover {
    text-decoration: underline;
  }

  .help {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
  }
</style>
