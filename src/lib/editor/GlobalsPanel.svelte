<script lang="ts">
  import {
    getProject,
    addGlobal as storeAddGlobal,
    removeGlobal as storeRemoveGlobal,
    updateGlobal as storeUpdateGlobal,
  } from "../stores/project.svelte";
  import type { GlobalVariable, GlobalVarType } from "../types/project";
  import FormulaHelper from "./FormulaHelper.svelte";

  let editingGlobal = $state<string | null>(null);
  let formulaHelperTarget = $state<string | null>(null);

  function handleUpdateGlobal(name: string, field: string, value: any) {
    storeUpdateGlobal(name, field, value);
  }

  function handleAddGlobal() {
    const name = storeAddGlobal("text");
    editingGlobal = name;
  }

  function handleRemoveGlobal(name: string) {
    storeRemoveGlobal(name);
    if (editingGlobal === name) editingGlobal = null;
  }

  function typeIcon(type: GlobalVarType): string {
    switch (type) {
      case "text": return "Aa";
      case "number": return "#";
      case "color": return "\u25CF";
      case "switch": return "\u21C4";
      case "list": return "\u2630";
      default: return "?";
    }
  }

  function displayValue(g: GlobalVariable): string {
    if (g.type === "switch") return g.value ? "ON" : "OFF";
    if (g.type === "color") return String(g.value);
    return String(g.value).substring(0, 30) || "(empty)";
  }

  function handleFormulaInsert(formula: string) {
    if (!formulaHelperTarget) return;
    const g = getProject().globals.find(v => v.name === formulaHelperTarget);
    if (!g) return;
    const current = String(g.value ?? "");
    handleUpdateGlobal(g.name, "value", current + formula);
    formulaHelperTarget = null;
  }
</script>

<div class="globals-panel">
  <div class="panel-header">
    <span>Globals</span>
    <button class="add-btn" title="Add global variable" onclick={handleAddGlobal}>+</button>
  </div>
  <div class="globals-list">
    {#each getProject().globals as g, i (i)}
      <div
        style="cursor: pointer; border-left: 2px solid {editingGlobal === g.name ? 'var(--accent)' : 'transparent'}; background: {editingGlobal === g.name ? 'var(--accent-dim)' : 'transparent'}; transition: background 0.1s; user-select: none;"
        onclick={() => editingGlobal = editingGlobal === g.name ? null : g.name}
        role="button"
        tabindex="0"
        onkeydown={(e) => { if (e.key === 'Enter') editingGlobal = editingGlobal === g.name ? null : g.name; }}
      >
        <div class="global-row">
          <span style="font-size: {g.type === 'color' ? '16px' : '12px'}; font-weight: 700; width: 20px; text-align: center; color: {g.type === 'color' ? String(g.value) : 'var(--text-muted)'};"
          >{typeIcon(g.type)}</span>
          <span class="global-name">{g.name}</span>
          <span class="global-value-preview">{displayValue(g)}</span>
          <button
            class="delete-btn"
            title="Delete"
            onclick={(e) => { e.stopPropagation(); handleRemoveGlobal(g.name); }}
          >&times;</button>
        </div>
        {#if editingGlobal === g.name}
          <div class="global-edit" onclick={(e) => e.stopPropagation()}>
            <label>Name</label>
            <input type="text" value={g.name}
              oninput={(e) => {
                const newName = (e.target as HTMLInputElement).value;
                handleUpdateGlobal(g.name, 'name', newName);
                editingGlobal = newName;
              }} />
            <label>Type</label>
            <select value={g.type}
              onchange={(e) => handleUpdateGlobal(g.name, 'type', (e.target as HTMLSelectElement).value)}>
              <option value="text">Text</option>
              <option value="number">Number</option>
              <option value="color">Color</option>
              <option value="switch">Switch</option>
              <option value="list">List</option>
            </select>
            {#if g.type === "color"}
              <label>Value</label>
              <input type="color" value={String(g.value) || "#ffffff"}
                oninput={(e) => handleUpdateGlobal(g.name, 'value', (e.target as HTMLInputElement).value)} />
            {:else if g.type === "switch"}
              <label>Value</label>
              <button
                style="padding: 4px 12px; border-radius: 4px; font-size: 12px; font-weight: 600; cursor: pointer; transition: background 0.1s, color 0.1s; background: {g.value ? 'var(--accent-dim)' : 'var(--bg-input)'}; color: {g.value ? 'var(--accent)' : 'var(--text-muted)'};"
                onclick={() => handleUpdateGlobal(g.name, 'value', !g.value)}>
                {g.value ? "ON" : "OFF"}
              </button>
            {:else if g.type === "number"}
              <label>Value</label>
              <div class="input-with-fx">
                <input type="number" value={g.value}
                  oninput={(e) => handleUpdateGlobal(g.name, 'value', Number((e.target as HTMLInputElement).value))} />
                <button class="fx-btn" title="Formula Helper" onclick={() => formulaHelperTarget = g.name}>fx</button>
              </div>
            {:else}
              <label>Value</label>
              <div class="input-with-fx">
                <input type="text" value={g.value}
                  oninput={(e) => handleUpdateGlobal(g.name, 'value', (e.target as HTMLInputElement).value)} />
                <button class="fx-btn" title="Formula Helper" onclick={() => formulaHelperTarget = g.name}>fx</button>
              </div>
            {/if}
            {#if g.type === "list" && g.options}
              <label>Options</label>
              <input type="text" value={g.options.join(", ")} placeholder="option1, option2, ..."
                oninput={(e) => handleUpdateGlobal(g.name, 'options',
                  (e.target as HTMLInputElement).value.split(',').map(s => s.trim()).filter(Boolean))} />
            {/if}
          </div>
        {/if}
      </div>
    {/each}
    {#if getProject().globals.length === 0}
      <div class="empty-state">No global variables. Import a preset or add one.</div>
    {/if}
  </div>
</div>

<FormulaHelper
  open={formulaHelperTarget !== null}
  onInsert={handleFormulaInsert}
  onClose={() => formulaHelperTarget = null}
/>

<style>
  .globals-panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    font-weight: 600;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }
  .add-btn {
    font-size: 16px;
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .add-btn:hover {
    background: var(--bg-secondary);
    color: var(--accent);
  }
  .globals-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .global-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
  }
  .global-name {
    flex: 1;
    font-size: 12px;
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .global-value-preview {
    font-size: 11px;
    color: var(--text-muted);
    max-width: 60px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .delete-btn {
    font-size: 16px;
    width: 20px;
    padding: 0;
    color: var(--text-muted);
    opacity: 0.6;
    transition: opacity 0.1s, color 0.1s;
  }
  .delete-btn:hover {
    opacity: 1 !important;
    color: var(--error);
  }
  .global-edit {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
  }
  .global-edit label {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 2px;
  }
  .global-edit input,
  .global-edit select {
    width: 100%;
    font-size: 12px;
    padding: 3px 6px;
  }
  .global-edit input[type="color"] {
    height: 28px;
    padding: 2px;
    cursor: pointer;
  }
  .input-with-fx {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .input-with-fx input {
    flex: 1;
  }
  .fx-btn {
    font-size: 10px;
    font-weight: 700;
    font-family: var(--font-mono);
    padding: 3px 6px;
    border-radius: 3px;
    background: var(--bg-input);
    color: var(--accent);
    border: 1px solid var(--border);
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .fx-btn:hover {
    background: var(--accent-dim);
    border-color: var(--accent);
  }
  .empty-state {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.6;
  }
</style>
