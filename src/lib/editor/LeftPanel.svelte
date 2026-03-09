<script lang="ts">
  import LayerTree from "./LayerTree.svelte";
  import GlobalsPanel from "./GlobalsPanel.svelte";
  import ShortcutsPanel from "./ShortcutsPanel.svelte";
  import BackgroundPanel from "./BackgroundPanel.svelte";

  let activeTab = $state("layers");
</script>

<div class="left-panel">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div style="display: flex; border-bottom: 1px solid var(--border); flex-shrink: 0; user-select: none;">
    <span
      style="flex: 1; padding: 6px 2px 4px; text-align: center; cursor: pointer; font-size: 10px; letter-spacing: 0.3px; text-transform: uppercase; border-bottom: 2px solid {activeTab === 'layers' ? 'var(--accent)' : 'transparent'}; color: {activeTab === 'layers' ? 'var(--accent)' : 'var(--text-muted)'};"
      onclick={() => { activeTab = "layers"; }}
    >Layers</span>
    <span
      style="flex: 1; padding: 6px 2px 4px; text-align: center; cursor: pointer; font-size: 10px; letter-spacing: 0.3px; text-transform: uppercase; border-bottom: 2px solid {activeTab === 'globals' ? 'var(--accent)' : 'transparent'}; color: {activeTab === 'globals' ? 'var(--accent)' : 'var(--text-muted)'};"
      onclick={() => { activeTab = "globals"; }}
    >Globals</span>
    <span
      style="flex: 1; padding: 6px 2px 4px; text-align: center; cursor: pointer; font-size: 10px; letter-spacing: 0.3px; text-transform: uppercase; border-bottom: 2px solid {activeTab === 'shortcuts' ? 'var(--accent)' : 'transparent'}; color: {activeTab === 'shortcuts' ? 'var(--accent)' : 'var(--text-muted)'};"
      onclick={() => { activeTab = "shortcuts"; }}
    >Shortcuts</span>
    <span
      style="flex: 1; padding: 6px 2px 4px; text-align: center; cursor: pointer; font-size: 10px; letter-spacing: 0.3px; text-transform: uppercase; border-bottom: 2px solid {activeTab === 'background' ? 'var(--accent)' : 'transparent'}; color: {activeTab === 'background' ? 'var(--accent)' : 'var(--text-muted)'};"
      onclick={() => { activeTab = "background"; }}
    >BG</span>
  </div>

  <div class="tab-content">
    {#if activeTab === "layers"}
      <LayerTree />
    {:else if activeTab === "globals"}
      <GlobalsPanel />
    {:else if activeTab === "shortcuts"}
      <ShortcutsPanel />
    {:else if activeTab === "background"}
      <BackgroundPanel />
    {/if}
  </div>
</div>

<style>
  .left-panel {
    width: var(--panel-width);
    min-width: var(--panel-width);
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .tab-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
