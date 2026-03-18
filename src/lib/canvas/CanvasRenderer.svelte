<script lang="ts">
  import { onMount } from "svelte";
  import { renderProject, getLayerBounds, type LayerBounds, hasAnimatedGifs } from "./renderer";
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
  import { markDirty, shouldRenderFullFps, hasActiveLoopingAnimations, setWakeCallback, setIdleTimeout, needsRepaint, clearRepaint, IDLE_INTERVAL_MS } from "./renderScheduler";
  import MapOverlay from "./MapOverlay.svelte";
  import { launcherHitRegions, setLauncherHoverCoords, toggleStartMenu, closeStartMenu } from "./renderer";
  import StartMenuOverlay from "./StartMenuOverlay.svelte";
  import { getSettings } from "../stores/settings.svelte";

  interface Props {
    fullscreen?: boolean;
  }
  let { fullscreen = false }: Props = $props();

  const isWallpaperView = new URLSearchParams(window.location.search).has("wallpaper");

  /** Open a URL — uses webkit message handler in wallpaper mode, Tauri IPC otherwise */
  async function openUrl(url: string) {
    if (isWallpaperView && (window as any).webkit?.messageHandlers?.lava) {
      (window as any).webkit.messageHandlers.lava.postMessage(
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
  let startMenuVisible = $state(false);
  let startMenuAnchorBounds = $state<{x:number;y:number;w:number;h:number} | null>(null);
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
  let canvasOffsetX = $state(0);
  let canvasOffsetY = $state(0);
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

  // Search overlay state
  let searchOverlayOpen = $state(false);
  let searchOverlayBounds = $state<{ x: number; y: number; w: number; h: number } | null>(null);
  let searchOverlayEl: HTMLDivElement | undefined = $state(undefined);

  // Collect all visible map layers from the project (for HTML overlays)
  let mapLayers = $derived.by(() => {
    const project = getProject();
    const result: import("../types/project").Layer[] = [];
    function collect(layers: import("../types/project").Layer[]) {
      for (const l of layers) {
        if (l.type === "map" && l.visible !== false) result.push(l);
        if (l.children) collect(l.children);
      }
    }
    collect(project.layers);
    return result;
  });

  let spaceHeld = $state(false);

  // OSD overlay state
  let osdText = $state("");
  let osdValue = $state(0); // 0-100 for progress bar
  let osdShowTime = $state(0);
  const OSD_DURATION = 1800; // ms visible
  const OSD_FADE = 400; // ms fade out

  function showOsd(label: string, value: number) {
    osdText = label;
    osdValue = Math.max(0, Math.min(100, value));
    osdShowTime = performance.now();
    markDirty();
  }

  function drawOsd(ctx: CanvasRenderingContext2D, timestamp: number) {
    if (!osdShowTime) return;
    const elapsed = timestamp - osdShowTime;
    if (elapsed > OSD_DURATION + OSD_FADE) { osdShowTime = 0; return; }

    let alpha = 1;
    if (elapsed > OSD_DURATION) {
      alpha = 1 - (elapsed - OSD_DURATION) / OSD_FADE;
    }

    ctx.save();
    ctx.setTransform(1, 0, 0, 1, 0, 0);

    const w = 220, h = 44, r = 12;
    const x = (canvas.width - w) / 2;
    const y = canvas.height - 80;

    // Background pill
    ctx.globalAlpha = alpha * 0.85;
    ctx.fillStyle = "#1a1a2e";
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, r);
    ctx.fill();
    ctx.strokeStyle = "rgba(255,255,255,0.12)";
    ctx.lineWidth = 1;
    ctx.stroke();

    // Icon + text
    ctx.globalAlpha = alpha;
    ctx.fillStyle = "#ffffff";
    ctx.font = "16px sans-serif";
    ctx.textBaseline = "middle";
    ctx.fillText(osdText, x + 14, y + h / 2);

    // Progress bar track
    const barX = x + 14, barY = y + h - 10, barW = w - 28, barH = 4;
    ctx.fillStyle = "rgba(255,255,255,0.15)";
    ctx.beginPath();
    ctx.roundRect(barX, barY, barW, barH, 2);
    ctx.fill();

    // Progress bar fill
    ctx.fillStyle = "#5599ff";
    const fillW = barW * (osdValue / 100);
    if (fillW > 0) {
      ctx.beginPath();
      ctx.roundRect(barX, barY, fillW, barH, 2);
      ctx.fill();
    }

    ctx.restore();
  }

  const HANDLE_HIT_RADIUS = 10;
  const HANDLE_PAD = 2;
  let rafId: number;

  onMount(() => {
    ctx = canvas.getContext("2d")!;
    updateCanvasSize();
    loadBundledIconFonts();
    rafId = requestAnimationFrame(renderLoop);

    // Set up wake callback for render scheduler (wallpaper mode)
    if (isWallpaperView) {
      setWakeCallback(() => {
        rafId = requestAnimationFrame(renderLoop);
      });
    }

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

    let newStyleWidth: number;
    let newStyleHeight: number;

    if (fullscreen) {
      canvas.width = project.resolution.width;
      canvas.height = project.resolution.height;
      newStyleWidth = containerRect.width;
      newStyleHeight = containerRect.height;
      baseScale = containerRect.width / project.resolution.width;
    } else {
      const scaleX = (containerRect.width - 40) / project.resolution.width;
      const scaleY = (containerRect.height - 40) / project.resolution.height;
      baseScale = Math.min(scaleX, scaleY, 1);

      canvas.width = project.resolution.width;
      canvas.height = project.resolution.height;

      newStyleWidth = Math.round(project.resolution.width * baseScale);
      newStyleHeight = Math.round(project.resolution.height * baseScale);
    }

    canvasStyleWidth = newStyleWidth;
    canvasStyleHeight = newStyleHeight;

    // Compute canvas offset within container directly from known dimensions.
    // The canvas is centered by flexbox, so this is exact and avoids the
    // getBoundingClientRect() staleness bug (Svelte flushes DOM updates
    // asynchronously, so the canvas still has its old CSS size at this point).
    canvasOffsetX = (containerRect.width - newStyleWidth) / 2;
    canvasOffsetY = (containerRect.height - newStyleHeight) / 2;
  }

  function renderLoop(timestamp: number) {
    if (!ctx) {
      rafId = requestAnimationFrame(renderLoop);
      return;
    }

    // Wallpaper mode: decide whether to render or skip this frame
    if (isWallpaperView) {
      const looping = hasActiveLoopingAnimations(getProject().layers) || hasAnimatedGifs();
      const fullFps = shouldRenderFullFps(timestamp, looping);

      if (fullFps) {
        // Active mode (user interaction, animations): render + schedule at 60fps
        clearRepaint();
        doRender(timestamp);
        rafId = requestAnimationFrame(renderLoop);
      } else if (needsRepaint()) {
        // Idle mode but content changed (clock tick, provider update): render once
        clearRepaint();
        doRender(timestamp);
        const timeoutId = setTimeout(() => {
          rafId = requestAnimationFrame(renderLoop);
        }, IDLE_INTERVAL_MS);
        setIdleTimeout(timeoutId);
      } else {
        // Idle mode, nothing changed: skip render entirely
        const timeoutId = setTimeout(() => {
          rafId = requestAnimationFrame(renderLoop);
        }, IDLE_INTERVAL_MS);
        setIdleTimeout(timeoutId);
      }
    } else {
      // Editor mode: always render at 60fps
      doRender(timestamp);
      rafId = requestAnimationFrame(renderLoop);
    }
  }

  function doRender(timestamp: number) {
    const project = getProject();
    const selectedId = fullscreen ? null : getSelectedLayerId();

    if (!fullscreen && (zoom !== 1 || panX !== 0 || panY !== 0)) {
      ctx.save();
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.fillStyle = "#0d0d1a";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.restore();

      ctx.save();
      ctx.translate(panX, panY);
      ctx.scale(zoom, zoom);
      renderProject(ctx, project, selectedId, timestamp, hoveredLayerId);
      ctx.restore();
    } else {
      renderProject(ctx, project, selectedId, timestamp, hoveredLayerId);
    }

    drawOsd(ctx, timestamp);
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
    // Check for scroll actions on hovered layer (works in any mode)
    const id = hitTest(e.clientX, e.clientY);
    if (id) {
      const project = getProject();
      const action = findScrollActionForHit(project.layers, id);
      if (action) {
        e.preventDefault();
        handleScrollAction(action, e.deltaY);
        return;
      }
    }
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
    // Close start menu when clicking the canvas (the HTML overlay handles its own backdrop clicks)
    if (startMenuVisible) {
      startMenuVisible = false;
      closeStartMenu();
      markDirty();
    }
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
      // Trigger tap on hit layer and all ancestors (so parent tap animations fire too)
      const now = performance.now();
      const project = getProject();
      const path = findPathToLayer(project.layers, id);
      if (path) {
        for (const layer of path) triggerTap(layer.id, now);
      } else {
        triggerTap(id, now);
      }
      if (getInteractiveMode()) {
        const project = getProject();
        const action = findClickActionForHit(project.layers, id, e.clientX, e.clientY);
        if (action) {
          handleClickAction(action, project, id);
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
    // Pass hover coords to launcher renderer for highlight drawing
    const hproj = canvasToProject(e.clientX, e.clientY);
    setLauncherHoverCoords(hproj.x, hproj.y);
  }

  function onMouseUp() {
    dragMode = "none";
    resizeHandleIdx = -1;
  }

  function onFullscreenMouseMove(e: MouseEvent) {
    markDirty();
    setScrollPosition(e.clientX / window.innerWidth);

    // Track hovered layer for hover animations
    const hit = hitTest(e.clientX, e.clientY);
    if (hit !== hoveredLayerId) {
      hoveredLayerId = hit;
    }
    const hproj2 = canvasToProject(e.clientX, e.clientY);
    setLauncherHoverCoords(hproj2.x, hproj2.y);
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
  function findClickActionForHit(layers: Layer[], hitId: string, clientX?: number, clientY?: number): string | null {
    const path = findPathToLayer(layers, hitId);
    if (!path) return null;
    // Check launcher sub-regions first
    const hitLayer = path[path.length - 1];
    if (hitLayer?.type === "launcher" && clientX !== undefined && clientY !== undefined) {
      const proj = canvasToProject(clientX, clientY);
      const regions = launcherHitRegions.get(hitLayer.id) ?? [];
      for (const r of regions) {
        if (proj.x >= r.bx && proj.x <= r.bx + r.bw && proj.y >= r.by && proj.y <= r.by + r.bh) {
          return `app:${r.exec}`;
        }
      }
    }
    for (let i = path.length - 1; i >= 0; i--) {
      if (path[i].properties.clickAction) return path[i].properties.clickAction!;
    }
    return null;
  }

  /** Find the nearest scrollAction walking from hit layer up to root */
  function findScrollActionForHit(layers: Layer[], hitId: string): string | null {
    const path = findPathToLayer(layers, hitId);
    if (!path) return null;
    for (let i = path.length - 1; i >= 0; i--) {
      if (path[i].properties.scrollAction) return path[i].properties.scrollAction!;
    }
    return null;
  }

  function onFullscreenClick(e: MouseEvent) {
    markDirty();
    if (startMenuVisible) {
      startMenuVisible = false;
      closeStartMenu();
      markDirty();
    }
    const id = hitTest(e.clientX, e.clientY);
    if (id) {
      const now = performance.now();
      const project = getProject();
      const path = findPathToLayer(project.layers, id);
      if (path) {
        for (const layer of path) triggerTap(layer.id, now);
      } else {
        triggerTap(id, now);
      }
      const action = findClickActionForHit(project.layers, id, e.clientX, e.clientY);
      if (action) {
        handleClickAction(action, project, id);
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

  async function handleClickAction(action: string, project: ReturnType<typeof getProject>, triggerId?: string) {
    if (action === "app:overlay:start_menu") {
      startMenuVisible = !startMenuVisible;
      if (triggerId) {
        const bounds = getLayerBounds().get(triggerId);
        startMenuAnchorBounds = bounds ? { x: bounds.x, y: bounds.y, w: bounds.w, h: bounds.h } : null;
      }
      toggleStartMenu();
      const wk = (window as any).webkit?.messageHandlers?.lava;
      if (wk) wk.postMessage(JSON.stringify({ type: startMenuVisible ? "start_menu_open" : "start_menu_close" }));
      markDirty();
      return;
    }
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
        const isTauriCtx = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";
        if (isTauriCtx) {
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
      const isTauriCtx = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";
      if (isTauriCtx) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("music_control", { action: musicAction });
        } catch (e) {
          console.warn("Music control failed:", e);
        }
      } else {
        (window as any).webkit?.messageHandlers?.lava?.postMessage(
          JSON.stringify({ type: "music_control", action: musicAction })
        );
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
        (window as any).webkit?.messageHandlers?.lava?.postMessage(
          JSON.stringify({ type: "launch_app", command: cmd })
        );
      }
    } else if (action === "editor:show") {
      // editor:show — bring the editor window back into focus
      const isTauriCtx = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";
      if (isTauriCtx) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("show_editor");
        } catch (e) {
          console.warn("Show editor failed:", e);
        }
      } else {
        (window as any).webkit?.messageHandlers?.lava?.postMessage(
          JSON.stringify({ type: "show_editor" })
        );
      }
    } else if (action === "search" || action === "search:") {
      // Open inline search overlay at the triggering layer's bounds
      if (triggerId) {
        const bounds = getLayerBounds().get(triggerId);
        if (bounds) {
          searchOverlayBounds = { x: bounds.x, y: bounds.y, w: bounds.w, h: bounds.h };
          searchOverlayOpen = true;
        }
      }
      return;
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

  const searchUrls: Record<string, string> = {
    google: "https://www.google.com/search?q=",
    perplexity: "https://www.perplexity.ai/search?q=",
    bing: "https://www.bing.com/search?q=",
    chatgpt: "https://chatgpt.com/?q=",
    duckduckgo: "https://duckduckgo.com/?q=",
  };

  function doWebSearch(query: string) {
    const engine = getSettings().searchEngine || "google";
    const url = (searchUrls[engine] || searchUrls.google) + encodeURIComponent(query);
    openUrl(url);
  }

  function openSearchOverlay() {
    if (!searchOverlayBounds || !searchOverlayOpen || !canvas) return;
    // Remove existing portal if any
    if (searchOverlayEl) { searchOverlayEl.remove(); searchOverlayEl = undefined; }

    const b = searchOverlayBounds;
    const rect = canvas.getBoundingClientRect();
    const scaleX = rect.width / canvas.width;
    const scaleY = rect.height / canvas.height;
    // Project coords → screen coords via canvas transform
    const left = rect.left + (b.x * zoom + panX) * scaleX;
    const top = rect.top + (b.y * zoom + panY) * scaleY;
    const width = b.w * zoom * scaleX;
    const height = b.h * zoom * scaleY;

    const portal = document.createElement("div");
    portal.style.cssText = `position:fixed;left:${left}px;top:${top}px;width:${width}px;height:${height}px;z-index:99999;display:flex;align-items:center;`;

    let searchText = "";
    const fontSize = Math.max(12, height * 0.45);
    const pad = height * 0.4;
    const radius = Math.max(4, height * 0.2);

    // Use a div instead of input — WebKitGTK layer-shell doesn't render input text
    const box = document.createElement("div");
    box.style.cssText = `position:relative;z-index:99999;width:100%;height:100%;border:none;outline:2px solid #60cdff;border-radius:${radius}px;padding:0 ${pad}px;background:#181825;color:#cdd6f4;font-size:${fontSize}px;box-sizing:border-box;display:flex;align-items:center;cursor:text;overflow:hidden;white-space:nowrap;`;

    const textSpan = document.createElement("span");
    textSpan.style.cssText = "color:#cdd6f4;";
    const cursor = document.createElement("span");
    cursor.style.cssText = "display:inline-block;width:1px;height:1.1em;background:#cdd6f4;animation:blink 1s step-end infinite;margin-left:1px;";
    const placeholder = document.createElement("span");
    placeholder.style.cssText = "color:rgba(255,255,255,0.35);";
    placeholder.textContent = "Search the web...";

    // Add blink animation
    const style = document.createElement("style");
    style.textContent = "@keyframes blink{0%,100%{opacity:1}50%{opacity:0}}";
    portal.appendChild(style);

    box.appendChild(textSpan);
    box.appendChild(cursor);
    box.appendChild(placeholder);

    function updateDisplay() {
      textSpan.textContent = searchText;
      placeholder.style.display = searchText ? "none" : "";
    }

    // Fake input element interface for compatibility
    const input = { get value() { return searchText; }, set value(v: string) { searchText = v; updateDisplay(); } };

    function closeSearch() {
      searchOverlayOpen = false;
      if ((portal as any)._restoreLavaKey) (portal as any)._restoreLavaKey();
      portal.remove();
      searchOverlayEl = undefined;
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onOutsideClick);
      const wk2 = (window as any).webkit?.messageHandlers?.lava;
      if (wk2) wk2.postMessage(JSON.stringify({ type: "start_menu_close" }));
    }

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        const q = searchText.trim();
        if (q) doWebSearch(q);
        closeSearch();
        return;
      }
      if (e.key === "Escape") { closeSearch(); return; }
      if (e.key === "Backspace") { e.preventDefault(); searchText = searchText.slice(0, -1); updateDisplay(); return; }
      if (e.key === "Delete") { e.preventDefault(); searchText = ""; updateDisplay(); return; }
      if (e.key.length === 1 && !e.ctrlKey && !e.altKey && !e.metaKey) {
        e.preventDefault();
        searchText += e.key;
        updateDisplay();
      }
    };

    const onOutsideClick = (e: MouseEvent) => {
      if (!portal.contains(e.target as Node)) closeSearch();
    };
    setTimeout(() => document.addEventListener("mousedown", onOutsideClick), 50);

    portal.appendChild(box);
    document.body.appendChild(portal);
    searchOverlayEl = portal;

    // Request keyboard focus in wallpaper mode
    const wk = (window as any).webkit?.messageHandlers?.lava;
    if (wk) wk.postMessage(JSON.stringify({ type: "start_menu_open" }));

    document.addEventListener("keydown", onKey);

    // Hijack __lavaKey so Rust GTK key injection goes to the search box
    const originalLavaKey = (window as any).__lavaKey;
    (window as any).__lavaKey = (key: string) => {
      onKey(new KeyboardEvent("keydown", { key }));
    };
    // Store restore function for closeSearch
    (portal as any)._restoreLavaKey = () => {
      if (originalLavaKey) (window as any).__lavaKey = originalLavaKey;
      else delete (window as any).__lavaKey;
    };
  }

  $effect(() => {
    if (searchOverlayOpen && searchOverlayBounds) {
      // Small delay to let state settle
      setTimeout(openSearchOverlay, 10);
    }
  });

  let estimatedVolume = 50;
  let estimatedBrightness = 50;
  let lastScrollActionTime = 0;
  const SCROLL_THROTTLE = 80; // ms between scroll action fires

  function handleScrollAction(action: string, deltaY: number) {
    const now = performance.now();
    if (now - lastScrollActionTime < SCROLL_THROTTLE) return;
    lastScrollActionTime = now;
    const step = deltaY > 0 ? -2 : 2;

    if (action === "volume:adjust") {
      estimatedVolume = Math.max(0, Math.min(100, estimatedVolume + step));
      showOsd(`\u{1F50A} ${estimatedVolume}%`, estimatedVolume);
      // Fire and forget — don't block the UI
      if (isWallpaperView && (window as any).webkit?.messageHandlers?.lava) {
        (window as any).webkit.messageHandlers.lava.postMessage(
          JSON.stringify({ type: "adjust_volume", delta: String(step) })
        );
      } else {
        import("@tauri-apps/api/core").then(({ invoke }) =>
          invoke("adjust_volume", { delta: step })
        ).catch(() => {});
      }
    } else if (action === "brightness:adjust") {
      estimatedBrightness = Math.max(0, Math.min(100, estimatedBrightness + step));
      showOsd(`\u{2600} ${estimatedBrightness}%`, estimatedBrightness);
      const cmd = `brightnessctl set ${step > 0 ? `+${step}%` : `${-step}%-`}`;
      if (isWallpaperView && (window as any).webkit?.messageHandlers?.lava) {
        (window as any).webkit.messageHandlers.lava.postMessage(
          JSON.stringify({ type: "launch_command", command: cmd })
        );
      } else {
        import("@tauri-apps/api/core").then(({ invoke }) =>
          invoke("launch_app", { command: cmd })
        ).catch(() => {});
      }
    }
  }

  function onFullscreenWheel(e: WheelEvent) {
    markDirty();
    const id = hitTest(e.clientX, e.clientY);
    if (!id) return;
    const project = getProject();
    const action = findScrollActionForHit(project.layers, id);
    if (action) {
      e.preventDefault();
      handleScrollAction(action, e.deltaY);
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
    onwheel={fullscreen ? onFullscreenWheel : onWheel}
  />
  {#each mapLayers as mapLayer (mapLayer.id)}
    <MapOverlay
      layer={mapLayer}
      {baseScale}
      {zoom}
      {panX}
      {panY}
      {canvasOffsetX}
      {canvasOffsetY}
      containerW={getProject().resolution.width}
      containerH={getProject().resolution.height}
      interactive={isWallpaperView || fullscreen}
    />
  {/each}
  <StartMenuOverlay
    anchorBounds={startMenuAnchorBounds}
    {baseScale}
    {zoom}
    {panX}
    {panY}
    {canvasOffsetX}
    {canvasOffsetY}
    containerW={getProject().resolution.width}
    containerH={getProject().resolution.height}
    interactive={isWallpaperView || fullscreen}
    visible={startMenuVisible}
    smBg="#1c1c1c"
    smAccent="#60cdff"
    onclose={() => {
      startMenuVisible = false;
      closeStartMenu();
      const wk = (window as any).webkit?.messageHandlers?.lava;
      if (wk) wk.postMessage(JSON.stringify({ type: "start_menu_close" }));
      markDirty();
    }}
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
