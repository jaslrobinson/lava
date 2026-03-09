<script lang="ts">
  import { onMount } from "svelte";
  import Toolbar from "./lib/editor/Toolbar.svelte";
  import LeftPanel from "./lib/editor/LeftPanel.svelte";
  import CanvasRenderer from "./lib/canvas/CanvasRenderer.svelte";
  import PropertyPanel from "./lib/editor/PropertyPanel.svelte";
  import FormulaBar from "./lib/editor/FormulaBar.svelte";
  import { getWallpaperMode, setWallpaperMode, setProject, copySelectedLayer, pasteLayer, getSelectedLayerId, getCopiedLayer, removeLayer, undo, redo } from "./lib/stores/project.svelte";

  // Check if this is the wallpaper helper webview (loaded with ?wallpaper=true)
  const isWallpaperView = new URLSearchParams(window.location.search).has("wallpaper");

  onMount(async () => {
    if (isWallpaperView) {
      // Wait for injected project data from the helper binary
      const checkProject = () => {
        const proj = (window as any).__KLWP_PROJECT;
        if (proj) {
          setProject(proj);
        } else {
          setTimeout(checkProject, 100);
        }
      };
      checkProject();
      return;
    }

    // Editor mode: global keyboard shortcuts
    const onKeyDown = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      if ((e.ctrlKey || e.metaKey) && e.key === "c") {
        if (getSelectedLayerId()) { copySelectedLayer(); e.preventDefault(); }
      } else if ((e.ctrlKey || e.metaKey) && e.key === "v") {
        if (getCopiedLayer()) {
          const name = prompt("Name for pasted layer:", getCopiedLayer()!.name + " copy");
          if (name !== null) { pasteLayer(name); e.preventDefault(); }
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key === "z") {
        undo(); e.preventDefault();
      } else if ((e.ctrlKey || e.metaKey) && (e.key === "y" || (e.shiftKey && e.key === "Z"))) {
        redo(); e.preventDefault();
      } else if (e.key === "Delete") {
        const id = getSelectedLayerId();
        if (id) {
          if (confirm("Delete selected layer?")) { removeLayer(id); e.preventDefault(); }
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);

    // Listen for global shortcut exit event (Super+Escape)
    const { listen } = await import("@tauri-apps/api/event");
    const unlistenExit = await listen("exit-wallpaper", () => {
      exitWallpaperMode();
    });

    // Listen for tray "Stop Wallpaper" event
    const unlistenTrayStop = await listen("tray-stop-wallpaper", () => {
      exitWallpaperMode();
    });

    // Listen for tray "Start Wallpaper" event (show editor so user can start)
    const unlistenTrayStart = await listen("tray-start-wallpaper", () => {
      // The editor window is already shown by the tray handler;
      // this event is a hint to the frontend if needed in future.
    });

    return () => {
      window.removeEventListener("keydown", onKeyDown);
      unlistenExit();
      unlistenTrayStop();
      unlistenTrayStart();
    };
  });

  async function exitWallpaperMode() {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("stop_wallpaper_mode");
      setWallpaperMode(false);
    } catch (e) {
      console.error("Failed to stop wallpaper:", e);
    }
  }

  function handleWallpaperKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") exitWallpaperMode();
  }
</script>

{#if isWallpaperView}
  <div class="wallpaper-mode">
    <CanvasRenderer fullscreen={true} />
  </div>
{:else if getWallpaperMode()}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="wallpaper-mode" tabindex="0" onkeydown={handleWallpaperKeydown}>
    <CanvasRenderer fullscreen={true} />
  </div>
{:else}
  <div class="app-layout">
    <Toolbar />
    <div class="main-area">
      <LeftPanel />
      <CanvasRenderer />
      <PropertyPanel />
    </div>
    <FormulaBar />
  </div>
{/if}

<style>
  .app-layout {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .main-area {
    flex: 1;
    display: flex;
    overflow: hidden;
  }
  .wallpaper-mode {
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    display: flex;
    cursor: default;
    background: transparent;
  }
</style>
