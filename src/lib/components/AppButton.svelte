<script lang="ts">
  export let type: "primary" | "warning" | "danger" | "plain" = "primary";
  export let onclick: () => void = () => {};
  export let disabled: boolean = false;
</script>

<button on:click={onclick} class="app-button {type}" disabled={disabled}>
  <slot />
</button>

<style lang="scss">
  .app-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: var(--radius);
    border: 1px solid transparent;
    background-color: var(--accent);
    color: white;
    font-size: 0.8rem;
    font-weight: 600;
    transition: background-color 0.15s ease, transform 0.15s ease, box-shadow 0.15s ease;

    &:hover {
        background-color: var(--accent-hover);
    }

    &:active {
        transform: translateY(1px);
    }

    &.primary {
      background-color: var(--accent);
      color: white;

      &:hover {
        background-color: var(--accent-hover);
      }
    }

    &.warning {
      background-color: var(--warning);
      color: white;

      &:hover {
        background-color: color-mix(in srgb, var(--warning) 80%, #000 20%);
      }
    }

    &.danger {
      background-color: var(--danger);
      color: white;

      &:hover {
        background-color: color-mix(in srgb, var(--danger) 80%, #000 20%);
      }
    }

    &.plain {
      background-color: var(--bg-card);
      color: var(--text-primary);
      border: 1px solid var(--border);

      &:hover {
        background-color: color-mix(in srgb, var(--bg-card) 80%, #000 20%);
      }
    }
  }
</style>
