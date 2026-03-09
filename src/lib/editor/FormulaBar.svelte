<script lang="ts">
  import { getSelectedLayer } from "../stores/project.svelte";

  let activeField = $state("text");

  function getFormulaValue(): string {
    const layer = getSelectedLayer();
    if (!layer) return "";
    const val = layer.properties[activeField as keyof typeof layer.properties];
    return val != null ? String(val) : "";
  }
</script>

<div class="formula-bar">
  {#if getSelectedLayer()}
    {@const layer = getSelectedLayer()!}
    <span class="formula-label">{layer.name}</span>
    <span class="formula-separator">/</span>
    <select class="field-select" bind:value={activeField}>
      <option value="x">x</option>
      <option value="y">y</option>
      <option value="width">width</option>
      <option value="height">height</option>
      <option value="rotation">rotation</option>
      <option value="opacity">opacity</option>
      {#if layer.type === "text"}
        <option value="text">text</option>
        <option value="fontSize">fontSize</option>
        <option value="color">color</option>
      {/if}
      {#if layer.type === "progress"}
        <option value="value">value</option>
        <option value="min">min</option>
        <option value="max">max</option>
      {/if}
    </select>
    <span class="formula-eq">=</span>
    <input
      class="formula-input"
      type="text"
      value={getFormulaValue()}
      placeholder="Formula or value..."
      readonly
    />
  {:else}
    <span class="formula-placeholder">Select a layer to view formulas</span>
  {/if}
</div>

<style>
  .formula-bar {
    height: var(--formula-bar-height);
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    padding: 0 12px;
    gap: 8px;
    font-size: 12px;
  }
  .formula-label {
    color: var(--accent);
    font-weight: 600;
    white-space: nowrap;
  }
  .formula-separator {
    color: var(--text-muted);
  }
  .field-select {
    padding: 2px 4px;
    font-size: 11px;
    min-width: 80px;
  }
  .formula-eq {
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .formula-input {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 3px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    color: var(--text-primary);
    border-radius: 3px;
  }
  .formula-placeholder {
    color: var(--text-muted);
    font-style: italic;
  }
</style>
