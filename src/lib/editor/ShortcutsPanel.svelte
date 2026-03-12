<script lang="ts">
  import { getProject, addShortcut, updateShortcut, removeShortcut } from "../stores/project.svelte";

  let editingId = $state<string | null>(null);
  let newKeys = $state("");
  let newAction = $state("music:play-pause");
  let newLabel = $state("");
  let isAdding = $state(false);

  const project = $derived(getProject());

  const actionTypes = [
    { prefix: "music:", label: "Music", placeholder: "play-pause, next, previous, stop" },
    { prefix: "app:", label: "App", placeholder: "firefox, kitty, etc." },
    { prefix: "overlay:", label: "Overlay", placeholder: "layer name" },
    { prefix: "url:", label: "URL", placeholder: "https://..." },
    { prefix: "set:", label: "Set Var", placeholder: "varName:value" },
    { prefix: "inc:", label: "Inc Var", placeholder: "varName:amount" },
  ];

  function getActionType(action: string) {
    return actionTypes.find(t => action.startsWith(t.prefix)) ?? actionTypes[0];
  }

  function getActionPrefix(action: string) {
    const t = getActionType(action);
    return t.prefix;
  }

  function getActionValue(action: string) {
    const prefix = getActionPrefix(action);
    return action.slice(prefix.length);
  }

  function handleAdd() {
    if (!newKeys.trim()) return;
    addShortcut(newKeys.trim(), newAction, newLabel.trim() || undefined);
    newKeys = "";
    newAction = "music:play-pause";
    newLabel = "";
    isAdding = false;
  }

  function handleActionTypeChange(id: string, newPrefix: string, currentAction: string) {
    const oldValue = getActionValue(currentAction);
    updateShortcut(id, "action", newPrefix + oldValue);
  }

  function handleActionValueChange(id: string, prefix: string, newValue: string) {
    updateShortcut(id, "action", prefix + newValue);
  }
</script>

<div class="shortcuts-panel">
  <div class="panel-header">
    <span>Shortcuts</span>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      class="add-btn"
      style="cursor: pointer; font-size: 16px; color: var(--accent); padding: 0 4px; user-select: none;"
      onclick={() => { isAdding = !isAdding; }}
    >+</span>
  </div>

  {#if isAdding}
    <div class="add-form">
      <div class="form-row">
        <label>Keys</label>
        <input
          type="text"
          class="field"
          placeholder="e.g. Super+1"
          bind:value={newKeys}
          onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter") handleAdd(); }}
        />
      </div>
      <div class="form-row">
        <label>Action</label>
        <select class="field" bind:value={newAction}>
          {#each actionTypes as at}
            <option value={at.prefix}>{at.label}</option>
          {/each}
        </select>
      </div>
      <div class="form-row">
        <label>Value</label>
        <input
          type="text"
          class="field"
          placeholder={getActionType(newAction).placeholder}
          value=""
          oninput={(e: Event) => {
            const val = (e.target as HTMLInputElement).value;
            newAction = getActionPrefix(newAction) + val;
          }}
        />
      </div>
      <div class="form-row">
        <label>Label</label>
        <input
          type="text"
          class="field"
          placeholder="Optional name"
          bind:value={newLabel}
        />
      </div>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="form-actions">
        <span
          style="padding: 4px 12px; background: var(--accent); color: #fff; border-radius: 4px; font-size: 11px; cursor: pointer; user-select: none;"
          onclick={handleAdd}
        >Add</span>
        <span
          style="padding: 4px 12px; background: var(--bg-tertiary); color: var(--text-secondary); border-radius: 4px; font-size: 11px; cursor: pointer; user-select: none;"
          onclick={() => { isAdding = false; }}
        >Cancel</span>
      </div>
    </div>
  {/if}

  <div class="shortcuts-list">
    {#if project.shortcuts.length === 0 && !isAdding}
      <div class="empty-state">
        <p>No shortcuts configured.</p>
        <p class="hint">Add keyboard shortcuts to trigger actions in wallpaper mode.</p>
      </div>
    {:else}
      {#each project.shortcuts as sc (sc.id)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="shortcut-item"
          onclick={() => { editingId = editingId === sc.id ? null : sc.id; }}
        >
          <div class="shortcut-summary">
            <span class="sc-keys">{sc.keys}</span>
            <span class="sc-label">{sc.label || getActionType(sc.action).label}</span>
            <span class="sc-value">{getActionValue(sc.action)}</span>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <span
              class="sc-delete"
              style="color: var(--text-muted); font-size: 13px; cursor: pointer; padding: 0 4px; user-select: none;"
              onclick={(e: MouseEvent) => { e.stopPropagation(); removeShortcut(sc.id); }}
            >&times;</span>
          </div>

          {#if editingId === sc.id}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="shortcut-edit" onclick={(e: MouseEvent) => e.stopPropagation()}>
              <div class="form-row">
                <label>Keys</label>
                <input
                  type="text"
                  class="field"
                  value={sc.keys}
                  oninput={(e: Event) => updateShortcut(sc.id, "keys", (e.target as HTMLInputElement).value)}
                />
              </div>
              <div class="form-row">
                <label>Type</label>
                <select
                  class="field"
                  value={getActionPrefix(sc.action)}
                  onchange={(e: Event) => handleActionTypeChange(sc.id, (e.target as HTMLSelectElement).value, sc.action)}
                >
                  {#each actionTypes as at}
                    <option value={at.prefix}>{at.label}</option>
                  {/each}
                </select>
              </div>
              <div class="form-row">
                <label>Value</label>
                <input
                  type="text"
                  class="field"
                  value={getActionValue(sc.action)}
                  placeholder={getActionType(sc.action).placeholder}
                  oninput={(e: Event) => handleActionValueChange(sc.id, getActionPrefix(sc.action), (e.target as HTMLInputElement).value)}
                />
              </div>
              <div class="form-row">
                <label>Label</label>
                <input
                  type="text"
                  class="field"
                  value={sc.label ?? ""}
                  placeholder="Optional"
                  oninput={(e: Event) => updateShortcut(sc.id, "label", (e.target as HTMLInputElement).value || undefined)}
                />
              </div>
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .shortcuts-panel {
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
  .add-btn:hover {
    opacity: 0.8;
  }
  .shortcuts-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .empty-state {
    padding: 32px 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.6;
  }
  .hint {
    font-size: 11px;
    opacity: 0.7;
    margin-top: 4px;
  }
  .add-form {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .form-row label {
    font-size: 11px;
    color: var(--text-muted);
    width: 40px;
    flex-shrink: 0;
    text-align: right;
  }
  .field {
    flex: 1;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--text-primary);
    padding: 3px 6px;
    font-size: 11px;
    outline: none;
  }
  .field:focus {
    border-color: var(--accent);
  }
  .form-actions {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
    padding-top: 4px;
  }
  .shortcut-item {
    border-bottom: 1px solid var(--border-subtle, rgba(255,255,255,0.05));
    cursor: pointer;
  }
  .shortcut-item:hover {
    background: var(--bg-hover, rgba(255,255,255,0.03));
  }
  .shortcut-summary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    font-size: 11px;
  }
  .sc-keys {
    background: var(--bg-tertiary);
    padding: 2px 6px;
    border-radius: 3px;
    font-family: monospace;
    font-size: 10px;
    color: var(--accent);
    white-space: nowrap;
  }
  .sc-label {
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .sc-value {
    color: var(--text-muted);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sc-delete:hover {
    color: var(--text-primary) !important;
  }
  .shortcut-edit {
    padding: 6px 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    background: var(--bg-tertiary);
  }
</style>
