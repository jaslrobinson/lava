<script lang="ts">
  import { onMount } from "svelte";
  import Toolbar from "./lib/editor/Toolbar.svelte";
  import LeftPanel from "./lib/editor/LeftPanel.svelte";
  import CanvasRenderer from "./lib/canvas/CanvasRenderer.svelte";
  import PropertyPanel from "./lib/editor/PropertyPanel.svelte";
  import FormulaBar from "./lib/editor/FormulaBar.svelte";
  import { getWallpaperMode, setWallpaperMode, setProject } from "./lib/stores/project.svelte";

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
