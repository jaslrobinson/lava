<script lang="ts">
  import { onMount } from "svelte";
  import { renderProject, getLayerBounds, type LayerBounds } from "./renderer";
  import {
    getProject,
    getSelectedLayerId,
    setSelectedLayerId,
    updateLayerProperty,
    getSelectedLayer,
    updateGlobal,
    getInteractiveMode,
    flattenLayers,
  } from "../stores/project.svelte";
  import type { Layer } from "../types/project";
  import { startFormulaLoop, stopFormulaLoop, evaluateSync, flushGlobalsNow, resolveFormula, hasFormula } from "../formula/service";
  import { setScrollPosition, triggerTap } from "./animationState";
  import { loadBundledIconFonts } from "../fonts/fontLoader";

  interface Props {
    fullscreen?: boolean;
  }
  let { fullscreen = false }: Props = $props();

  const isWallpaperView = new URLSearchParams(window.location.search).has("wallpaper");

  /** Open a URL — uses webkit message handler in wallpaper mode, Tauri IPC otherwise */
  async function openUrl(url: string) {
    if (isWallpaperView && (window as any).webkit?.messageHandlers?.klwp) {
      (window as any).webkit.messageHandlers.klwp.postMessage(
        JSON.stringify({ type: "open_url", url })
      );
    } else {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("open_url", { url });
      } catch (e) {
        console.error("Failed to open URL:", e);
      }
    }
  }

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let containerEl: HTMLDivElement;
  let baseScale = $state(1);
  let canvasStyleWidth = $state(800);
  let canvasStyleHeight = $state(450);
  let canvasReady = $state(false);

  // Zoom/pan state — applied as canvas context transform, not CSS
  let zoom = $state(1);
  let panX = $state(0); // in canvas internal coords
  let panY = $state(0);

  // Interaction state
  type DragMode = "none" | "move" | "pan" | "resize";
  let dragMode = $state<DragMode>("none");
  let dragOffset = $state({ x: 0, y: 0 });
  let panStart = $state({ x: 0, y: 0, px: 0, py: 0 });
  let resizeHandleIdx = $state(-1);
  let resizeStartMouse = $state({ x: 0, y: 0 });
  let resizeStartProps = $state({ x: 0, y: 0, w: 0, h: 0 });
  let resizeAnchorFx = $state(0);
  let resizeAnchorFy = $state(0);

  function getAnchorFactors(anchor: string): { fx: number; fy: number } {
    switch (anchor) {
      case "top-center":    return { fx: -0.5, fy: 0 };
      case "top-right":     return { fx: -1,   fy: 0 };
      case "center-left":   return { fx: 0,    fy: -0.5 };
      case "center":        return { fx: -0.5, fy: -0.5 };
      case "center-right":  return { fx: -1,   fy: -0.5 };
      case "bottom-left":   return { fx: 0,    fy: -1 };
      case "bottom-center": return { fx: -0.5, fy: -1 };
      case "bottom-right":  return { fx: -1,   fy: -1 };
      default:              return { fx: 0,    fy: 0 }; // top-left
    }
  }
  let hoveredHandle = $state(-1);
  let hoveredLayerId = $state<string | null>(null);
  let spaceHeld = $state(false);

  const HANDLE_HIT_RADIUS = 10;
  const HANDLE_PAD = 2;
  let rafId: number;

  onMount(() => {
    ctx = canvas.getContext("2d")!;
    updateCanvasSize();
    loadBundledIconFonts();
    rafId = requestAnimationFrame(renderLoop);

    startFormulaLoop(() => {
      const project = getProject();
      const globals: Record<string, string> = {};
      for (const g of project.globals) {
        globals[g.name] = String(g.value);
      }
      return globals;
    }).then(() => { canvasReady = true; });

    const observer = new ResizeObserver(() => updateCanvasSize());
    observer.observe(containerEl);

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.code === "Space" && !e.repeat && (e.target === document.body || containerEl.contains(e.target as Node))) {
        spaceHeld = true;
        e.preventDefault();
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.code === "Space") spaceHeld = false;
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);

    return () => {
      cancelAnimationFrame(rafId);
      observer.disconnect();
      stopFormulaLoop();
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
    };
  });

  function updateCanvasSize() {
    if (!containerEl) return;
    const project = getProject();
    const containerRect = containerEl.getBoundingClientRect();
    if (containerRect.width === 0 || containerRect.height === 0) return;

    if (fullscreen) {
      canvas.width = project.resolution.width;
      canvas.height = project.resolution.height;
      canvasStyleWidth = containerRect.width;
      canvasStyleHeight = containerRect.height;
      baseScale = containerRect.width / project.resolution.width;
    } else {
      const scaleX = (containerRect.width - 40) / project.resolution.width;
      const scaleY = (containerRect.height - 40) / project.resolution.height;
      baseScale = Math.min(scaleX, scaleY, 1);

      canvas.width = project.resolution.width;
      canvas.height = project.resolution.height;

      canvasStyleWidth = Math.round(project.resolution.width * baseScale);
      canvasStyleHeight = Math.round(project.resolution.height * baseScale);
    }
  }

  function renderLoop(timestamp: number) {
    if (!ctx) {
      rafId = requestAnimationFrame(renderLoop);
      return;
    }
    const project = getProject();
    const selectedId = fullscreen ? null : getSelectedLayerId();

    if (!fullscreen && (zoom !== 1 || panX !== 0 || panY !== 0)) {
      // Clear full canvas with neutral background (visible when zoomed out)
      ctx.save();
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.fillStyle = "#0d0d1a";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.restore();

      // Apply zoom/pan as canvas context transform
      ctx.save();
      ctx.translate(panX, panY);
      ctx.scale(zoom, zoom);
      renderProject(ctx, project, selectedId, timestamp, hoveredLayerId);
      ctx.restore();
    } else {
      renderProject(ctx, project, selectedId, timestamp, hoveredLayerId);
    }

    rafId = requestAnimationFrame(renderLoop);
  }

  /** Convert screen coords to project coords, accounting for zoom/pan */
  function canvasToProject(clientX: number, clientY: number): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    // Screen → canvas internal coords
    const cx = (clientX - rect.left) * (canvas.width / rect.width);
    const cy = (clientY - rect.top) * (canvas.height / rect.height);
    // Reverse zoom/pan transform → project coords
    return {
      x: (cx - panX) / zoom,
      y: (cy - panY) / zoom,
    };
  }

  /** Convert screen pixel delta to canvas internal coord delta (for pan) */
  function screenToCanvasDelta(screenDx: number, screenDy: number): { dx: number; dy: number } {
    const rect = canvas.getBoundingClientRect();
    return {
      dx: screenDx * (canvas.width / rect.width),
      dy: screenDy * (canvas.height / rect.height),
    };
  }

  /** Handle positions in project coords, matching drawSelectionOutline in renderer.ts */
  function getHandlePositions(b: LayerBounds): [number, number][] {
    const p = HANDLE_PAD;
    return [
      [b.x - p,       b.y - p],       // 0: top-left
      [b.x + b.w / 2, b.y - p],       // 1: top-center
      [b.x + b.w + p, b.y - p],       // 2: top-right
      [b.x + b.w + p, b.y + b.h / 2], // 3: center-right
      [b.x + b.w + p, b.y + b.h + p], // 4: bottom-right
      [b.x + b.w / 2, b.y + b.h + p], // 5: bottom-center
      [b.x - p,       b.y + b.h + p], // 6: bottom-left
      [b.x - p,       b.y + b.h / 2], // 7: center-left
    ];
  }

  function hitTestHandle(projX: number, projY: number): number {
    const selectedId = getSelectedLayerId();
    if (!selectedId) return -1;
    const bounds = getLayerBounds().get(selectedId);
    if (!bounds) return -1;
    const handles = getHandlePositions(bounds);
    const r = HANDLE_HIT_RADIUS / (baseScale * zoom);
    for (let i = 0; i < handles.length; i++) {
      const dx = projX - handles[i][0];
      const dy = projY - handles[i][1];
      if (dx * dx + dy * dy <= r * r) return i;
    }
    return -1;
  }

  function hitTest(clientX: number, clientY: number): string | null {
    const { x, y } = canvasToProject(clientX, clientY);
    const bounds = getLayerBounds();
    let hit: string | null = null;
    for (const [id, b] of bounds) {
      if (x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h) {
        hit = id;
      }
    }
    return hit;
  }

  const HANDLE_CURSORS = ["nwse-resize", "ns-resize", "nesw-resize", "ew-resize", "nwse-resize", "ns-resize", "nesw-resize", "ew-resize"];

  function getCursorStyle(): string {
    if (dragMode === "pan") return "grabbing";
    if (spaceHeld) return "grab";
    if (dragMode === "resize") return HANDLE_CURSORS[resizeHandleIdx] || "default";
    if (hoveredHandle >= 0) return HANDLE_CURSORS[hoveredHandle] || "default";
    if (dragMode === "move") return "move";
    return "crosshair";
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const factor = e.deltaY > 0 ? 0.9 : 1.1;
    const newZoom = Math.max(0.1, Math.min(10, zoom * factor));

    // Zoom toward cursor: keep the project point under the mouse fixed
    const rect = canvas.getBoundingClientRect();
    const cx = (e.clientX - rect.left) * (canvas.width / rect.width);
    const cy = (e.clientY - rect.top) * (canvas.height / rect.height);

    // Adjust pan so (cx - panX) / zoom == (cx - newPanX) / newZoom
    panX = cx - (cx - panX) * newZoom / zoom;
    panY = cy - (cy - panY) * newZoom / zoom;
    zoom = newZoom;
  }

  function onMouseDown(e: MouseEvent) {
    // Middle button or Space+click: start panning
    if (e.button === 1 || (spaceHeld && e.button === 0)) {
      dragMode = "pan";
      panStart = { x: e.clientX, y: e.clientY, px: panX, py: panY };
      e.preventDefault();
      return;
    }

    if (e.button !== 0) return;

    const proj = canvasToProject(e.clientX, e.clientY);

    // Check resize handles first (skip in interactive mode — click actions take priority)
    if (!getInteractiveMode()) {
      const hIdx = hitTestHandle(proj.x, proj.y);
      if (hIdx >= 0) {
        const layer = getSelectedLayer();
        if (layer) {
          dragMode = "resize";
          resizeHandleIdx = hIdx;
          resizeStartMouse = { x: proj.x, y: proj.y };
          resizeStartProps = {
            x: Number(layer.properties.x) || 0,
            y: Number(layer.properties.y) || 0,
            w: Number(layer.properties.width) || 100,
            h: Number(layer.properties.height) || 100,
          };
          const anchor = layer.properties.anchor || "top-left";
          const factors = getAnchorFactors(anchor);
          resizeAnchorFx = factors.fx;
          resizeAnchorFy = factors.fy;
          return;
        }
      }
    }

    // Hit test layers
    const id = hitTest(e.clientX, e.clientY);
    setSelectedLayerId(id);
    if (id) {
      triggerTap(id, performance.now());
      if (getInteractiveMode()) {
        const project = getProject();
        const action = findClickActionForHit(project.layers, id);
        if (action) {
          handleClickAction(action, project);
          return; // Don't start a drag after firing a click action
        }
      }
    }

    if (id) {
      const layer = getSelectedLayer();
      if (layer && !layer.locked) {
        dragMode = "move";
        dragOffset = {
          x: proj.x - (Number(layer.properties.x) || 0),
          y: proj.y - (Number(layer.properties.y) || 0),
        };
      }
    }
  }

  function onMouseMove(e: MouseEvent) {
    setScrollPosition(e.clientX / window.innerWidth);

    if (dragMode === "pan") {
      // Convert screen pixel delta to canvas internal coords
      const d = screenToCanvasDelta(e.clientX - panStart.x, e.clientY - panStart.y);
      panX = panStart.px + d.dx;
      panY = panStart.py + d.dy;
      return;
    }

    if (dragMode === "resize") {
      const selectedId = getSelectedLayerId();
      if (!selectedId) return;
      const proj = canvasToProject(e.clientX, e.clientY);
      const dx = proj.x - resizeStartMouse.x;
      const dy = proj.y - resizeStartMouse.y;
      const s = resizeStartProps;

      // Compute the visual edge deltas for each handle
      let dLeft = 0, dRight = 0, dTop = 0, dBottom = 0;
      switch (resizeHandleIdx) {
        case 0: dLeft = dx; dTop = dy; break;           // top-left
        case 1: dTop = dy; break;                         // top-center
        case 2: dRight = dx; dTop = dy; break;           // top-right
        case 3: dRight = dx; break;                       // center-right
        case 4: dRight = dx; dBottom = dy; break;        // bottom-right
        case 5: dBottom = dy; break;                      // bottom-center
        case 6: dLeft = dx; dBottom = dy; break;         // bottom-left
        case 7: dLeft = dx; break;                        // center-left
      }

      let newW = s.w + dRight - dLeft;
      let newH = s.h + dBottom - dTop;

      // Clamp minimum dimensions
      if (newW < 10) {
        if (dLeft !== 0) dLeft = s.w - 10;
        if (dRight !== 0) dRight = 10 - s.w + dLeft;
        newW = 10;
      }
      if (newH < 10) {
        if (dTop !== 0) dTop = s.h - 10;
        if (dBottom !== 0) dBottom = 10 - s.h + dTop;
        newH = 10;
      }

      // Apply anchor correction: compensate for how anchor positioning shifts the origin when width/height change
      const dw = newW - s.w;
      const dh = newH - s.h;
      const newX = s.x + dLeft - resizeAnchorFx * dw;
      const newY = s.y + dTop - resizeAnchorFy * dh;

      updateLayerProperty(selectedId, "width", Math.round(newW));
      updateLayerProperty(selectedId, "height", Math.round(newH));
      if (dLeft !== 0 || resizeAnchorFx !== 0) updateLayerProperty(selectedId, "x", Math.round(newX));
      if (dTop !== 0 || resizeAnchorFy !== 0) updateLayerProperty(selectedId, "y", Math.round(newY));
      return;
    }

    if (dragMode === "move") {
      const selectedId = getSelectedLayerId();
      if (!selectedId) return;
      const pos = canvasToProject(e.clientX, e.clientY);
      updateLayerProperty(selectedId, "x", Math.round(pos.x - dragOffset.x));
      updateLayerProperty(selectedId, "y", Math.round(pos.y - dragOffset.y));
      return;
    }

    // Hover: update cursor based on handle proximity
    const proj = canvasToProject(e.clientX, e.clientY);
    hoveredHandle = hitTestHandle(proj.x, proj.y);

    // Track hovered layer for hover animations
    if (fullscreen || getInteractiveMode()) {
      const hit = hitTest(e.clientX, e.clientY);
      if (hit !== hoveredLayerId) {
        hoveredLayerId = hit;
      }
    }
  }

  function onMouseUp() {
    dragMode = "none";
    resizeHandleIdx = -1;
  }

  function onFullscreenMouseMove(e: MouseEvent) {
    setScrollPosition(e.clientX / window.innerWidth);

    // Track hovered layer for hover animations
    const hit = hitTest(e.clientX, e.clientY);
    if (hit !== hoveredLayerId) {
      hoveredLayerId = hit;
    }
  }

  /** Walk the layer tree to find the path from root to a target layer */
  function findPathToLayer(layers: Layer[], targetId: string, path: Layer[] = []): Layer[] | null {
    for (const layer of layers) {
      if (layer.id === targetId) return [...path, layer];
      if (layer.children) {
        const result = findPathToLayer(layer.children, targetId, [...path, layer]);
        if (result) return result;
      }
    }
    return null;
  }

  /** Find the nearest clickAction walking from hit layer up to root */
  function findClickActionForHit(layers: Layer[], hitId: string): string | null {
    const path = findPathToLayer(layers, hitId);
    if (!path) return null;
    for (let i = path.length - 1; i >= 0; i--) {
      if (path[i].properties.clickAction) return path[i].properties.clickAction!;
    }
    return null;
  }

  function onFullscreenClick(e: MouseEvent) {
    const id = hitTest(e.clientX, e.clientY);
    if (id) {
      triggerTap(id, performance.now());
      const project = getProject();
      const action = findClickActionForHit(project.layers, id);
      if (action) {
        handleClickAction(action, project);
      }
    }
  }

  function buildGlobalsMap(project: ReturnType<typeof getProject>): Record<string, string> {
    const globals: Record<string, string> = {};
    for (const g of project.globals) {
      const raw = String(g.value);
      // Resolve formula-valued globals through the cache so downstream
      // gv() lookups return the evaluated value, not the raw formula text.
      globals[g.name] = hasFormula(raw) ? resolveFormula(raw) : raw;
    }
    return globals;
  }

  async function handleClickAction(action: string, project: ReturnType<typeof getProject>) {
    if (action.startsWith("set:")) {
      // set:varName:value — set a global variable
      const parts = action.split(":");
      const varName = parts[1];
      const value = parts.slice(2).join(":");
      updateGlobal(varName, "value", value);
      flushGlobalsNow(buildGlobalsMap(getProject()));
    } else if (action.startsWith("inc:")) {
      // inc:varName:amount — increment a numeric global
      const parts = action.split(":");
      const varName = parts[1];
      const amount = parseInt(parts[2]) || 0;
      const global = project.globals.find(g => g.name === varName);
      const current = parseInt(String(global?.value ?? "0")) || 0;
      updateGlobal(varName, "value", Math.max(0, current + amount));
      flushGlobalsNow(buildGlobalsMap(getProject()));
    } else if (action.startsWith("url:")) {
      // url:formulaOrLiteral — resolve formula and open in browser
      const urlExpr = action.slice(4);
      let resolved = evaluateSync(urlExpr, buildGlobalsMap(project));
      // If sync eval didn't resolve (still contains $), try async via Tauri
      if (resolved.includes("$")) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          resolved = await invoke<string>("evaluate_formula", {
            formula: urlExpr,
            globals: buildGlobalsMap(project),
          });
        } catch (e) {
          console.warn("Async formula eval failed:", e);
        }
      }
      resolved = resolved.trim();
      if (resolved && (resolved.startsWith("http://") || resolved.startsWith("https://"))) {
        openUrl(resolved);
      } else {
        console.warn("Click action url: resolved to non-URL:", resolved, "(from:", urlExpr, ")");
      }
    } else if (action.startsWith("music:")) {
      // music:play-pause, music:next, music:previous, music:play, music:pause, music:stop
      const musicAction = action.slice(6);
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("music_control", { action: musicAction });
      } catch (e) {
        console.warn("Music control failed:", e);
      }
    } else if (action.startsWith("app:")) {
      // app:command — launch application
      const cmd = action.slice(4);
      const isTauriCtx = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";
      if (isTauriCtx) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("launch_app", { command: cmd });
        } catch (e) {
          console.warn("Launch app failed:", e);
        }
      } else {
        (window as any).webkit?.messageHandlers?.klwp?.postMessage(
          JSON.stringify({ type: "launch_app", command: cmd })
        );
      }
    } else if (action.startsWith("overlay:")) {
      // overlay:layerName — toggle visibility of a named layer
      const targetName = action.slice(8).toLowerCase();
      const allLayers = flattenLayers(project.layers);
      const target = allLayers.find(l => l.name.toLowerCase() === targetName);
      if (target) {
        // Toggle: if currently hidden, show; if visible, hide
        const isVisible = target.visible !== false && target.properties.visible !== false && target.properties.visible !== "NEVER";
        updateLayerProperty(target.id, "visible", isVisible ? false : true);
      }
    }
  }

  function resetView() {
    zoom = 1;
    panX = 0;
    panY = 0;
  }

  function onDblClick() {
    if (zoom !== 1 || panX !== 0 || panY !== 0) {
      resetView();
    }
  }
</script>

<div class="canvas-container" bind:this={containerEl} style={fullscreen ? "background: transparent;" : ""}>
  <canvas
    bind:this={canvas}
    style="width: {canvasStyleWidth}px; height: {canvasStyleHeight}px;{fullscreen ? ' box-shadow: none;' : ''} cursor: {fullscreen ? 'default' : getCursorStyle()}; opacity: {canvasReady ? 1 : 0}; transition: opacity 0.3s ease;"
    onmousedown={fullscreen ? undefined : onMouseDown}
    onmousemove={fullscreen ? onFullscreenMouseMove : onMouseMove}
    onmouseup={fullscreen ? undefined : onMouseUp}
    onmouseleave={fullscreen ? undefined : onMouseUp}
    onclick={fullscreen ? onFullscreenClick : undefined}
    ondblclick={fullscreen ? undefined : onDblClick}
    onwheel={fullscreen ? undefined : onWheel}
  />
  {#if !fullscreen && zoom !== 1}
    <button class="zoom-indicator" onclick={resetView} title="Click to reset view">
      {Math.round(zoom * 100)}%
    </button>
  {/if}
</div>

<style>
  .canvas-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    background: #111;
    position: relative;
  }
  canvas {
    box-shadow: 0 0 20px rgba(0, 0, 0, 0.5);
  }
  .zoom-indicator {
    position: absolute;
    bottom: 12px;
    right: 12px;
    padding: 4px 10px;
    background: rgba(0, 0, 0, 0.7);
    color: #ccc;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    cursor: pointer;
    user-select: none;
    z-index: 10;
  }
  .zoom-indicator:hover {
    background: rgba(0, 0, 0, 0.85);
    color: #fff;
    border-color: rgba(255, 255, 255, 0.3);
  }
</style>
