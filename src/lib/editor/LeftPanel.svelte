<script lang="ts">
  import LayerTree from "./LayerTree.svelte";
  import GlobalsPanel from "./GlobalsPanel.svelte";
  import ShortcutsPanel from "./ShortcutsPanel.svelte";
  import BackgroundPanel from "./BackgroundPanel.svelte";
  import SettingsPanel from "./SettingsPanel.svelte";

  let activeTab = $state("layers");

  const tabs: { id: string; label: string }[] = [
    { id: "layers", label: "Layers" },
    { id: "globals", label: "Globals" },
    { id: "shortcuts", label: "Shortcuts" },
    { id: "background", label: "BG" },
    { id: "settings", label: "\u2699" },
  ];
</script>

<div class="left-panel">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="tab-bar">
    {#each tabs as tab}
      <span
        class="tab"
        style="border-bottom-color: {activeTab === tab.id ? 'var(--accent)' : 'transparent'}; color: {activeTab === tab.id ? 'var(--accent)' : 'var(--text-muted)'};"
        onclick={() => { activeTab = tab.id; }}
      >{tab.label}</span>
    {/each}
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
    {:else if activeTab === "settings"}
      <SettingsPanel />
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
  .tab-bar {
    display: flex;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
  }
  .tab {
    flex: 1;
    padding: 8px 4px 6px;
    text-align: center;
    cursor: pointer;
    font-size: 11px;
    letter-spacing: 0.3px;
    text-transform: uppercase;
    border-bottom: 2px solid transparent;
    transition: color 0.15s;
  }
  .tab:hover {
    color: var(--text-primary) !important;
  }
  .tab-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
