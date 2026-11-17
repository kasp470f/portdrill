<script lang="ts">
  /** Group picker: existing groups, "No group", or type a new one. */
  interface Props {
    groups: string[];
    value: string;
    label?: string;
    hint?: string;
  }

  let { groups, value = $bindable(), label = "Group", hint }: Props = $props();

  const NEW = "\u0000new";
  const id = `group-${Math.random().toString(36).slice(2)}`;

  // Start in "new" mode when the value isn't one of the known groups.
  let creating = $state(false);
  let draft = $state("");

  $effect.pre(() => {
    if (value && !groups.includes(value) && !creating) {
      creating = true;
      draft = value;
    }
  });

  let selected = $derived(creating ? NEW : value);

  function onSelect(e: Event) {
    const v = (e.target as HTMLSelectElement).value;
    if (v === NEW) {
      creating = true;
      draft = "";
      value = "";
    } else {
      creating = false;
      value = v;
    }
  }
</script>

<div class="group-select">
  <label class="field-label" for={id}>
    <span>{label}{#if hint} <span class="field-hint">· {hint}</span>{/if}</span>
  </label>
  <div class="row">
    <select {id} class="field-input" value={selected} onchange={onSelect}>
      {#each groups as g (g)}
        <option value={g}>{g}</option>
      {/each}
      <option value="">No group</option>
      <option value={NEW}>New group…</option>
    </select>
    {#if creating}
      <input
        class="field-input"
        placeholder="Group name"
        aria-label="New group name"
        bind:value={draft}
        oninput={() => (value = draft.trim())}
      />
    {/if}
  </div>
</div>

<style>
  .group-select {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  select {
    width: 200px;
    flex-shrink: 0;
  }
</style>
