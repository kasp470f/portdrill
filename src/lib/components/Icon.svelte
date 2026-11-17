<script lang="ts" module>
  export type IconName =
    | "plus"
    | "search"
    | "more"
    | "eye"
    | "eyeOff"
    | "chevronRight"
    | "chevronDown"
    | "close"
    | "trash"
    | "terminal"
    | "alert"
    | "shieldAlert"
    | "file"
    | "arrowRight"
    | "grip";
</script>

<script lang="ts">
  interface Props {
    name: IconName;
    size?: number;
    strokeWidth?: number;
  }

  let { name, size = 15, strokeWidth = 2 }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={name === "more" || name === "grip" ? "currentColor" : "none"}
  stroke={name === "more" || name === "grip" ? "none" : "currentColor"}
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  style="display: block; flex-shrink: 0;"
>
  {#if name === "plus"}
    <path d="M12 5v14M5 12h14" />
  {:else if name === "search"}
    <circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" />
  {:else if name === "more"}
    <circle cx="5" cy="12" r="1.8" /><circle cx="12" cy="12" r="1.8" /><circle cx="19" cy="12" r="1.8" />
  {:else if name === "eye"}
    <path d="M2 12c1-2.5 5-7 10-7s9 4.5 10 7c-1 2.5-5 7-10 7S3 14.5 2 12z" /><circle cx="12" cy="12" r="3" />
  {:else if name === "eyeOff"}
    <path d="M3 3l18 18" /><path
      d="M10.6 5.1A10 10 0 0 1 12 5c5 0 9 4.5 10 7a13 13 0 0 1-3.2 4.3M6.6 6.6C4.4 8 2.8 10.2 2 12c1 2.5 5 7 10 7 1.7 0 3.3-.5 4.6-1.3"
    />
  {:else if name === "chevronRight"}
    <path d="M9 6l6 6-6 6" />
  {:else if name === "chevronDown"}
    <path d="M6 9l6 6 6-6" />
  {:else if name === "close"}
    <path d="M6 6l12 12M18 6L6 18" />
  {:else if name === "trash"}
    <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" />
  {:else if name === "terminal"}
    <path d="M4 17l6-5-6-5" /><path d="M12 19h8" />
  {:else if name === "alert"}
    <circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16h.01" />
  {:else if name === "shieldAlert"}
    <path d="M12 3l8 3v6c0 4.5-3.4 8.3-8 9-4.6-.7-8-4.5-8-9V6l8-3z" /><path d="M12 8v5M12 16h.01" />
  {:else if name === "file"}
    <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" /><path d="M14 3v6h6" />
  {:else if name === "arrowRight"}
    <path d="M4 12h15M13 6l6 6-6 6" />
  {:else if name === "grip"}
    <circle cx="9" cy="6" r="1.6" /><circle cx="15" cy="6" r="1.6" />
    <circle cx="9" cy="12" r="1.6" /><circle cx="15" cy="12" r="1.6" />
    <circle cx="9" cy="18" r="1.6" /><circle cx="15" cy="18" r="1.6" />
  {/if}
</svg>
