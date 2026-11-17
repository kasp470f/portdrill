<script lang="ts">
  import { openNewRule } from "../stores/ui";
  import Icon from "./Icon.svelte";
</script>

<div class="empty-state">
  <div class="intro">
    <h2>Set up your first tunnel</h2>
    <p>
      A rule is one SSH connection. It can carry several forwards, and you turn the whole rule on or off
      with one switch.
    </p>
  </div>

  <div class="options">
    <button type="button" class="option primary" onclick={() => openNewRule("fill")}>
      <span class="icon accent"><Icon name="plus" size={16} strokeWidth={2.5} /></span>
      <span class="text">
        <span class="title">Fill in a new rule</span>
        <span class="sub">Server, then the ports you want to forward</span>
      </span>
    </button>
    <button type="button" class="option" onclick={() => openNewRule("paste")}>
      <span class="icon"><Icon name="terminal" size={16} /></span>
      <span class="text">
        <span class="title">Paste an SSH command</span>
        <span class="sub mono">ssh -N -L 5432:db.internal:5432 user@bastion</span>
      </span>
    </button>
  </div>

  <div class="kinds">
    <h3>Three kinds of forward</h3>
    <dl>
      <dt><span class="badge">L</span>Local</dt>
      <dd>Reach something behind the server, like a database, from a port on this computer.</dd>
      <dt><span class="badge">R</span>Remote</dt>
      <dd>Let the server reach something on this computer, like a local dev server for webhooks.</dd>
      <dt><span class="badge">D</span>SOCKS</dt>
      <dd>A proxy your browser or other apps can use to connect as if they were on the server.</dd>
    </dl>
  </div>
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    gap: 26px;
    padding: 20px 28px 24px;
  }

  .intro {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h2 {
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .intro p {
    font-size: 14px;
    line-height: 1.55;
    color: var(--text-secondary);
  }

  .options {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .option {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    border-radius: 12px;
    border: 1px solid var(--border-strong);
    background: var(--bg-card);
    text-align: left;
  }

  .option:hover {
    border-color: #4a4a52;
    background: var(--bg-hover);
  }

  .option.primary {
    border-color: #33407a;
    background: #1a1d2b;
  }

  .icon {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: 9px;
    background: #26262b;
    color: var(--text-strong);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon.accent {
    background: var(--accent);
    color: var(--accent-text);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .title {
    font-size: 14px;
    font-weight: 600;
  }

  .sub {
    font-size: 13px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub.mono {
    font-size: 12px;
  }

  .kinds {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: #161618;
  }

  h3 {
    font-size: 13px;
    font-weight: 600;
  }

  dl {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr);
    gap: 10px;
    font-size: 13px;
  }

  /* The label row is exactly one text line tall, so the badge and the word
     centre on the first line of the description beside them. */
  dt {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 1.5em;
    font-weight: 600;
    line-height: 1;
  }

  dd {
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .badge {
    width: 18px;
    height: 18px;
    border-radius: 4px;
    background: var(--bg-badge);
    color: var(--text-strong);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
</style>
