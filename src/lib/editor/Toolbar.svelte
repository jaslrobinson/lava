<script lang="ts">
  import { addLayer, getProject, getProjectSnapshot, setProject, getIsDirty, getSelectedLayer, isContainerType, getInteractiveMode, setInteractiveMode, getSelectedLayerId, copySelectedLayer, getCopiedLayer, pasteLayer, undo, redo, canUndo, canRedo, insertWidget, ensureGlobal, getCurrentProjectPath, setCurrentProjectPath } from "../stores/project.svelte";
  import { updateSetting } from "../stores/settings.svelte";
  import type { LayerType, Project, GlobalVarType, Layer } from "../types/project";
  import { createDefaultProject } from "../types/project";
  import { setDebugOverlay, getDebugOverlay } from "../canvas/renderer";
  import WidgetsPanel from "./WidgetsPanel.svelte";
  import ThemesPanel from "./ThemesPanel.svelte";

  function handleCopy() {
    if (getSelectedLayerId()) copySelectedLayer();
  }

  function handlePaste() {
    const copied = getCopiedLayer();
    if (!copied) return;
    const name = prompt("Name for pasted layer:", copied.name + " copy");
    if (name !== null) pasteLayer(name);
  }

  async function handleNew() {
    if (getIsDirty()) {
      const confirmed = confirm("You have unsaved changes. Save before creating a new project?");
      if (confirmed) {
        await handleSave();
      }
    }
    setProject(createDefaultProject());
    setCurrentProjectPath("");
  }

  async function handleSave() {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await save({
        filters: [{ name: "LAVA Theme", extensions: ["lava", "klwp", "json"] }],
        defaultPath: `${getProject().name}.lava`,
      });
      if (path) {
        try {
          await invoke("save_project", { path, project: getProjectSnapshot() });
          const { addTheme } = await import("../stores/settings.svelte");
          addTheme(getProject().name, path);
          setCurrentProjectPath(path);
          updateSetting("lastProjectPath", path);
        } catch (e) {
          console.error("Save failed:", e);
          throw e;
        }
      }
    } catch (e) {
      console.error("Save failed:", e);
      alert(`Save failed: ${e}`);
    }
  }

  async function handleLoad() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await open({
        filters: [{ name: "LAVA Theme", extensions: ["lava", "klwp", "json"] }],
        multiple: false,
      });
      if (path) {
        const project = await invoke<Project>("load_project", { path });
        setProject(project);
        setCurrentProjectPath(path as string);
        updateSetting("lastProjectPath", path as string);
      }
    } catch (e) {
      console.error("Load failed:", e);
    }
  }

  let importStatus = $state("");

  async function handleImportKomp() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await open({
        filters: [{ name: "LAVA Rock / KLWP Komponent", extensions: ["rock", "komp"] }],
        multiple: false,
      });
      if (path) {
        importStatus = "Importing...";
        const result = await invoke<{
          root: Layer;
          globals: { name: string; type: GlobalVarType; value: string | number | boolean }[];
          warnings: string[];
          assetCount: number;
          assetDir: string;
        }>("import_komp", { path });

        const proj = getProject();
        if (!proj.assetDir && result.assetDir) {
          setProject({ ...proj, assetDir: result.assetDir });
        }

        for (const g of result.globals) {
          ensureGlobal(g.name, g.type, g.value);
        }

        insertWidget(result.root);

        const warnCount = result.warnings.length;
        importStatus = `Komponent imported! ${result.assetCount} assets`;
        if (warnCount > 0) {
          console.warn("Komp import warnings:", result.warnings);
        }
        setTimeout(() => { importStatus = ""; }, 4000);
      }
    } catch (e) {
      importStatus = "";
      console.error("Komp import failed:", e);
      alert(`Komponent import failed: ${e}`);
    }
  }

  let wallpaperStatus = $state("");
  let wallpaperActive = $state(false);

  async function handleWallpaperToggle() {
    const { invoke } = await import("@tauri-apps/api/core");
    try {
      if (wallpaperActive) {
        await invoke("stop_wallpaper_mode");
        wallpaperActive = false;
        wallpaperStatus = "Stopped";
      } else {
        wallpaperStatus = "Starting...";
        const server = await invoke<string>("start_wallpaper_mode", { project: getProjectSnapshot() });
        wallpaperActive = true;
        wallpaperStatus = `Live (${server})`;
        // Save last project path for auto-start
        const projPath = getCurrentProjectPath();
        if (projPath) updateSetting("lastProjectPath", projPath);
      }
      setTimeout(() => { if (!wallpaperActive) wallpaperStatus = ""; }, 3000);
    } catch (e) {
      wallpaperStatus = "";
      alert(`Wallpaper: ${e}`);
    }
  }

  // --- Add panel state ---
  let addPanelOpen = $state(false);
  let addPanelTab = $state<"layers" | "widgets" | "themes">("layers");

  function toggleAddPanel() {
    addPanelOpen = !addPanelOpen;
  }

  function addLayerAndClose(type: LayerType) {
    addLayer(type);
    addPanelOpen = false;
  }

  const layerButtons: { type: LayerType; label: string; icon: string }[] = [
    { type: "text", label: "Text", icon: "T" },
    { type: "shape", label: "Shape", icon: "\u25A0" },
    { type: "image", label: "Image", icon: "\u{1F5BC}" },
    { type: "progress", label: "Progress", icon: "\u25CB" },
    { type: "fonticon", label: "Icon", icon: "\u2605" },
    { type: "visualizer", label: "Visualizer", icon: "\u2248" },
    { type: "stack", label: "Stack", icon: "\u2261" },
    { type: "overlap", label: "Overlap", icon: "\u29C9" },
  ];

  // Close panel on outside click
  function handlePanelBackdrop() {
    addPanelOpen = false;
  }
</script>

<div class="toolbar">
  <div class="toolbar-left">
    <span class="project-name">{getProject().name}</span>
    {#if getIsDirty()}
      <span class="dirty-indicator">{"\u2022"}</span>
    {/if}
  </div>

  <div class="toolbar-center">
    <button
      class="add-btn"
      class:add-btn-active={addPanelOpen}
      title="Add layer, widget, or theme"
      onclick={toggleAddPanel}
    >
      <span class="add-icon">+</span>
    </button>
    {#if getSelectedLayer() && isContainerType(getSelectedLayer()!.type)}
      <span class="target-indicator">into {getSelectedLayer()!.name}</span>
    {/if}
  </div>

  <div class="toolbar-right">
    <button class="toolbar-btn" title="Copy (Ctrl+C)" disabled={!getSelectedLayerId()} onclick={handleCopy}>
      <span class="btn-label">Copy</span>
    </button>
    <button class="toolbar-btn" title="Paste (Ctrl+V)" disabled={!getCopiedLayer()} onclick={handlePaste}>
      <span class="btn-label">Paste</span>
    </button>
    <span class="separator"></span>
    <button class="toolbar-btn" title="Undo (Ctrl+Z)" disabled={!canUndo()} onclick={() => undo()}>
      <span class="btn-icon">{"\u21A9"}</span>
    </button>
    <button class="toolbar-btn" title="Redo (Ctrl+Y)" disabled={!canRedo()} onclick={() => redo()}>
      <span class="btn-icon">{"\u21AA"}</span>
    </button>
    <span class="separator"></span>
    <button
      class="toolbar-btn"
      class:interactive-active={getInteractiveMode()}
      title={getInteractiveMode() ? "Interactive mode ON" : "Interactive mode OFF"}
      onclick={() => setInteractiveMode(!getInteractiveMode())}
    >
      <span class="btn-icon">{getInteractiveMode() ? "\u25B6" : "\u23F8"}</span>
    </button>
    <button
      class="toolbar-btn"
      class:debug-active={getDebugOverlay()}
      title={getDebugOverlay() ? "Debug overlay ON" : "Debug overlay OFF"}
      onclick={() => setDebugOverlay(!getDebugOverlay())}
    >
      <span class="btn-icon">{getDebugOverlay() ? "\u{1F41E}" : "\u{1F50D}"}</span>
    </button>
    <button
      class="toolbar-btn"
      class:wallpaper-active={wallpaperActive}
      title={wallpaperActive ? "Stop live wallpaper" : "Apply as live wallpaper"}
      onclick={handleWallpaperToggle}
    >
      <span class="btn-icon">{wallpaperActive ? "\u23F9" : "\u{1F5BC}"}</span>
    </button>
    {#if wallpaperStatus}
      <span class="import-status">{wallpaperStatus}</span>
    {/if}
    <span class="separator"></span>
    <button class="toolbar-btn" title="Import .rock / .komp" onclick={handleImportKomp}>
      <span class="btn-label">Import</span>
    </button>
    {#if importStatus}
      <span class="import-status">{importStatus}</span>
    {/if}
    <span class="separator"></span>
    <button class="toolbar-btn" title="New project" onclick={handleNew}>
      <span class="btn-label">New</span>
    </button>
    <button class="toolbar-btn" title="Open project" onclick={handleLoad}>
      <span class="btn-label">Open</span>
    </button>
    <button class="toolbar-btn accent" title="Save project" onclick={handleSave}>
      <span class="btn-label">Save</span>
    </button>
  </div>
</div>

<!-- Add panel dropdown -->
{#if addPanelOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="add-panel-backdrop" onclick={handlePanelBackdrop}></div>
  <div class="add-panel">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="add-panel-tabs">
      <span
        class="add-panel-tab"
        style="border-bottom-color: {addPanelTab === 'layers' ? 'var(--accent)' : 'transparent'}; color: {addPanelTab === 'layers' ? 'var(--accent)' : 'var(--text-muted)'};"
        onclick={() => { addPanelTab = "layers"; }}
      >Layers</span>
      <span
        class="add-panel-tab"
        style="border-bottom-color: {addPanelTab === 'widgets' ? 'var(--accent)' : 'transparent'}; color: {addPanelTab === 'widgets' ? 'var(--accent)' : 'var(--text-muted)'};"
        onclick={() => { addPanelTab = "widgets"; }}
      >Widgets</span>
      <span
        class="add-panel-tab"
        style="border-bottom-color: {addPanelTab === 'themes' ? 'var(--accent)' : 'transparent'}; color: {addPanelTab === 'themes' ? 'var(--accent)' : 'var(--text-muted)'};"
        onclick={() => { addPanelTab = "themes"; }}
      >Themes</span>
    </div>
    <div class="add-panel-content">
      {#if addPanelTab === "layers"}
        <div class="layer-grid">
          {#each layerButtons as btn}
            <button
              class="layer-card"
              title={`Add ${btn.label}`}
              onclick={() => addLayerAndClose(btn.type)}
            >
              <span class="layer-card-icon">{btn.icon}</span>
              <span class="layer-card-label">{btn.label}</span>
            </button>
          {/each}
        </div>
      {:else if addPanelTab === "widgets"}
        <WidgetsPanel onAdd={() => { addPanelOpen = false; }} />
      {:else if addPanelTab === "themes"}
        <ThemesPanel />
      {/if}
    </div>
  </div>
{/if}

<style>
  .toolbar {
    height: var(--toolbar-height);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    padding: 0 12px;
    gap: 8px;
    user-select: none;
  }
  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 100px;
  }
  .project-name {
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
  }
  .dirty-indicator {
    color: var(--accent);
    font-size: 18px;
    line-height: 1;
  }
  .toolbar-center {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
    justify-content: flex-end;
  }

  /* + button */
  .add-btn {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-dim);
    color: var(--accent);
    border: 1px solid var(--accent);
    cursor: pointer;
    transition: all 0.15s;
  }
  .add-btn:hover, .add-btn-active {
    background: var(--accent);
    color: #fff;
  }
  .add-icon {
    font-size: 20px;
    font-weight: 300;
    line-height: 1;
  }

  .toolbar-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
    color: var(--text-secondary);
    transition: background 0.1s, color 0.1s;
  }
  .toolbar-btn:hover {
    background: var(--bg-panel);
    color: var(--text-primary);
  }
  .toolbar-btn.accent {
    background: var(--accent-dim);
    color: var(--accent);
  }
  .toolbar-btn.accent:hover {
    background: var(--accent);
    color: #fff;
  }
  .toolbar-btn.interactive-active {
    background: #b8860b;
    color: #fff;
  }
  .toolbar-btn.interactive-active:hover {
    background: #d4a017;
  }
  .toolbar-btn.debug-active {
    background: #e74c3c;
    color: #fff;
  }
  .toolbar-btn.debug-active:hover {
    background: #c0392b;
  }
  .toolbar-btn.wallpaper-active {
    background: #2d7d46;
    color: #fff;
  }
  .toolbar-btn.wallpaper-active:hover {
    background: #c0392b;
  }
  .toolbar-btn:disabled {
    opacity: 0.3;
    pointer-events: none;
  }
  .import-status {
    font-size: 11px;
    color: var(--accent);
    white-space: nowrap;
  }
  .btn-icon {
    font-size: 14px;
  }
  .btn-label {
    font-size: 12px;
  }
  .separator {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 2px;
  }
  .target-indicator {
    font-size: 10px;
    color: var(--accent);
    opacity: 0.8;
    white-space: nowrap;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--accent-dim);
  }

  /* Add panel dropdown */
  .add-panel-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 99;
  }
  .add-panel {
    position: fixed;
    top: var(--toolbar-height);
    left: 50%;
    transform: translateX(-50%);
    width: 480px;
    max-height: 520px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 0 0 8px 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    z-index: 100;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .add-panel-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
  }
  .add-panel-tab {
    flex: 1;
    padding: 10px 8px 8px;
    text-align: center;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.3px;
    text-transform: uppercase;
    border-bottom: 2px solid transparent;
    transition: color 0.15s;
  }
  .add-panel-tab:hover {
    color: var(--text-primary) !important;
  }
  .add-panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  /* Layer type grid */
  .layer-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .layer-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 16px 8px;
    border-radius: 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    cursor: pointer;
    transition: all 0.15s;
  }
  .layer-card:hover {
    background: var(--accent-dim);
    border-color: var(--accent);
    color: var(--accent);
  }
  .layer-card-icon {
    font-size: 24px;
    line-height: 1;
  }
  .layer-card-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .layer-card:hover .layer-card-label {
    color: var(--accent);
  }
</style>
