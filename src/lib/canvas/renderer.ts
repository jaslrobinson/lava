import type { Project, Layer } from "../types/project";
import type { PaintStroke } from "../types/paint";
import { resolveFormula, hasFormula } from "../formula/service";
import { computeAnimatedDeltas } from "./animationEngine";
import { initEngineTime, markLayerSeen, beginFrame, updateHoverState } from "./animationState";
import { getAudioBands, getAudioPeaks, initAudioVisualizer } from "./audioVisualizer";
import { renderPaintStrokes } from "./paintEngine";

// Detect if we're running inside Tauri (vs plain WebKitGTK wallpaper view)
const isTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";
let convertFileSrc: ((path: string) => string) | null = null;
if (isTauri) {
  import("@tauri-apps/api/core").then(m => { convertFileSrc = m.convertFileSrc; });
}

/** Resolve a property value that might be a formula */
function resolve(value: string | number | undefined, fallback: string = ""): string {
  if (value === undefined || value === null) return fallback;
  const str = String(value);
  if (hasFormula(str)) return resolveFormula(str);
  return str;
}

/** Resolve a property value as a number, evaluating any formula first */
function resolveNumber(value: string | number | undefined, fallback: number = 0): number {
  if (value === undefined || value === null) return fallback;
  // If it's already a plain number, fast path
  if (typeof value === "number") return isNaN(value) ? fallback : value;
  const str = String(value);
  if (!hasFormula(str)) {
    const n = Number(str);
    return isNaN(n) ? fallback : n;
  }
  const resolved = resolveFormula(str);
  // Loading placeholder or error — use fallback
  if (resolved === "\u2026" || resolved === "[err]") return fallback;
  const n = Number(resolved);
  return isNaN(n) ? fallback : n;
}

/** Safely append alpha to a color string (handles #RGB, #RRGGBB; non-hex passes through) */
function withAlpha(color: string, alpha: number): string {
  const a = Math.round(alpha * 255).toString(16).padStart(2, '0');
  if (color.match(/^#[0-9a-fA-F]{6}$/)) return color + a;
  if (color.match(/^#[0-9a-fA-F]{3}$/)) {
    const r = color[1], g = color[2], b = color[3];
    return `#${r}${r}${g}${g}${b}${b}${a}`;
  }
  return color;
}

// Detect whether CanvasRenderingContext2D.filter actually works (not just accepted silently).
// We draw a red pixel, apply an invert filter, and check if it changed to cyan.
let _canvasFilterSupported: boolean | null = null;
function canvasFilterWorks(): boolean {
  if (_canvasFilterSupported !== null) return _canvasFilterSupported;
  try {
    const c = document.createElement("canvas");
    c.width = 1; c.height = 1;
    const cx = c.getContext("2d")!;
    cx.fillStyle = "#ff0000";
    cx.fillRect(0, 0, 1, 1);
    cx.filter = "invert(1)";
    cx.drawImage(c, 0, 0);
    cx.filter = "none";
    const p = cx.getImageData(0, 0, 1, 1).data;
    // After invert, red (255,0,0) should become cyan (0,255,255)
    _canvasFilterSupported = p[0] < 128 && p[1] > 128 && p[2] > 128;
  } catch {
    _canvasFilterSupported = false;
  }
  return _canvasFilterSupported;
}

// Debug overlay: shows bounds, transform info, and position markers on each layer
let debugOverlay = false;
export function setDebugOverlay(enabled: boolean) { debugOverlay = enabled; }
export function getDebugOverlay(): boolean { return debugOverlay; }

// Reusable offscreen canvas to avoid allocating a new one every frame
let _reusableOffscreen: HTMLCanvasElement | null = null;
function getReusableCanvas(w: number, h: number): HTMLCanvasElement {
  if (!_reusableOffscreen) _reusableOffscreen = document.createElement("canvas");
  _reusableOffscreen.width = w;
  _reusableOffscreen.height = h;
  return _reusableOffscreen;
}

// Track whether any visible GIF images were rendered this frame (need full-fps for animation)
let hasGifImages = false;
export function hasAnimatedGifs(): boolean { return hasGifImages; }

// Image cache to avoid reloading every frame
const imageCache = new Map<string, HTMLImageElement>();
const imageLoadingSet = new Set<string>();
const imageFailedMap = new Map<string, number>(); // URL -> failure timestamp
const IMAGE_RETRY_MS = 5000; // Retry failed images after 5 seconds
const MAX_IMAGE_CACHE = 200;

/** Clear the image caches so re-importing a preset can retry previously failed images */
export function clearImageCache() {
  imageCache.clear();
  imageLoadingSet.clear();
  imageFailedMap.clear();
}

// Current asset directory for resolving kfile:// and relative paths
let currentAssetDir: string | undefined;

function resolveImagePath(src: string): string | null {
  if (!src) return null;

  // Skip unresolved KLWP formulas (e.g. "$gv(...)$")
  if (src.includes("$") && src.includes("(")) return null;

  // Resolve kfile:// paths using asset directory
  if (src.startsWith("kfile://")) {
    if (!currentAssetDir) return null;
    const relative = src.replace("kfile://org.kustom.provider/", "");
    return `${currentAssetDir}/${relative}`;
  }

  // Handle file:// URLs (e.g. MPRIS cover art like file:///tmp/album.png)
  if (src.startsWith("file://")) {
    return src.replace("file://", "");
  }

  return src;
}

function getCachedImage(src: string): HTMLImageElement | null {
  const resolvedPath = resolveImagePath(src);
  if (!resolvedPath) return null;

  let resolved: string;
  if (resolvedPath.startsWith("http") || resolvedPath.startsWith("data:")) {
    resolved = resolvedPath;
  } else if (!isTauri || import.meta.env.DEV) {
    // Wallpaper WebKitGTK OR Tauri dev mode: Vite dev server serves any local file
    resolved = `/__lava_assets${resolvedPath}`;
  } else if (convertFileSrc) {
    // Production Tauri: use asset:// protocol
    resolved = convertFileSrc(resolvedPath);
  } else {
    // Tauri but convertFileSrc not loaded yet — wait rather than failing
    return null;
  }

  // Check if previously failed — retry after cooldown
  const failedAt = imageFailedMap.get(resolved);
  if (failedAt !== undefined) {
    if (Date.now() - failedAt < IMAGE_RETRY_MS) return null;
    imageFailedMap.delete(resolved); // Allow retry
  }

  if (imageCache.has(resolved)) {
    const img = imageCache.get(resolved)!;
    return img.complete && img.naturalWidth > 0 ? img : null;
  }

  if (imageLoadingSet.has(resolved)) return null;

  imageLoadingSet.add(resolved);
  const img = new Image();
  // Only set crossOrigin for Tauri asset:// URLs, not http/proxy/external URLs
  if (!resolved.startsWith("http") && !resolved.startsWith("/__lava_assets")) {
    img.crossOrigin = "anonymous";
  }
  img.onload = () => {
    imageCache.set(resolved, img);
    imageLoadingSet.delete(resolved);
    // Prune oldest entries if cache is too large
    if (imageCache.size > MAX_IMAGE_CACHE) {
      const first = imageCache.keys().next().value;
      if (first) imageCache.delete(first);
    }
  };
  img.onerror = () => {
    imageLoadingSet.delete(resolved);
    imageFailedMap.set(resolved, Date.now());
  };
  img.src = resolved;
  return null;
}

/** Container dimensions for anchor-based positioning */
interface ContainerSize {
  width: number;
  height: number;
}

/** Absolute bounds for a rendered layer */
export interface LayerBounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

// Map of layer ID -> absolute bounds, rebuilt each frame
let computedBounds = new Map<string, LayerBounds>();

/** Get the computed absolute bounds for all layers (updated each frame) */
export function getLayerBounds(): Map<string, LayerBounds> {
  return computedBounds;
}

// Base transform at start of renderProject — used to derive absolute project-space
// positions from the canvas transform matrix (accounts for zoom/pan).
let baseTransform: DOMMatrix | null = null;

/** Walk layer tree to collect the hovered layer ID + all ancestor IDs */
function collectAncestors(targetId: string, layers: Layer[], ancestors: Set<string>): boolean {
  for (const layer of layers) {
    if (layer.id === targetId) return true;
    if (layer.children?.length) {
      if (collectAncestors(targetId, layer.children, ancestors)) {
        ancestors.add(layer.id);
        return true;
      }
    }
  }
  return false;
}

function getHoveredIds(hoveredLayerId: string | null, layers: Layer[]): Set<string> {
  if (!hoveredLayerId) return new Set();
  const ids = new Set<string>([hoveredLayerId]);
  collectAncestors(hoveredLayerId, layers, ids);
  return ids;
}

export function renderProject(ctx: CanvasRenderingContext2D, project: Project, selectedId: string | null, timestamp: number = 0, hoveredLayerId: string | null = null) {
  hasGifImages = false;
  beginFrame();
  initEngineTime(timestamp);
  updateHoverState(getHoveredIds(hoveredLayerId, project.layers), timestamp);
  const { width, height } = project.resolution;
  const container: ContainerSize = { width, height };

  initAudioVisualizer();
  currentAssetDir = project.assetDir;
  computedBounds = new Map();
  baseTransform = ctx.getTransform();

  ctx.clearRect(0, 0, width, height);
  if (project.background.type === "color") {
    ctx.fillStyle = project.background.value;
    ctx.fillRect(0, 0, width, height);
  } else if (project.background.type === "image" && project.background.value) {
    const bgImg = getCachedImage(project.background.value);
    if (bgImg) {
      // Fill the canvas, cropping to maintain aspect ratio
      const scale = Math.max(width / bgImg.naturalWidth, height / bgImg.naturalHeight);
      const drawW = bgImg.naturalWidth * scale;
      const drawH = bgImg.naturalHeight * scale;
      ctx.drawImage(bgImg, (width - drawW) / 2, (height - drawH) / 2, drawW, drawH);
    } else {
      // Fallback while loading
      ctx.fillStyle = "#1a1a2e";
      ctx.fillRect(0, 0, width, height);
    }
  }

  for (const layer of project.layers) {
    if (!isLayerVisible(layer)) continue;
    renderLayer(ctx, layer, container, 0, 0, timestamp);
  }

  if (selectedId) {
    const bounds = computedBounds.get(selectedId);
    if (bounds) drawSelectionOutline(ctx, bounds);
  }
}

/** Check if a layer should be visible based on its visibility property */
function isLayerVisible(layer: Layer): boolean {
  if (layer.visible === false) return false;
  const vis = layer.properties.visible;
  if (vis === undefined || vis === true) return true;
  if (vis === false) return false;
  // Formula-driven visibility: "ALWAYS", "NEVER", "REMOVE", or a formula
  const resolved = resolve(vis as any, "always").trim().toLowerCase();
  if (resolved === "never" || resolved === "remove" || resolved === "0" || resolved === "false") return false;
  return true;
}

/** Calculate anchor-adjusted position within a container */
function anchorPosition(
  offsetX: number,
  offsetY: number,
  itemW: number,
  itemH: number,
  anchor: string | undefined,
  container: ContainerSize,
): { x: number; y: number } {
  let x = offsetX;
  let y = offsetY;

  switch (anchor) {
    case "center":
      x = container.width / 2 + offsetX - itemW / 2;
      y = container.height / 2 + offsetY - itemH / 2;
      break;
    case "top-center":
      x = container.width / 2 + offsetX - itemW / 2;
      y = offsetY;
      break;
    case "top-right":
      x = container.width + offsetX - itemW;
      y = offsetY;
      break;
    case "center-left":
      x = offsetX;
      y = container.height / 2 + offsetY - itemH / 2;
      break;
    case "center-right":
      x = container.width + offsetX - itemW;
      y = container.height / 2 + offsetY - itemH / 2;
      break;
    case "bottom-left":
      x = offsetX;
      y = container.height + offsetY - itemH;
      break;
    case "bottom-center":
      x = container.width / 2 + offsetX - itemW / 2;
      y = container.height + offsetY - itemH;
      break;
    case "bottom-right":
      x = container.width + offsetX - itemW;
      y = container.height + offsetY - itemH;
      break;
    // "top-left" or undefined: use raw offset
  }

  return { x, y };
}

function renderLayer(ctx: CanvasRenderingContext2D, layer: Layer, container: ContainerSize, parentAbsX: number, parentAbsY: number, timestamp: number = 0) {
  if (!isLayerVisible(layer)) return;
  markLayerSeen(layer.id, timestamp);
  ctx.save();

  const props = layer.properties;
  const deltas = computeAnimatedDeltas(layer, timestamp);

  const offsetX = resolveNumber(props.x, 0) + deltas.dx;
  const offsetY = resolveNumber(props.y, 0) + deltas.dy;
  const w = resolveNumber(props.width, 0);
  const h = resolveNumber(props.height, 0);
  const opacity = deltas.opacityOverride !== null
    ? (deltas.opacityOverride / 255) * deltas.opacityMultiplier
    : (resolveNumber(props.opacity, 255) / 255) * deltas.opacityMultiplier;
  const rotation = resolveNumber(props.rotation, 0) + deltas.dRotation;

  // Anchor-based positioning (local to container)
  const { x, y } = anchorPosition(offsetX, offsetY, w, h, props.anchor, container);

  // Compute absolute project-space position from the canvas transform matrix.
  // This is more reliable than manually accumulating parentAbsX because the canvas
  // context naturally tracks all parent translations, rotations, and scales.
  let absX: number, absY: number;
  if (baseTransform && baseTransform.a !== 0) {
    const ct = ctx.getTransform();
    absX = (ct.e + x * ct.a + y * ct.c - baseTransform.e) / baseTransform.a;
    absY = (ct.f + x * ct.b + y * ct.d - baseTransform.f) / baseTransform.d;
  } else {
    absX = parentAbsX + x;
    absY = parentAbsY + y;
  }
  computedBounds.set(layer.id, { x: absX, y: absY, w: w || 100, h: h || 100 });

  ctx.globalAlpha *= opacity;

  // Compute total blur from animations + FX property
  const fxBlurVal = resolveNumber(props.fxBlur, 0);
  const totalBlur = deltas.blur + fxBlurVal;

  // Apply blend mode — defer until just before compositing if using offscreen
  const blendMode = props.blendMode;

  const cx = x + w / 2;
  const cy = y + h / 2;

  const scaleX = resolveNumber(props.scaleX, 1) * deltas.scaleX;
  const scaleY = resolveNumber(props.scaleY, 1) * deltas.scaleY;

  if (rotation || scaleX !== 1 || scaleY !== 1) {
    ctx.translate(cx, cy);
    if (rotation) ctx.rotate((rotation * Math.PI) / 180);
    if (scaleX !== 1 || scaleY !== 1) ctx.scale(scaleX, scaleY);
    ctx.translate(-cx, -cy);
  }

  // Only use offscreen for blur when there's no blend mode conflict
  const hasBlend = !!(blendMode && blendMode !== "source-over");
  const blurNeedsOffscreen = totalBlur > 0 && !hasBlend;
  const needsOffscreen = deltas.colorOverride || deltas.flashOverlay > 0 || blurNeedsOffscreen;

  // For color/flash overlays, render to a temporary canvas so source-atop
  // only affects this layer's pixels, not previously drawn content.
  // Uses HTMLCanvasElement instead of OffscreenCanvas for WebKitGTK compatibility.
  // Apply blend mode now if rendering directly (no offscreen)
  if (!needsOffscreen && blendMode && blendMode !== "source-over") {
    ctx.globalCompositeOperation = blendMode as GlobalCompositeOperation;
  }

  let drawCtx = ctx;
  let offscreen: HTMLCanvasElement | null = null;
  if (needsOffscreen) {
    const margin = Math.ceil(totalBlur) + 2;
    const ow = (w || 100) + margin * 2;
    const oh = (h || 100) + margin * 2;
    offscreen = getReusableCanvas(ow, oh);
    drawCtx = offscreen.getContext("2d")!;
    // Copy font from main context so text renders correctly on the offscreen canvas
    drawCtx.font = ctx.font;
    // Shift drawing so layer content is at (margin, margin) on the offscreen canvas
    drawCtx.translate(margin - x, margin - y);
  }

  switch (layer.type) {
    case "text":
      renderText(drawCtx, layer, x, y, w, h);
      break;
    case "shape":
      renderShape(drawCtx, layer, x, y, w, h);
      break;
    case "image":
      renderImage(drawCtx, layer, x, y, w, h);
      break;
    case "progress":
      renderProgress(drawCtx, layer, x, y, w, h);
      break;
    case "overlap":
      renderOverlap(drawCtx, layer, x, y, container, parentAbsX, parentAbsY, timestamp);
      break;
    case "stack":
      renderStack(drawCtx, layer, x, y, container, parentAbsX, parentAbsY, timestamp);
      break;
    case "fonticon":
      renderFontIcon(drawCtx, layer, x, y, w, h);
      break;
    case "visualizer":
      renderVisualizer(drawCtx, layer, x, y, w, h);
      break;
    case "map":
      renderMapPlaceholder(drawCtx, layer, x, y, w, h);
      break;
    case "launcher":
      renderLauncherLayer(drawCtx, layer, x, y, w, h);
      break;
    case "paint":
      renderPaintLayer(drawCtx, layer, x, y, w, h);
      break;
  }

  // Blur + blend mode combo: render to temp canvas, then multi-draw with offsets
  // on the main ctx which has blend mode active. Each offset draw blends properly.
  if (!offscreen && totalBlur > 0 && hasBlend) {
    // Capture what was just drawn by rendering again to a temp canvas
    const lw = w || 100;
    const lh = h || 100;
    const tmpC = document.createElement("canvas");
    tmpC.width = Math.ceil(lw);
    tmpC.height = Math.ceil(lh);
    const tmpX = tmpC.getContext("2d")!;
    tmpX.translate(-x, -y);
    // Re-render the layer onto the temp canvas
    switch (layer.type) {
      case "text": renderText(tmpX, layer, x, y, lw, lh); break;
      case "shape": renderShape(tmpX, layer, x, y, lw, lh); break;
      case "image": renderImage(tmpX, layer, x, y, lw, lh); break;
      case "progress": renderProgress(tmpX, layer, x, y, lw, lh); break;
      case "fonticon": renderFontIcon(tmpX, layer, x, y, lw, lh); break;
      case "visualizer": renderVisualizer(tmpX, layer, x, y, lw, lh); break;
      case "paint": renderPaintLayer(tmpX, layer, x, y, lw, lh); break;
    }
    // Now overdraw on main ctx at offsets with reduced alpha
    const radius = Math.min(totalBlur, 50);
    const rings = Math.max(2, Math.ceil(radius / 4));
    let sampleCount = 0;
    const offsets: [number, number][] = [];
    for (let ring = 1; ring <= rings; ring++) {
      const r = (ring / rings) * radius;
      const count = Math.max(4, Math.ceil(ring * 4));
      for (let j = 0; j < count; j++) {
        const angle = (j / count) * Math.PI * 2;
        offsets.push([Math.cos(angle) * r, Math.sin(angle) * r]);
        sampleCount++;
      }
    }
    const alpha = 1 / (sampleCount + 1);
    // Clear the original sharp draw first
    ctx.save();
    ctx.globalCompositeOperation = "destination-out";
    ctx.globalAlpha = 1;
    ctx.fillRect(x, y, lw, lh);
    ctx.restore();
    // Redraw blurred with blend mode
    ctx.globalAlpha *= alpha;
    ctx.drawImage(tmpC, x, y);
    for (const [dx, dy] of offsets) {
      ctx.drawImage(tmpC, x + dx, y + dy);
    }
  }

  if (offscreen) {
    const margin = Math.ceil(totalBlur) + 2;

    // Reset transform so fillRect covers the entire offscreen canvas
    drawCtx.setTransform(1, 0, 0, 1, 0, 0);

    // Color overlay on the offscreen canvas (source-atop only hits this layer's pixels)
    if (deltas.colorOverride) {
      drawCtx.save();
      drawCtx.globalAlpha = deltas.colorStrength;
      drawCtx.globalCompositeOperation = "source-atop";
      drawCtx.fillStyle = deltas.colorOverride;
      drawCtx.fillRect(0, 0, offscreen.width, offscreen.height);
      drawCtx.restore();
    }

    // Flash overlay on the offscreen canvas
    if (deltas.flashOverlay > 0) {
      drawCtx.save();
      drawCtx.globalAlpha = deltas.flashOverlay;
      drawCtx.globalCompositeOperation = "source-atop";
      drawCtx.fillStyle = "#ffffff";
      drawCtx.fillRect(0, 0, offscreen.width, offscreen.height);
      drawCtx.restore();
    }

    // Apply blur by downscaling then upscaling the offscreen canvas.
    // This avoids ctx.filter (broken in WebKitGTK) and getImageData (tainted canvas).
    if (totalBlur > 0) {
      const ow = offscreen.width;
      const oh = offscreen.height;
      // Scale factor: lower = more blur. blur 5 → 20%, blur 20 → 5%, blur 50 → 2%
      const scale = Math.max(0.02, 1 / (1 + totalBlur * 0.8));
      const sw = Math.max(2, Math.round(ow * scale));
      const sh = Math.max(2, Math.round(oh * scale));
      const tmpCanvas = document.createElement("canvas");
      const tmpCtx = tmpCanvas.getContext("2d")!;
      // Multiple passes for smoother blur
      const passes = Math.min(6, Math.max(2, Math.ceil(totalBlur / 5)));
      // First pass: read from offscreen
      tmpCanvas.width = sw;
      tmpCanvas.height = sh;
      tmpCtx.drawImage(offscreen, 0, 0, sw, sh);
      drawCtx.clearRect(0, 0, ow, oh);
      drawCtx.imageSmoothingEnabled = true;
      drawCtx.drawImage(tmpCanvas, 0, 0, ow, oh);
      // Subsequent passes: read from offscreen (which now has pass 1 result)
      for (let p = 1; p < passes; p++) {
        tmpCtx.clearRect(0, 0, sw, sh);
        tmpCtx.drawImage(offscreen, 0, 0, sw, sh);
        drawCtx.clearRect(0, 0, ow, oh);
        drawCtx.drawImage(tmpCanvas, 0, 0, ow, oh);
      }
    }

    // Composite the offscreen result back to the main canvas
    ctx.drawImage(offscreen, x - margin, y - margin);
  }

  ctx.restore();
}

/** Word-wrap text to fit within maxWidth, splitting on spaces */
function wrapText(ctx: CanvasRenderingContext2D, text: string, maxWidth: number): string[] {
  if (maxWidth <= 0 || ctx.measureText(text).width <= maxWidth) return [text];
  const words = text.split(/(\s+)/); // keep whitespace tokens for accurate joining
  const lines: string[] = [];
  let current = "";
  for (const word of words) {
    const test = current + word;
    if (current && ctx.measureText(test).width > maxWidth) {
      lines.push(current.trimEnd());
      current = word.trimStart();
    } else {
      current = test;
    }
  }
  if (current) lines.push(current.trimEnd());
  return lines.length > 0 ? lines : [text];
}

function renderText(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, _h: number) {
  const props = layer.properties;
  const fontSize = resolveNumber(props.fontSize, 24);
  const rawFamily = props.fontFamily || "sans-serif";
  // Quote font family names containing spaces for ctx.font
  const fontFamily = rawFamily.includes(" ") && !rawFamily.includes(",") && !rawFamily.startsWith('"') && !rawFamily.startsWith("'")
    ? `"${rawFamily}"` : rawFamily;
  const color = resolve(props.color, "#ffffff");
  const text = resolve(props.text, "");
  const align = props.textAlign || "left";

  ctx.font = `${fontSize}px ${fontFamily}`;
  ctx.fillStyle = color;
  ctx.textAlign = align;
  ctx.textBaseline = "top";

  if (props.shadow) {
    ctx.shadowColor = resolve(props.shadow.color, "#000000");
    ctx.shadowOffsetX = resolveNumber(props.shadow.dx, 0);
    ctx.shadowOffsetY = resolveNumber(props.shadow.dy, 0);
    ctx.shadowBlur = resolveNumber(props.shadow.radius, 0);
  }

  let textX = x;
  if (align === "center") textX = x + w / 2;
  else if (align === "right") textX = x + w;

  // Split on explicit newlines, then word-wrap each line to fit within width
  const rawLines = text.split("\n");
  const wrappedLines: string[] = [];
  for (const line of rawLines) {
    wrappedLines.push(...wrapText(ctx, line, w));
  }

  const maxLines = resolveNumber(props.maxLines, 0);
  const displayLines = maxLines > 0 ? wrappedLines.slice(0, maxLines) : wrappedLines;
  const lineSpacing = resolveNumber(props.lineSpacing, 0);
  const lineHeight = fontSize + lineSpacing;

  for (let i = 0; i < displayLines.length; i++) {
    const lineX = textX;
    const lineY = y + i * lineHeight;
    // Stroke (outline)
    if (props.textStroke) {
      const savedShadowColor = ctx.shadowColor;
      ctx.shadowColor = "transparent";
      const strokeColor = resolve(props.textStroke, "#000000");
      const sw = resolveNumber(props.textStrokeWidth, 2);

      if (props.textFillEnabled === false) {
        // Outline-only: use strokeText for hollow letters
        ctx.strokeStyle = strokeColor;
        ctx.lineWidth = sw;
        ctx.lineJoin = "round";
        ctx.strokeText(displayLines[i], lineX, lineY);
      } else {
        // Filled + outline: offset renders for crisp border, then fill on top
        const savedFill: string | CanvasGradient | CanvasPattern = ctx.fillStyle;
        ctx.fillStyle = strokeColor;
        for (let a = 0; a < Math.PI * 2; a += Math.PI / 4) {
          ctx.fillText(displayLines[i], lineX + Math.cos(a) * sw, lineY + Math.sin(a) * sw);
        }
        ctx.fillStyle = savedFill;
      }
      ctx.shadowColor = savedShadowColor;
    }
    // Fill
    if (props.textFillEnabled !== false) {
      ctx.fillText(displayLines[i], lineX, lineY);
    }
  }

  if (debugOverlay) {
    // DEBUG: compute actual absolute project-space position from canvas transform
    const _dt = ctx.getTransform();
    const _bt = baseTransform;
    let _debugAbsX = textX; // fallback
    if (_bt && _bt.a !== 0) {
      // canvas position of textX: _dt.e + textX * _dt.a
      // project position: (canvasPos - _bt.e) / _bt.a
      _debugAbsX = (_dt.e + textX * _dt.a - _bt.e) / _bt.a;
    }
    ctx.save();
    ctx.strokeStyle = "red";
    ctx.lineWidth = 2;
    ctx.setLineDash([]);
    ctx.beginPath();
    ctx.moveTo(textX, y);
    ctx.lineTo(textX, y + fontSize);
    ctx.stroke();
    // Show debug values above the text
    ctx.fillStyle = "yellow";
    ctx.font = "14px monospace";
    ctx.textAlign = "left";
    ctx.textBaseline = "bottom";
    ctx.fillText(`local=${Math.round(textX)} abs=${Math.round(_debugAbsX)} dt.e=${Math.round(_dt.e)} bt.e=${Math.round(_bt?.e||0)} dt.a=${_dt.a.toFixed(2)}`, textX, y - 2);
    ctx.restore();
  }

  // Update computed bounds to reflect actual text dimensions and alignment
  let maxLineWidth = 0;
  for (const line of displayLines) {
    const measured = ctx.measureText(line).width;
    if (measured > maxLineWidth) maxLineWidth = measured;
  }
  const totalHeight = displayLines.length * lineHeight - lineSpacing;
  const boundsEntry = computedBounds.get(layer.id);
  if (boundsEntry) {
    // Use the actual measured text width for bounds so the selection outline
    // wraps the visible text, not the (potentially huge) specified text box.
    if (align === "center") {
      // Text centered at x + w/2, visual left = x + w/2 - maxLineWidth/2
      boundsEntry.x += w / 2 - maxLineWidth / 2;
    } else if (align === "right") {
      // Text right-aligned at x + w, visual left = x + w - maxLineWidth
      boundsEntry.x += w - maxLineWidth;
    }
    boundsEntry.w = maxLineWidth;
    if (totalHeight > boundsEntry.h) boundsEntry.h = totalHeight;
  }

  // Reset shadow
  ctx.shadowColor = "transparent";
  ctx.shadowOffsetX = 0;
  ctx.shadowOffsetY = 0;
  ctx.shadowBlur = 0;
}

function renderShape(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const props = layer.properties;
  const kind = props.shapeKind || "rectangle";
  const fillRaw = resolve(props.fill, "#e94560");
  const noFill = fillRaw === "none" || fillRaw === "transparent";
  const stroke = props.stroke ? resolve(props.stroke) : undefined;
  const strokeWidth = resolveNumber(props.strokeWidth, 0);
  const effectiveStrokeWidth = stroke ? Math.max(strokeWidth, 1) : 0;
  const cornerRadius = resolveNumber(props.cornerRadius, 0);
  const skewX = resolveNumber(props.skewX, 0);
  const skewY = resolveNumber(props.skewY, 0);

  if (!noFill) ctx.fillStyle = fillRaw;
  if (stroke) {
    ctx.strokeStyle = stroke;
    ctx.lineWidth = effectiveStrokeWidth;
  }

  // Apply skew transform around the shape center
  const hasSkew = skewX !== 0 || skewY !== 0;
  if (hasSkew) {
    ctx.save();
    const cx = x + w / 2;
    const cy = y + h / 2;
    ctx.translate(cx, cy);
    ctx.transform(1, Math.tan(skewY * Math.PI / 180), Math.tan(skewX * Math.PI / 180), 1, 0, 0);
    ctx.translate(-cx, -cy);
  }

  // Set shadow right before draw calls so it's in the active context
  if (props.shadow) {
    ctx.shadowColor = resolve(props.shadow.color, "#000000");
    ctx.shadowOffsetX = resolveNumber(props.shadow.dx, 0);
    ctx.shadowOffsetY = resolveNumber(props.shadow.dy, 0);
    ctx.shadowBlur = resolveNumber(props.shadow.radius, 0);
  }

  switch (kind) {
    case "rectangle":
      if (cornerRadius > 0) {
        roundedRect(ctx, x, y, w, h, cornerRadius);
      } else {
        ctx.beginPath();
        ctx.rect(x, y, w, h);
      }
      if (!noFill) ctx.fill();
      if (stroke && effectiveStrokeWidth > 0) ctx.stroke();
      break;

    case "circle": {
      const radius = Math.min(w, h) / 2;
      ctx.beginPath();
      ctx.arc(x + w / 2, y + h / 2, radius, 0, Math.PI * 2);
      if (!noFill) ctx.fill();
      if (stroke && effectiveStrokeWidth > 0) ctx.stroke();
      break;
    }

    case "oval":
      ctx.beginPath();
      ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
      if (!noFill) ctx.fill();
      if (stroke && effectiveStrokeWidth > 0) ctx.stroke();
      break;

    case "triangle":
      ctx.beginPath();
      ctx.moveTo(x + w / 2, y);
      ctx.lineTo(x + w, y + h);
      ctx.lineTo(x, y + h);
      ctx.closePath();
      if (!noFill) ctx.fill();
      if (stroke && effectiveStrokeWidth > 0) ctx.stroke();
      break;

    case "arc": {
      const r = Math.min(w, h) / 2;
      ctx.beginPath();
      ctx.arc(x + w / 2, y + h / 2, r, 0, Math.PI);
      if (!noFill) ctx.fill();
      if (stroke && effectiveStrokeWidth > 0) ctx.stroke();
      break;
    }

    case "line": {
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + w, y + h);
      if (stroke) {
        ctx.stroke();
      } else {
        // Fallback: use fill color as stroke for lines if no stroke set
        ctx.strokeStyle = fillRaw;
        ctx.lineWidth = Math.max(effectiveStrokeWidth, 1);
        ctx.stroke();
      }
      break;
    }
  }

  if (hasSkew) ctx.restore();

  // Reset shadow
  ctx.shadowColor = "transparent";
  ctx.shadowOffsetX = 0;
  ctx.shadowOffsetY = 0;
  ctx.shadowBlur = 0;
}

function roundedRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  r = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
}

/** Create a shape path without filling or stroking it */
function createShapePath(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const props = layer.properties;
  const kind = props.shapeKind || "rectangle";
  const cornerRadius = resolveNumber(props.cornerRadius, 0);

  switch (kind) {
    case "rectangle":
      if (cornerRadius > 0) {
        roundedRect(ctx, x, y, w, h, cornerRadius);
      } else {
        ctx.beginPath();
        ctx.rect(x, y, w, h);
      }
      break;

    case "circle": {
      const radius = Math.min(w, h) / 2;
      ctx.beginPath();
      ctx.arc(x + w / 2, y + h / 2, radius, 0, Math.PI * 2);
      break;
    }

    case "oval":
      ctx.beginPath();
      ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
      break;

    case "triangle":
      ctx.beginPath();
      ctx.moveTo(x + w / 2, y);
      ctx.lineTo(x + w, y + h);
      ctx.lineTo(x, y + h);
      ctx.closePath();
      break;

    case "arc": {
      const r = Math.min(w, h) / 2;
      ctx.beginPath();
      ctx.arc(x + w / 2, y + h / 2, r, 0, Math.PI);
      break;
    }

    case "line": {
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + w, y + h);
      break;
    }
  }
}

/** Get descriptive label for an image that can't be displayed */
function getImageLabel(src: string): string {
  if (!src) return "No Image";
  const resolvedPath = resolveImagePath(src);
  if (!resolvedPath) {
    // Path can't be resolved (formula, missing asset dir, etc.)
    const name = src.split("/").pop() || src;
    return name.length > 22 ? name.slice(0, 19) + "..." : name;
  }
  // Check if it already failed to load
  let resolved: string;
  if (resolvedPath.startsWith("http") || resolvedPath.startsWith("data:")) {
    resolved = resolvedPath;
  } else if (!isTauri || import.meta.env.DEV) {
    resolved = `/__lava_assets${resolvedPath}`;
  } else if (convertFileSrc) {
    resolved = convertFileSrc(resolvedPath);
  } else {
    return "Loading...";
  }
  if (imageFailedMap.has(resolved)) return "Failed to load";
  return "Loading...";
}

function renderImage(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const src = resolve(layer.properties.src, "");
  const img = getCachedImage(src);

  // Flag animated GIFs so the render loop stays at full fps
  if (src && src.toLowerCase().endsWith(".gif")) hasGifImages = true;

  if (!img) {
    // Placeholder while loading or no source
    ctx.fillStyle = "#2a3a4a";
    ctx.fillRect(x, y, w, h);
    ctx.strokeStyle = "#556677";
    ctx.lineWidth = 1;
    ctx.strokeRect(x, y, w, h);

    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.lineTo(x + w, y + h);
    ctx.moveTo(x + w, y);
    ctx.lineTo(x, y + h);
    ctx.strokeStyle = "#3a4a5a";
    ctx.stroke();

    ctx.fillStyle = "#8899aa";
    ctx.font = `${Math.max(12, Math.min(w, h) * 0.15)}px sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(getImageLabel(src), x + w / 2, y + h / 2);
    return;
  }

  const scaleMode = layer.properties.scaleMode || "fit";
  const imgW = img.naturalWidth;
  const imgH = img.naturalHeight;
  const cornerRadius = resolveNumber(layer.properties.cornerRadius, 0);

  // Compute scaleMode dimensions first so shadow matches the visible image
  let drawX = x, drawY = y, drawW = w, drawH = h;

  switch (scaleMode) {
    case "stretch":
      // Just draw at the layer dimensions
      break;

    case "fit": {
      const scale = Math.min(w / imgW, h / imgH);
      drawW = imgW * scale;
      drawH = imgH * scale;
      drawX = x + (w - drawW) / 2;
      drawY = y + (h - drawH) / 2;
      break;
    }

    case "fill":
    case "crop": {
      const scale = Math.max(w / imgW, h / imgH);
      drawW = imgW * scale;
      drawH = imgH * scale;
      drawX = x + (w - drawW) / 2;
      drawY = y + (h - drawH) / 2;
      break;
    }
  }

  // Draw shadow using the image itself so transparent areas don't show an
  // opaque black fill.  We render the image (with optional corner-radius
  // clip) into a small offscreen canvas, then drawImage that onto the main
  // context with shadow enabled — the browser derives the shadow shape from
  // the offscreen bitmap's alpha channel.
  if (layer.properties.shadow) {
    const offCanvas = document.createElement("canvas");
    offCanvas.width = drawW;
    offCanvas.height = drawH;
    const offCtx = offCanvas.getContext("2d")!;
    if (cornerRadius > 0) {
      offCtx.beginPath();
      roundedRect(offCtx, 0, 0, drawW, drawH, cornerRadius);
      offCtx.clip();
    }
    offCtx.drawImage(img, 0, 0, drawW, drawH);

    ctx.save();
    ctx.shadowColor = resolve(layer.properties.shadow.color, "#000000");
    ctx.shadowOffsetX = resolveNumber(layer.properties.shadow.dx, 0);
    ctx.shadowOffsetY = resolveNumber(layer.properties.shadow.dy, 0);
    ctx.shadowBlur = resolveNumber(layer.properties.shadow.radius, 0);
    ctx.drawImage(offCanvas, drawX, drawY);
    ctx.restore();
  }

  ctx.save();
  ctx.beginPath();
  if (cornerRadius > 0) {
    roundedRect(ctx, x, y, w, h, cornerRadius);
  } else {
    ctx.rect(x, y, w, h);
  }
  ctx.clip();

  // Apply FX filters using CSS filter string on the canvas context.
  // This avoids getImageData (which fails on tainted/cross-origin canvases
  // from asset:// or http images) and is GPU-accelerated.
  // Blur is now handled at renderLayer level (combined with animation blur).
  // Only non-blur FX filters are handled here.
  const fxContrast = resolveNumber(layer.properties.fxContrast, 100);
  const fxSaturation = resolveNumber(layer.properties.fxSaturation, 100);
  const fxBrightness = resolveNumber(layer.properties.fxBrightness, 100);
  const fxSepia = resolveNumber(layer.properties.fxSepia, 0);
  const fxGrayscale = resolveNumber(layer.properties.fxGrayscale, 0);
  const fxInvert = resolveNumber(layer.properties.fxInvert, 0);
  const hasFx = fxContrast !== 100 || fxSaturation !== 100 ||
    fxBrightness !== 100 || fxSepia > 0 || fxGrayscale > 0 || fxInvert > 0;

  if (hasFx) {
    // Build a compound CSS filter string
    const filters: string[] = [];
    if (fxBrightness !== 100) filters.push(`brightness(${fxBrightness / 100})`);
    if (fxContrast !== 100) filters.push(`contrast(${fxContrast / 100})`);
    if (fxSaturation !== 100) filters.push(`saturate(${fxSaturation / 100})`);
    if (fxGrayscale > 0) filters.push(`grayscale(${fxGrayscale / 100})`);
    if (fxSepia > 0) filters.push(`sepia(${fxSepia / 100})`);
    if (fxInvert > 0) filters.push(`invert(${fxInvert / 100})`);
    const filterStr = filters.join(" ");

    if (canvasFilterWorks()) {
      // GPU-accelerated path: apply non-blur CSS filters on offscreen canvas.
      // Blur is handled separately via manual box blur since CSS blur() is
      // unreliable across WebKitGTK versions.
      const nonBlurFilters: string[] = [];
      if (fxBrightness !== 100) nonBlurFilters.push(`brightness(${fxBrightness / 100})`);
      if (fxContrast !== 100) nonBlurFilters.push(`contrast(${fxContrast / 100})`);
      if (fxSaturation !== 100) nonBlurFilters.push(`saturate(${fxSaturation / 100})`);
      if (fxGrayscale > 0) nonBlurFilters.push(`grayscale(${fxGrayscale / 100})`);
      if (fxSepia > 0) nonBlurFilters.push(`sepia(${fxSepia / 100})`);
      if (fxInvert > 0) nonBlurFilters.push(`invert(${fxInvert / 100})`);

      const offCanvas = document.createElement("canvas");
      offCanvas.width = Math.ceil(drawW);
      offCanvas.height = Math.ceil(drawH);
      const offCtx = offCanvas.getContext("2d")!;
      if (nonBlurFilters.length > 0) offCtx.filter = nonBlurFilters.join(" ");
      offCtx.drawImage(img, 0, 0, drawW, drawH);
      offCtx.filter = "none";

      ctx.drawImage(offCanvas, drawX, drawY, drawW, drawH);
    } else {
      // Fallback for WebKitGTK builds where ctx.filter is not implemented:
      // draw to offscreen and apply pixel-level adjustments.
      const offCanvas = document.createElement("canvas");
      offCanvas.width = Math.ceil(drawW);
      offCanvas.height = Math.ceil(drawH);
      const offCtx = offCanvas.getContext("2d")!;
      offCtx.drawImage(img, 0, 0, drawW, drawH);

      try {
        const imageData = offCtx.getImageData(0, 0, offCanvas.width, offCanvas.height);
        const d = imageData.data;
        const contrastF = fxContrast / 100;
        const brightnessF = fxBrightness / 100;
        const saturationF = fxSaturation / 100;
        const sepiaF = fxSepia / 100;
        const grayscaleF = fxGrayscale / 100;
        const invertF = fxInvert / 100;

        for (let i = 0; i < d.length; i += 4) {
          let r = d[i], g = d[i + 1], b = d[i + 2];
          if (brightnessF !== 1) { r *= brightnessF; g *= brightnessF; b *= brightnessF; }
          if (contrastF !== 1) {
            r = (r - 128) * contrastF + 128;
            g = (g - 128) * contrastF + 128;
            b = (b - 128) * contrastF + 128;
          }
          if (saturationF !== 1) {
            const gray = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            r = gray + (r - gray) * saturationF;
            g = gray + (g - gray) * saturationF;
            b = gray + (b - gray) * saturationF;
          }
          if (grayscaleF > 0) {
            const gray = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            r = r + (gray - r) * grayscaleF;
            g = g + (gray - g) * grayscaleF;
            b = b + (gray - b) * grayscaleF;
          }
          if (sepiaF > 0) {
            const sr = Math.min(255, r * 0.393 + g * 0.769 + b * 0.189);
            const sg = Math.min(255, r * 0.349 + g * 0.686 + b * 0.168);
            const sb = Math.min(255, r * 0.272 + g * 0.534 + b * 0.131);
            r = r + (sr - r) * sepiaF;
            g = g + (sg - g) * sepiaF;
            b = b + (sb - b) * sepiaF;
          }
          if (invertF > 0) {
            r = r + (255 - 2 * r) * invertF;
            g = g + (255 - 2 * g) * invertF;
            b = b + (255 - 2 * b) * invertF;
          }
          d[i] = Math.max(0, Math.min(255, r));
          d[i + 1] = Math.max(0, Math.min(255, g));
          d[i + 2] = Math.max(0, Math.min(255, b));
        }

        offCtx.putImageData(imageData, 0, 0);
        ctx.drawImage(offCanvas, drawX, drawY, drawW, drawH);
      } catch {
        // getImageData failed (tainted canvas from cross-origin image) —
        // try applying filter directly on the main context as last resort
        try {
          ctx.filter = filterStr;
          ctx.drawImage(img, drawX, drawY, drawW, drawH);
          ctx.filter = "none";
        } catch {
          // No filter support at all — draw image unfiltered
          ctx.drawImage(img, drawX, drawY, drawW, drawH);
        }
      }
    }
  } else {
    ctx.drawImage(img, drawX, drawY, drawW, drawH);
  }


  // Apply tint color overlay over the drawn image
  const tint = layer.properties.tint;
  if (tint && tint !== "#ffffff" && tint !== "#FFFFFF" && tint !== "#fff" && tint !== "#FFF") {
    ctx.globalCompositeOperation = "source-atop";
    ctx.fillStyle = resolve(tint, "#ffffff");
    ctx.globalAlpha *= 0.4;
    ctx.fillRect(drawX, drawY, drawW, drawH);
    ctx.globalCompositeOperation = "source-over";
  }

  ctx.restore();
}

function renderProgress(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const props = layer.properties;
  const min = resolveNumber(props.min, 0);
  const max = resolveNumber(props.max, 100);
  const value = resolveNumber(props.value, 50);
  const color = resolve(props.color, "#e94560");
  const trackColor = resolve(props.trackColor, "#ffffff20");
  const strokeWidth = resolveNumber(props.strokeWidth, 6);
  const style = props.style || "arc";
  const progress = Math.max(0, Math.min(1, (value - min) / (max - min)));

  if (style === "bar") {
    // Horizontal bar
    ctx.fillStyle = trackColor;
    ctx.fillRect(x, y + h / 2 - strokeWidth / 2, w, strokeWidth);
    ctx.fillStyle = color;
    ctx.fillRect(x, y + h / 2 - strokeWidth / 2, w * progress, strokeWidth);
  } else {
    // Arc / circle
    const cx = x + w / 2;
    const cy = y + h / 2;
    const radius = Math.min(w, h) / 2 - strokeWidth / 2;
    const startAngle = -Math.PI / 2;
    const endAngle = startAngle + Math.PI * 2 * progress;

    // Track
    ctx.beginPath();
    ctx.arc(cx, cy, radius, 0, Math.PI * 2);
    ctx.strokeStyle = trackColor;
    ctx.lineWidth = strokeWidth;
    ctx.lineCap = "round";
    ctx.stroke();

    // Progress
    ctx.beginPath();
    ctx.arc(cx, cy, radius, startAngle, endAngle);
    ctx.strokeStyle = color;
    ctx.lineWidth = strokeWidth;
    ctx.lineCap = "round";
    ctx.stroke();
  }
}

/** Resolve a raw image source to a URL the browser can load.
 *  Handles kfile://, absolute paths (via convertFileSrc), data: and http URLs. */
function resolveImageSrc(src: string): string | null {
  const resolvedPath = resolveImagePath(src);
  if (!resolvedPath) return null;

  if (resolvedPath.startsWith("http") || resolvedPath.startsWith("data:")) {
    return resolvedPath;
  }
  if (!isTauri || import.meta.env.DEV) {
    return `/__lava_assets${resolvedPath}`;
  }
  if (convertFileSrc) {
    return convertFileSrc(resolvedPath);
  }
  return null; // Tauri production but convertFileSrc not loaded yet
}

function renderFontIcon(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const props = layer.properties;
  const color = resolve(props.color, "#ffffff");

  // If iconSrc is set, render as image instead of font glyph
  const iconSrc = props.iconSrc ? resolve(props.iconSrc, "") : "";
  if (iconSrc) {
    const resolved = resolveImageSrc(iconSrc);
    if (resolved) {
      const img = getCachedImage(iconSrc);
      if (img) {
        // Tint: draw image then overlay color using composite
        ctx.save();
        ctx.drawImage(img, x, y, w, h);
        if (color && color !== "#ffffff" && color !== "#FFFFFF" && color !== "#fff" && color !== "#FFF") {
          ctx.globalCompositeOperation = "source-atop";
          ctx.fillStyle = color;
          ctx.fillRect(x, y, w, h);
          ctx.globalCompositeOperation = "source-over";
        }
        ctx.restore();
      } else {
        // Placeholder while icon image loads
        ctx.fillStyle = color;
        ctx.globalAlpha *= 0.3;
        ctx.fillRect(x, y, w, h);
      }
    }
    return;
  }

  // Font glyph rendering
  const glyphCode = props.glyphCode || "e88a";
  const fontSize = resolveNumber(props.fontSize, 48);
  const iconSet = props.iconSet || "Material Icons";

  // Map common icon set names to CSS font families
  const fontMap: Record<string, string> = {
    "material": "Material Icons",
    "fontawesome": "Font Awesome 6 Free",
    "weathericons": "Weather Icons",
  };
  const fontFamily = fontMap[iconSet] || iconSet;

  // Convert hex glyph code to character
  const codePoint = parseInt(glyphCode, 16);
  const char = isNaN(codePoint) ? "?" : String.fromCodePoint(codePoint);

  // Check if the font is actually loaded; show a colored placeholder if not
  const fontSpec = `${fontSize}px "${fontFamily}"`;
  const fontReady = document.fonts.check(fontSpec);

  if (fontReady) {
    ctx.font = `${fontSize}px "${fontFamily}", sans-serif`;
    ctx.fillStyle = color;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(char, x + w / 2, y + h / 2);
  } else {
    // Font not loaded yet — draw a colored square placeholder
    ctx.fillStyle = color;
    ctx.globalAlpha *= 0.25;
    const size = Math.min(w, h) * 0.6;
    const px = x + (w - size) / 2;
    const py = y + (h - size) / 2;
    ctx.fillRect(px, py, size, size);
  }
}

function renderVisualizer(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const props = layer.properties;
  const vizStyle = resolve(props.vizStyle, "bars");

  if (vizStyle === "wave") {
    renderVisualizerWave(ctx, layer, x, y, w, h);
  } else {
    renderVisualizerBars(ctx, layer, x, y, w, h);
  }
}

function renderVisualizerBars(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const props = layer.properties;
  const barCount = resolveNumber(props.barCount, 24);
  const barSpacing = resolveNumber(props.barSpacing, 3);
  const sensitivity = resolveNumber(props.sensitivity, 1.0);

  const colorTop = resolve(props.colorTop, "#88C0D0");
  const colorMid = resolve(props.colorMid, "#5E81AC");
  const colorBottom = resolve(props.colorBottom, "#2E3440");
  const peakColor = resolve(props.peakColor, "#ECEFF4");

  const bandsData = getAudioBands();
  const peaksData = getAudioPeaks();

  const totalSpacing = barSpacing * (barCount - 1);
  const barW = Math.max(1, (w - totalSpacing) / barCount);
  const cornerR = Math.min(barW / 2, 3);

  for (let i = 0; i < barCount; i++) {
    const bandIdx = Math.min(
      Math.floor((i / barCount) * bandsData.length),
      bandsData.length - 1
    );
    const rawVal = bandsData[bandIdx] ?? 0;
    const val = Math.min(1, rawVal * sensitivity);
    const barH = Math.max(2, val * h);

    const bx = x + i * (barW + barSpacing);
    const by = y + h - barH;

    const grad = ctx.createLinearGradient(bx, by + barH, bx, by);
    grad.addColorStop(0, withAlpha(colorBottom, 0.38));
    grad.addColorStop(0.4, withAlpha(colorMid, 0.8));
    grad.addColorStop(1, colorTop);

    ctx.fillStyle = grad;

    if (cornerR > 0 && barH > cornerR * 2) {
      ctx.beginPath();
      ctx.moveTo(bx, by + barH);
      ctx.lineTo(bx, by + cornerR);
      ctx.quadraticCurveTo(bx, by, bx + cornerR, by);
      ctx.lineTo(bx + barW - cornerR, by);
      ctx.quadraticCurveTo(bx + barW, by, bx + barW, by + cornerR);
      ctx.lineTo(bx + barW, by + barH);
      ctx.closePath();
      ctx.fill();
    } else {
      ctx.fillRect(bx, by, barW, barH);
    }

    if (peaksData[bandIdx] > 0.05) {
      const peakH = Math.min(1, peaksData[bandIdx] * sensitivity) * h;
      const peakY = y + h - peakH;
      ctx.fillStyle = withAlpha(peakColor, 0.8);
      ctx.fillRect(bx, peakY - 2, barW, 2);
    }
  }
}

function renderVisualizerWave(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const props = layer.properties;
  const pointCount = resolveNumber(props.barCount, 24);
  const sensitivity = resolveNumber(props.sensitivity, 1.0);

  const colorTop = resolve(props.colorTop, "#88C0D0");
  const colorMid = resolve(props.colorMid, "#5E81AC");
  const colorBottom = resolve(props.colorBottom, "#2E3440");

  const bandsData = getAudioBands();

  // Build data points along the width
  const points: { x: number; y: number }[] = [];
  for (let i = 0; i < pointCount; i++) {
    const bandIdx = Math.min(
      Math.floor((i / pointCount) * bandsData.length),
      bandsData.length - 1
    );
    const rawVal = bandsData[bandIdx] ?? 0;
    const val = Math.min(1, rawVal * sensitivity);
    const px = x + (i / (pointCount - 1)) * w;
    const py = y + h - Math.max(2, val * h);
    points.push({ x: px, y: py });
  }

  if (points.length < 2) return;

  // Catmull-Rom to cubic bezier conversion for smooth curves.
  // For each segment [P[i], P[i+1]], control points are:
  //   CP1 = P[i]   + (P[i+1] - P[i-1]) / 6
  //   CP2 = P[i+1] - (P[i+2] - P[i])   / 6
  ctx.beginPath();
  ctx.moveTo(x, y + h); // bottom-left corner
  ctx.lineTo(points[0].x, points[0].y);

  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[Math.max(0, i - 1)];
    const p1 = points[i];
    const p2 = points[i + 1];
    const p3 = points[Math.min(points.length - 1, i + 2)];

    const cp1x = p1.x + (p2.x - p0.x) / 6;
    const cp1y = p1.y + (p2.y - p0.y) / 6;
    const cp2x = p2.x - (p3.x - p1.x) / 6;
    const cp2y = p2.y - (p3.y - p1.y) / 6;

    ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
  }

  ctx.lineTo(x + w, y + h); // bottom-right corner
  ctx.closePath();

  // Filled gradient: colorBottom at base → colorMid → colorTop at peaks
  const fillGrad = ctx.createLinearGradient(x, y + h, x, y);
  fillGrad.addColorStop(0, withAlpha(colorBottom, 0.25));
  fillGrad.addColorStop(0.4, withAlpha(colorMid, 0.6));
  fillGrad.addColorStop(1, withAlpha(colorTop, 0.87));
  ctx.fillStyle = fillGrad;
  ctx.fill();

  // Stroke the wave line on top for definition
  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[Math.max(0, i - 1)];
    const p1 = points[i];
    const p2 = points[i + 1];
    const p3 = points[Math.min(points.length - 1, i + 2)];

    const cp1x = p1.x + (p2.x - p0.x) / 6;
    const cp1y = p1.y + (p2.y - p0.y) / 6;
    const cp2x = p2.x - (p3.x - p1.x) / 6;
    const cp2y = p2.y - (p3.y - p1.y) / 6;

    ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
  }

  const strokeGrad = ctx.createLinearGradient(x, y + h, x, y);
  strokeGrad.addColorStop(0, colorMid);
  strokeGrad.addColorStop(1, colorTop);
  ctx.strokeStyle = strokeGrad;
  ctx.lineWidth = 2;
  ctx.stroke();
}

function renderOverlap(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, container: ContainerSize, parentAbsX: number, parentAbsY: number, timestamp: number = 0) {
  if (!layer.children) return;
  ctx.save();
  ctx.translate(x, y);
  const absX = parentAbsX + x;
  const absY = parentAbsY + y;
  const childContainer: ContainerSize = {
    width: resolveNumber(layer.properties.width, container.width),
    height: resolveNumber(layer.properties.height, container.height),
  };

  if (layer.properties.clipFirstChild && layer.children.length > 0) {
    const firstChild = layer.children[0];

    // Render the first child normally
    renderLayer(ctx, firstChild, childContainer, absX, absY, timestamp);

    // Set up clipping path from the first child's shape
    ctx.save();
    const fcProps = firstChild.properties;
    const fcW = resolveNumber(fcProps.width, 0);
    const fcH = resolveNumber(fcProps.height, 0);
    const fcOffsetX = resolveNumber(fcProps.x, 0);
    const fcOffsetY = resolveNumber(fcProps.y, 0);
    const { x: fcX, y: fcY } = anchorPosition(fcOffsetX, fcOffsetY, fcW, fcH, fcProps.anchor, childContainer);

    if (firstChild.type === "shape") {
      createShapePath(ctx, firstChild, fcX, fcY, fcW, fcH);
    } else {
      // Non-shape first child: clip to its rectangular bounds
      ctx.beginPath();
      ctx.rect(fcX, fcY, fcW, fcH);
    }
    ctx.clip();

    // Render remaining children within the clip
    for (let i = 1; i < layer.children.length; i++) {
      renderLayer(ctx, layer.children[i], childContainer, absX, absY, timestamp);
    }
    ctx.restore();
  } else {
    for (const child of layer.children) {
      renderLayer(ctx, child, childContainer, absX, absY, timestamp);
    }
  }

  ctx.restore();
}

function renderStack(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, container: ContainerSize, parentAbsX: number, parentAbsY: number, timestamp: number = 0) {
  if (!layer.children) return;
  ctx.save();
  ctx.translate(x, y);
  const absX = parentAbsX + x;
  const absY = parentAbsY + y;

  const orientation = layer.properties.orientation || "vertical";
  const spacing = resolveNumber(layer.properties.spacing, 0);
  let offset = 0;
  const childContainer: ContainerSize = {
    width: resolveNumber(layer.properties.width, container.width),
    height: resolveNumber(layer.properties.height, container.height),
  };

  for (const child of layer.children) {
    if (!isLayerVisible(child)) continue;
    ctx.save();
    if (orientation === "horizontal") {
      ctx.translate(offset, 0);
      renderLayer(ctx, child, childContainer, absX + offset, absY, timestamp);
      offset += resolveNumber(child.properties.width, 0) + spacing;
    } else {
      ctx.translate(0, offset);
      renderLayer(ctx, child, childContainer, absX, absY + offset, timestamp);
      offset += resolveNumber(child.properties.height, 0) + spacing;
    }
    ctx.restore();
  }
  ctx.restore();
}

function drawSelectionOutline(ctx: CanvasRenderingContext2D, bounds: LayerBounds) {
  const { x, y, w, h } = bounds;
  const pad = 2;

  if (debugOverlay) {
    // DEBUG: green line at bounds origin + show values
    ctx.save();
    ctx.strokeStyle = "lime";
    ctx.lineWidth = 3;
    ctx.setLineDash([]);
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.lineTo(x, y + h);
    ctx.stroke();
    ctx.fillStyle = "lime";
    ctx.font = "14px monospace";
    ctx.textAlign = "left";
    ctx.textBaseline = "top";
    ctx.fillText(`bounds.x=${Math.round(x)} bounds.y=${Math.round(y)}`, x, y + h + 4);
    ctx.restore();
  }

  ctx.save();
  ctx.strokeStyle = "#4a9eff";
  ctx.lineWidth = 1.5;
  ctx.setLineDash([4, 3]);
  ctx.strokeRect(x - pad, y - pad, w + pad * 2, h + pad * 2);
  ctx.setLineDash([]);

  // Resize handles
  const handleSize = 6;
  ctx.fillStyle = "#4a9eff";
  const handles = [
    [x - pad, y - pad],
    [x + w / 2, y - pad],
    [x + w + pad, y - pad],
    [x + w + pad, y + h / 2],
    [x + w + pad, y + h + pad],
    [x + w / 2, y + h + pad],
    [x - pad, y + h + pad],
    [x - pad, y + h / 2],
  ];
  for (const [hx, hy] of handles) {
    ctx.fillRect(hx - handleSize / 2, hy - handleSize / 2, handleSize, handleSize);
  }

  ctx.restore();
}

function renderPaintLayer(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const strokes: PaintStroke[] = (layer.properties.paintStrokes as any) || [];

  // Draw a subtle canvas/background when there are no strokes (editor hint)
  if (strokes.length === 0) {
    ctx.save();
    ctx.fillStyle = "#1a1a2e";
    ctx.globalAlpha = 0.15;
    ctx.fillRect(x, y, w, h);
    ctx.globalAlpha = 1;
    ctx.strokeStyle = "#3a4a5a";
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 4]);
    ctx.strokeRect(x, y, w, h);
    ctx.setLineDash([]);
    ctx.fillStyle = "#6a7a8a";
    ctx.font = `${Math.max(12, Math.min(w, h) * 0.08)}px sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText("Paint Layer", x + w / 2, y + h / 2);
    ctx.restore();
    return;
  }

  // Clip to layer bounds and translate so strokes render relative to layer origin
  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, w, h);
  ctx.clip();
  ctx.translate(x, y);
  renderPaintStrokes(ctx, strokes);
  ctx.restore();
}

function renderMapPlaceholder(ctx: CanvasRenderingContext2D, _layer: Layer, x: number, y: number, w: number, h: number) {
  ctx.save();
  ctx.fillStyle = "#1a2a3a";
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, 8);
  ctx.fill();
  ctx.strokeStyle = "#3a5a7a";
  ctx.lineWidth = 2;
  ctx.stroke();

  // Crosshatch pattern
  ctx.strokeStyle = "#2a3a4a";
  ctx.lineWidth = 1;
  const step = 20;
  for (let i = x; i < x + w; i += step) {
    ctx.beginPath(); ctx.moveTo(i, y); ctx.lineTo(i, y + h); ctx.stroke();
  }
  for (let j = y; j < y + h; j += step) {
    ctx.beginPath(); ctx.moveTo(x, j); ctx.lineTo(x + w, j); ctx.stroke();
  }

  // Label
  ctx.fillStyle = "#7ab8d8";
  ctx.font = `bold ${Math.min(w / 8, 24)}px sans-serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText("Map", x + w / 2, y + h / 2);
  ctx.restore();
}

// ─── Launcher module-level state ──────────────────────────────────────────────
const _launcherApps: { name: string; exec: string; icon: string }[] = [];
let _launcherAppsLoaded = false;
let _launcherAppsRetryCount = 0;
const _launcherIconCache = new Map<string, HTMLImageElement | null>();

/** Hit regions keyed by layer id */
export const launcherHitRegions = new Map<string, Array<{ exec: string; bx: number; by: number; bw: number; bh: number }>>();

/** Hover position in project coords — set by CanvasRenderer on mousemove */
let _hoverX = -1;
let _hoverY = -1;
export function setLauncherHoverCoords(x: number, y: number) { _hoverX = x; _hoverY = y; }

/** Start Menu open state */
let _startMenuOpen = false;
function notifyStartMenuState(open: boolean) {
  const wk = (window as any).webkit?.messageHandlers?.lava;
  if (wk) wk.postMessage(JSON.stringify({ type: open ? "start_menu_open" : "start_menu_close" }));
}
export function toggleStartMenu() {
  _startMenuOpen = !_startMenuOpen;
  notifyStartMenuState(_startMenuOpen);
}
export function closeStartMenu() {
  if (_startMenuOpen) { _startMenuOpen = false; notifyStartMenuState(false); }
}
export function isStartMenuOpen() { return _startMenuOpen; }

/** Running window classes from hyprctl */
let _runningClasses: string[] = [];
let _activeClass = "";
let _windowStatePollId: ReturnType<typeof setInterval> | null = null;

async function loadLauncherApps() {
  if (_launcherAppsLoaded) return;
  _launcherAppsLoaded = true;
  if (isTauri) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const apps = await invoke<{ name: string; exec: string; icon: string }[]>("list_apps");
      _launcherApps.splice(0, _launcherApps.length, ...apps);
    } catch (e) {
      console.warn("list_apps failed:", e);
    }
  } else {
    // Wallpaper mode: use pre-injected apps list
    const injected = (window as any).__LAVA_APPS;
    if (Array.isArray(injected) && injected.length > 0) {
      _launcherApps.splice(0, _launcherApps.length, ...injected);
    } else if (_launcherAppsRetryCount < 20) {
      // __LAVA_APPS not yet injected — reset so next render frame retries
      _launcherAppsRetryCount++;
      _launcherAppsLoaded = false;
    }
  }
  // Start polling running windows every 2s (editor only — hyprctl not available in standalone)
  if (isTauri && !_windowStatePollId) {
    pollWindowState();
    _windowStatePollId = setInterval(pollWindowState, 2000);
  }
}

async function pollWindowState() {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const state = await invoke<{ running_classes: string[]; active_class: string }>("get_window_state");
    _runningClasses = state.running_classes;
    _activeClass = state.active_class;
  } catch { /* hyprland not running */ }
}

function getLauncherIcon(iconPath: string): HTMLImageElement | null {
  if (!iconPath) return null;
  if (_launcherIconCache.has(iconPath)) return _launcherIconCache.get(iconPath) ?? null;
  // iconPath is a resolved absolute path from list_apps
  // Use the same path resolution as getCachedImage for cross-mode compatibility
  let src: string;
  if (!isTauri || import.meta.env.DEV) {
    src = `/__lava_assets${iconPath}`;
  } else if (convertFileSrc) {
    src = convertFileSrc(iconPath);
  } else {
    return null; // convertFileSrc not loaded yet — wait
  }
  const img = new Image();
  img.onload = () => { _launcherIconCache.set(iconPath, img); };
  img.onerror = () => { _launcherIconCache.set(iconPath, null); };
  img.src = src;
  return null;
}

function isHovering(bx: number, by: number, bw: number, bh: number): boolean {
  return _hoverX >= bx && _hoverX <= bx + bw && _hoverY >= by && _hoverY <= by + bh;
}

function isAppRunning(exec: string, runningClasses: string[]): boolean {
  const base = exec.split(/[\s/]/).pop()?.toLowerCase() ?? "";
  return runningClasses.some(c => c === base || c.includes(base) || base.includes(c));
}

function isAppActive(exec: string, activeClass: string): boolean {
  if (!activeClass) return false;
  const base = exec.split(/[\s/]/).pop()?.toLowerCase() ?? "";
  return activeClass === base || activeClass.includes(base) || base.includes(activeClass);
}

function stringToColor(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) hash = str.charCodeAt(i) + ((hash << 5) - hash);
  return `hsl(${Math.abs(hash) % 360}, 55%, 42%)`;
}

// ─── Main launcher dispatcher ──────────────────────────────────────────────────

function renderLauncherLayer(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const props = layer.properties;
  const style = props.launcherStyle ?? "win11";
  const iconSize = props.launcherIconSize ?? 36;
  const pinned = props.pinnedApps ?? [];
  const p = props as any;
  const taskbarBg = String(p.taskbarBg ?? "#141414");
  const taskbarBgOpacity = Math.round(Number(p.taskbarBgOpacity ?? 235));
  const taskbarRadius = Number(p.taskbarRadius ?? 0);

  if (!_launcherAppsLoaded) { loadLauncherApps(); }

  const pinnedEntries = pinned.map(exec => {
    const app = _launcherApps.find(a => a.exec === exec)
      || _launcherApps.find(a => a.exec.startsWith(exec + " ") || a.exec.split(/[\s/]/).pop() === exec);
    return { exec, name: app?.name ?? exec, icon: app?.icon ?? exec };
  });

  const regions: Array<{ exec: string; bx: number; by: number; bw: number; bh: number }> = [];

  if (style === "win11") {
    renderWin11AppIcons(ctx, x, y, w, h, iconSize, pinnedEntries, regions);
  } else if (style === "macos") {
    renderMacosDock(ctx, x, y, w, h, iconSize, pinnedEntries, regions, taskbarBg, taskbarBgOpacity, taskbarRadius);
  } else {
    renderDeepinDock(ctx, x, y, w, h, iconSize, pinnedEntries, regions, taskbarBg, taskbarBgOpacity, taskbarRadius);
  }

  launcherHitRegions.set(layer.id, regions);
}

// ─── Windows 11 App Icons (pinned apps row only — no background/tray/clock) ───

function renderWin11AppIcons(
  ctx: CanvasRenderingContext2D,
  x: number, y: number, w: number, h: number,
  iconSize: number,
  apps: Array<{ exec: string; name: string; icon: string }>,
  regions: Array<{ exec: string; bx: number; by: number; bw: number; bh: number }>
) {
  ctx.save();

  const slotW = iconSize + 8;
  const slotH = iconSize + 8;
  const slotY = y + (h - slotH) / 2;
  const iconY = y + (h - iconSize) / 2;

  // Center the icon row within the layer bounds
  const totalW = apps.length * (slotW + 4) - (apps.length > 0 ? 4 : 0);
  let cx = x + (w - totalW) / 2;

  for (const app of apps) {
    const running = isAppRunning(app.exec, _runningClasses);
    const active = isAppActive(app.exec, _activeClass);
    const appHover = isHovering(cx, slotY, slotW, slotH);

    // Hover / active highlight background
    if (active) {
      ctx.fillStyle = "rgba(255,255,255,0.13)";
      ctx.beginPath(); ctx.roundRect(cx, slotY, slotW, slotH, 4); ctx.fill();
    } else if (appHover) {
      ctx.fillStyle = "rgba(255,255,255,0.09)";
      ctx.beginPath(); ctx.roundRect(cx, slotY, slotW, slotH, 4); ctx.fill();
    }

    // App icon image (with fallback letter circle)
    const imgX = cx + (slotW - iconSize) / 2;
    const img = getLauncherIcon(app.icon);
    if (img) {
      ctx.save();
      ctx.beginPath(); ctx.roundRect(imgX, iconY, iconSize, iconSize, 4); ctx.clip();
      ctx.drawImage(img, imgX, iconY, iconSize, iconSize);
      ctx.restore();
    } else {
      ctx.fillStyle = stringToColor(app.name);
      ctx.beginPath(); ctx.roundRect(imgX, iconY, iconSize, iconSize, 6); ctx.fill();
      ctx.fillStyle = "#fff";
      ctx.font = `bold ${Math.floor(iconSize * 0.48)}px sans-serif`;
      ctx.textAlign = "center"; ctx.textBaseline = "middle";
      ctx.fillText(app.name.charAt(0).toUpperCase(), imgX + iconSize / 2, iconY + iconSize / 2);
    }

    // Running indicator: small dot for running, wider pill for active
    if (running) {
      const dotW = active ? 14 : 4;
      const dotH = 3;
      const dotX = cx + slotW / 2 - dotW / 2;
      const dotY = y + h - 5;
      ctx.fillStyle = active ? "#60cdff" : "rgba(255,255,255,0.65)";
      ctx.beginPath(); ctx.roundRect(dotX, dotY, dotW, dotH, 1.5); ctx.fill();
    }

    regions.push({ exec: app.exec, bx: cx, by: y, bw: slotW, bh: h });
    cx += slotW + 4;
  }

  ctx.restore();
}

// ─── macOS Dock ───────────────────────────────────────────────────────────────

function renderMacosDock(
  ctx: CanvasRenderingContext2D,
  x: number, y: number, w: number, h: number,
  iconSize: number,
  apps: Array<{ exec: string; name: string; icon: string }>,
  regions: Array<{ exec: string; bx: number; by: number; bw: number; bh: number }>,
  bgColor = "#ffffff", bgOpacity = 46, _radius = 16
) {
  const gap = 8;
  const pad = 12;
  const totalW = apps.length * (iconSize + gap) - gap + pad * 2;
  const dockX = x + (w - totalW) / 2;
  const dockY = y + 4;
  const dockH = h - 8;

  ctx.save();
  const r = parseInt(bgColor.slice(1, 3), 16) || 255;
  const g = parseInt(bgColor.slice(3, 5), 16) || 255;
  const b = parseInt(bgColor.slice(5, 7), 16) || 255;
  ctx.fillStyle = `rgba(${r},${g},${b},${(bgOpacity / 255).toFixed(3)})`;
  ctx.beginPath(); ctx.roundRect(dockX, dockY, totalW, dockH, 16); ctx.fill();
  ctx.strokeStyle = "rgba(255,255,255,0.3)"; ctx.lineWidth = 1; ctx.stroke();

  let ix = dockX + pad;
  const iy = dockY + (dockH - iconSize) / 2;

  for (const app of apps) {
    const hover = isHovering(ix, dockY, iconSize, dockH);
    const running = isAppRunning(app.exec, _runningClasses);
    const active = isAppActive(app.exec, _activeClass);
    const scale = hover ? 1.15 : 1;
    const scaledSize = iconSize * scale;
    const scaledX = ix + (iconSize - scaledSize) / 2;
    const scaledY = iy + (iconSize - scaledSize);

    const img = getLauncherIcon(app.icon);
    if (img) {
      ctx.save();
      ctx.beginPath(); ctx.roundRect(scaledX, scaledY, scaledSize, scaledSize, scaledSize * 0.22); ctx.clip();
      ctx.drawImage(img, scaledX, scaledY, scaledSize, scaledSize);
      ctx.restore();
    } else {
      ctx.fillStyle = stringToColor(app.name);
      ctx.beginPath(); ctx.roundRect(scaledX, scaledY, scaledSize, scaledSize, scaledSize * 0.22); ctx.fill();
      ctx.fillStyle = "#fff";
      ctx.font = `bold ${Math.floor(scaledSize * 0.48)}px sans-serif`;
      ctx.textAlign = "center"; ctx.textBaseline = "middle";
      ctx.fillText(app.name.charAt(0).toUpperCase(), scaledX + scaledSize / 2, scaledY + scaledSize / 2);
    }
    if (running) {
      ctx.fillStyle = active ? "#ffffff" : "rgba(255,255,255,0.55)";
      ctx.beginPath(); ctx.arc(ix + iconSize / 2, dockY + dockH - 4, 2.5, 0, Math.PI * 2); ctx.fill();
    }
    regions.push({ exec: app.exec, bx: ix, by: dockY, bw: iconSize, bh: dockH });
    ix += iconSize + gap;
  }
  ctx.restore();
}

// ─── Deepin Dock ──────────────────────────────────────────────────────────────

function renderDeepinDock(
  ctx: CanvasRenderingContext2D,
  x: number, y: number, w: number, h: number,
  iconSize: number,
  apps: Array<{ exec: string; name: string; icon: string }>,
  regions: Array<{ exec: string; bx: number; by: number; bw: number; bh: number }>,
  bgColor = "#121a28", bgOpacity = 230, _radius = 14
) {
  const gap = 6;
  const pad = 10;
  const totalW = apps.length * (iconSize + gap) - gap + pad * 2;
  const dockX = x + (w - totalW) / 2;
  const dockY = y + 2;
  const dockH = h - 4;

  ctx.save();
  const r = parseInt(bgColor.slice(1, 3), 16) || 18;
  const g = parseInt(bgColor.slice(3, 5), 16) || 26;
  const b = parseInt(bgColor.slice(5, 7), 16) || 40;
  ctx.fillStyle = `rgba(${r},${g},${b},${(bgOpacity / 255).toFixed(3)})`;
  ctx.beginPath(); ctx.roundRect(dockX, dockY, totalW, dockH, 14); ctx.fill();
  ctx.strokeStyle = "rgba(32,140,255,0.35)"; ctx.lineWidth = 1; ctx.stroke();

  let ix = dockX + pad;
  const iy = dockY + (dockH - iconSize) / 2;

  for (const app of apps) {
    const hover = isHovering(ix, dockY, iconSize, dockH);
    const running = isAppRunning(app.exec, _runningClasses);
    const active = isAppActive(app.exec, _activeClass);

    if (active || hover) {
      ctx.fillStyle = active ? "rgba(32,140,255,0.22)" : "rgba(255,255,255,0.07)";
      ctx.beginPath(); ctx.roundRect(ix - 4, dockY + 3, iconSize + 8, dockH - 6, 8); ctx.fill();
    }

    const img = getLauncherIcon(app.icon);
    if (img) {
      ctx.save();
      ctx.beginPath(); ctx.roundRect(ix, iy, iconSize, iconSize, 10); ctx.clip();
      ctx.drawImage(img, ix, iy, iconSize, iconSize);
      ctx.restore();
    } else {
      ctx.fillStyle = stringToColor(app.name);
      ctx.beginPath(); ctx.roundRect(ix, iy, iconSize, iconSize, 10); ctx.fill();
      ctx.fillStyle = "#fff";
      ctx.font = `bold ${Math.floor(iconSize * 0.48)}px sans-serif`;
      ctx.textAlign = "center"; ctx.textBaseline = "middle";
      ctx.fillText(app.name.charAt(0).toUpperCase(), ix + iconSize / 2, iy + iconSize / 2);
    }
    if (running) {
      ctx.fillStyle = active ? "#208cff" : "rgba(255,255,255,0.5)";
      ctx.beginPath(); ctx.arc(ix + iconSize / 2, dockY + dockH - 4, 2.5, 0, Math.PI * 2); ctx.fill();
    }
    regions.push({ exec: app.exec, bx: ix, by: dockY, bw: iconSize, bh: dockH });
    ix += iconSize + gap;
  }
  ctx.restore();
}
