<script lang="ts">
  import { getSettings, removeTheme } from "../stores/settings.svelte";

  let confirmDeletePath = $state<string | null>(null);

  async function openTheme(path: string) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const { setProject } = await import("../stores/project.svelte");
      const project = await invoke<any>("load_project", { path });
      setProject(project);
    } catch (e) {
      console.error("Failed to open theme:", e);
    }
  }

  function confirmDelete(path: string) {
    confirmDeletePath = path;
  }

  function doDelete() {
    if (confirmDeletePath) {
      removeTheme(confirmDeletePath);
      confirmDeletePath = null;
    }
  }

  const themes = $derived(getSettings().savedThemes ?? []);
</script>

<div class="themes-panel">
  {#if themes.length === 0}
    <div class="empty">No saved themes yet.<br />Save a project to add it here.</div>
  {:else}
    <div class="theme-list">
      {#each themes as theme}
        <div class="theme-item">
          <span class="theme-name" onclick={() => openTheme(theme.path)} title={theme.path}>{theme.name}</span>
          <span class="del-btn" onclick={() => confirmDelete(theme.path)} title="Remove from list">×</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if confirmDeletePath !== null}
  <div class="overlay">
    <div class="dialog">
      <div class="dialog-title">Remove Theme</div>
      <div class="dialog-body">
        This removes <strong>{themes.find(t => t.path === confirmDeletePath)?.name ?? ''}</strong> from your themes list.<br /><br />
        <em>The file on disk will not be deleted.</em>
      </div>
      <div class="dialog-btns">
        <span class="btn-cancel" onclick={() => confirmDeletePath = null}>Cancel</span>
        <span class="btn-remove" onclick={doDelete}>Remove from List</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .themes-panel {
    padding: 8px 10px;
    height: 100%;
    overflow-y: auto;
  }
  .empty {
    font-size: 11px;
    color: var(--text-muted);
    text-align: center;
    margin-top: 24px;
    line-height: 1.6;
  }
  .theme-list {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .theme-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 5px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    gap: 6px;
  }
  .theme-item:hover {
    border-color: var(--accent);
    background: var(--accent-dim);
  }
  .theme-name {
    font-size: 12px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    cursor: pointer;
  }
  .del-btn {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    border-radius: 3px;
    background: transparent;
    color: var(--text-muted);
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    line-height: 1;
  }
  .del-btn:hover {
    background: #c0392b;
    color: #fff;
  }
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }
  .dialog {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 20px 24px;
    width: 340px;
    max-width: 90vw;
  }
  .dialog-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 12px;
  }
  .dialog-body {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.6;
    margin-bottom: 16px;
  }
  .dialog-body strong {
    color: var(--text-primary);
  }
  .dialog-body em {
    color: var(--text-muted);
    font-style: normal;
    font-size: 11px;
  }
  .dialog-btns {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn-cancel {
    padding: 5px 14px;
    font-size: 12px;
    border-radius: 4px;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    cursor: pointer;
    background: transparent;
  }
  .btn-cancel:hover {
    border-color: var(--text-muted);
    color: var(--text-primary);
  }
  .btn-remove {
    padding: 5px 14px;
    font-size: 12px;
    font-weight: 600;
    border-radius: 4px;
    background: #c0392b;
    color: #fff;
    cursor: pointer;
  }
  .btn-remove:hover {
    background: #e74c3c;
  }
</style>
