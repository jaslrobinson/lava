<script lang="ts">
  import { addLayer, getProject, setProject, getIsDirty, getSelectedLayer, isContainerType, setWallpaperMode } from "../stores/project.svelte";
  import type { LayerType, Project } from "../types/project";

  async function handleSave() {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await save({
        filters: [{ name: "KustomLinux Project", extensions: ["klwp", "json"] }],
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
        filters: [{ name: "KustomLinux Project", extensions: ["klwp", "json"] }],
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

  async function handleImportKlwp() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await open({
        filters: [{ name: "KLWP Preset", extensions: ["klwp"] }],
        multiple: false,
      });
      if (path) {
        importStatus = "Importing...";
        const result = await invoke<{
          project: Project;
          warnings: string[];
          assetCount: number;
          assetDir: string;
        }>("import_klwp", {
          path,
          targetWidth: getProject().resolution.width || 1920,
          targetHeight: getProject().resolution.height || 1080,
        });
        result.project.assetDir = result.assetDir;
        setProject(result.project);
        const warnCount = result.warnings.length;
        importStatus = `Imported! ${result.assetCount} assets, ${result.project.layers.length} layers`;
        if (warnCount > 0) {
          console.warn("KLWP Import warnings:", result.warnings);
        }
        setTimeout(() => { importStatus = ""; }, 4000);
      }
    } catch (e) {
      importStatus = "";
      console.error("KLWP Import failed:", e);
      alert(`Import failed: ${e}`);
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
    <button class="toolbar-btn" title="Import .klwp preset" onclick={handleImportKlwp}>
      <span class="btn-label">Import KLWP</span>
    </button>
    {#if importStatus}
      <span class="import-status">{importStatus}</span>
    {/if}
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
