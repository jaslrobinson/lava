<script lang="ts">
  import { addLayer, getProject, setProject, getIsDirty, getSelectedLayer, isContainerType, getInteractiveMode, setInteractiveMode, getSelectedLayerId, copySelectedLayer, getCopiedLayer, pasteLayer, undo, redo, canUndo, canRedo, insertWidget, ensureGlobal } from "../stores/project.svelte";
  import type { LayerType, Project, GlobalVarType, Layer } from "../types/project";
  import { createDefaultProject } from "../types/project";
  import { setDebugOverlay, getDebugOverlay } from "../canvas/renderer";

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
  }

  async function handleSave() {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await save({
        filters: [{ name: "KLLW Project", extensions: ["klwp", "json"] }],
        defaultPath: `${getProject().name}.klwp`,
      });
      if (path) {
        await invoke("save_project", { path, project: getProject() });
      }
    } catch (e) {
      console.error("Save failed:", e);
    }
  }

  async function handleLoad() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await open({
        filters: [{ name: "KLLW Project", extensions: ["klwp", "json"] }],
        multiple: false,
      });
      if (path) {
        const project = await invoke<Project>("load_project", { path });
        setProject(project);
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
        filters: [{ name: "KLWP Komponent", extensions: ["komp"] }],
        multiple: false,
      });
      if (path) {
        importStatus = "Importing komponent...";
        const result = await invoke<{
          root: Layer;
          globals: { name: string; type: GlobalVarType; value: string | number | boolean }[];
          warnings: string[];
          assetCount: number;
          assetDir: string;
        }>("import_komp", { path });

        // Store the asset directory in the project for path resolution
        const proj = getProject();
        if (!proj.assetDir && result.assetDir) {
          setProject({ ...proj, assetDir: result.assetDir });
        }

        // Ensure all globals from the komponent exist in the project
        for (const g of result.globals) {
          ensureGlobal(g.name, g.type, g.value);
        }

        // Insert the komponent as a widget layer
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

  async function handleImportRmskin() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await open({
        filters: [{ name: "Rainmeter Skin", extensions: ["rmskin"] }],
        multiple: false,
      });
      if (path) {
        importStatus = "Importing Rainmeter skin...";
        const result = await invoke<{
          root: Layer;
          globals: { name: string; type: GlobalVarType; value: string | number | boolean }[];
          warnings: string[];
          assetCount: number;
          assetDir: string;
        }>("import_rmskin", { path });

        // Store the asset directory in the project for path resolution
        const proj = getProject();
        if (!proj.assetDir && result.assetDir) {
          setProject({ ...proj, assetDir: result.assetDir });
        }

        for (const g of result.globals) {
          ensureGlobal(g.name, g.type, g.value);
        }

        insertWidget(result.root);

        const warnCount = result.warnings.length;
        importStatus = `Rainmeter imported! ${result.assetCount} assets`;
        if (warnCount > 0) {
          console.warn("Rainmeter import warnings:", result.warnings);
        }
        setTimeout(() => { importStatus = ""; }, 4000);
      }
    } catch (e) {
      importStatus = "";
      console.error("Rainmeter import failed:", e);
      alert(`Rainmeter import failed: ${e}`);
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
        const server = await invoke<string>("start_wallpaper_mode", { project: getProject() });
        wallpaperActive = true;
        wallpaperStatus = `Live (${server})`;
      }
      setTimeout(() => { if (!wallpaperActive) wallpaperStatus = ""; }, 3000);
    } catch (e) {
      wallpaperStatus = "";
      alert(`Wallpaper: ${e}`);
    }
  }

  const layerButtons: { type: LayerType; label: string; icon: string }[] = [
    { type: "text", label: "Text", icon: "T" },
    { type: "shape", label: "Shape", icon: "\u25A0" },
    { type: "image", label: "Image", icon: "\u{1F5BC}" },
    { type: "progress", label: "Progress", icon: "\u25CB" },
    { type: "fonticon", label: "Icon", icon: "\u2605" },
    { type: "group", label: "Group", icon: "\u{1F4C1}" },
    { type: "stack", label: "Stack", icon: "\u2261" },
    { type: "overlap", label: "Overlap", icon: "\u29C9" },
  ];
</script>

<div class="toolbar">
  <div class="toolbar-left">
    <span class="project-name">{getProject().name}</span>
    {#if getIsDirty()}
      <span class="dirty-indicator" title="Unsaved changes">\u2022</span>
    {/if}
  </div>

  <div class="toolbar-center">
    {#each layerButtons as btn}
      {@const selected = getSelectedLayer()}
      {@const targetName = selected && isContainerType(selected.type) ? selected.name : null}
      <button
        class="toolbar-btn"
        title={targetName ? `Add ${btn.label} into ${targetName}` : `Add ${btn.label} layer`}
        onclick={() => addLayer(btn.type)}
      >
        <span class="btn-icon">{btn.icon}</span>
        <span class="btn-label">{btn.label}</span>
      </button>
    {/each}
    {#if getSelectedLayer() && isContainerType(getSelectedLayer()!.type)}
      <span class="target-indicator">into {getSelectedLayer()!.name}</span>
    {/if}
  </div>

  <div class="toolbar-right">
    <button
      class="toolbar-btn"
      title="Copy selected layer (Ctrl+C)"
      disabled={!getSelectedLayerId()}
      onclick={handleCopy}
    >
      <span class="btn-label">Copy</span>
    </button>
    <button
      class="toolbar-btn"
      title="Paste layer (Ctrl+V)"
      disabled={!getCopiedLayer()}
      onclick={handlePaste}
    >
      <span class="btn-label">Paste</span>
    </button>
    <span class="separator"></span>
    <button
      class="toolbar-btn"
      title="Undo (Ctrl+Z)"
      disabled={!canUndo()}
      onclick={() => undo()}
    >
      <span class="btn-icon">{"\u21A9"}</span>
    </button>
    <button
      class="toolbar-btn"
      title="Redo (Ctrl+Y)"
      disabled={!canRedo()}
      onclick={() => redo()}
    >
      <span class="btn-icon">{"\u21AA"}</span>
    </button>
    <span class="separator"></span>
    <button
      class="toolbar-btn"
      class:interactive-active={getInteractiveMode()}
      title={getInteractiveMode() ? "Interactive mode ON — clicks trigger actions. Click to disable." : "Interactive mode OFF — clicks only select layers. Click to enable."}
      onclick={() => setInteractiveMode(!getInteractiveMode())}
    >
      <span class="btn-icon">{getInteractiveMode() ? "\u25B6" : "\u23F8"}</span>
      <span class="btn-label">{getInteractiveMode() ? "Interactive" : "Edit Only"}</span>
    </button>
    <button
      class="toolbar-btn"
      class:debug-active={getDebugOverlay()}
      title={getDebugOverlay() ? "Debug overlay ON — showing bounds & coordinates. Click to disable." : "Debug overlay OFF — click to show bounds & coordinate markers."}
      onclick={() => setDebugOverlay(!getDebugOverlay())}
    >
      <span class="btn-icon">{getDebugOverlay() ? "\u{1F41E}" : "\u{1F50D}"}</span>
      <span class="btn-label">{getDebugOverlay() ? "Debug ON" : "Debug"}</span>
    </button>
    <button
      class="toolbar-btn"
      class:wallpaper-active={wallpaperActive}
      title={wallpaperActive ? "Stop live wallpaper" : "Apply as live wallpaper"}
      onclick={handleWallpaperToggle}
    >
      <span class="btn-icon">{wallpaperActive ? "\u23F9" : "\u{1F5BC}"}</span>
      <span class="btn-label">{wallpaperActive ? "Stop Wallpaper" : "Wallpaper"}</span>
    </button>
    {#if wallpaperStatus}
      <span class="import-status">{wallpaperStatus}</span>
    {/if}
    <button class="toolbar-btn" title="Import KLWP Komponent" onclick={handleImportKomp}>
      <span class="btn-label">Import .komp</span>
    </button>
    <button class="toolbar-btn" title="Import Rainmeter Skin (.rmskin)" onclick={handleImportRmskin}>
      <span class="btn-label">Import .rmskin</span>
    </button>
    {#if importStatus}
      <span class="import-status">{importStatus}</span>
    {/if}
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
    min-width: 120px;
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
    gap: 4px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .toolbar-center::-webkit-scrollbar {
    display: none;
  }
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 120px;
    justify-content: flex-end;
  }
  .toolbar-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
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
    margin: 0 4px;
  }
  .toolbar-btn:disabled {
    opacity: 0.3;
    pointer-events: none;
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
</style>
