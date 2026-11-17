<script lang="ts">
  /**
   * On/off switch. "connecting" shows the yellow track and is disabled, so a
   * second click can't fire while ssh is still starting.
   */
  interface Props {
    state: "on" | "off" | "connecting";
    label: string;
    onToggle: () => void;
    disabled?: boolean;
  }

  let { state, label, onToggle, disabled = false }: Props = $props();
</script>

<button
  type="button"
  class="toggle {state}"
  role="switch"
  aria-checked={state !== "off"}
  aria-busy={state === "connecting"}
  aria-label={label}
  title={label}
  disabled={disabled || state === "connecting"}
  onclick={(e) => {
    e.stopPropagation();
    onToggle();
  }}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 40px;
    height: 24px;
    flex-shrink: 0;
    border-radius: 12px;
    border: 1px solid var(--border-button);
    background: #26262b;
    padding: 0;
    transition: background 0.15s, border-color 0.15s;
  }

  .toggle.on {
    background: var(--success-track);
    border-color: var(--success-track-border);
  }

  .toggle.connecting {
    background: var(--warning-track);
    border-color: var(--warning-track-border);
  }

  .toggle:disabled {
    cursor: progress;
  }

  .toggle.off:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-secondary);
    transition: left 0.15s, background 0.15s;
  }

  .on .knob,
  .connecting .knob {
    left: 20px;
    background: #ffffff;
  }
</style>
