import type { Project, Layer } from "../types/project";
import { resolveFormula, hasFormula } from "../formula/service";
import { computeAnimatedDeltas } from "./animationEngine";
import { initEngineTime, markLayerSeen } from "./animationState";

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

// Image cache to avoid reloading every frame
const imageCache = new Map<string, HTMLImageElement>();
const imageLoadingSet = new Set<string>();
const imageFailedMap = new Map<string, number>(); // URL -> failure timestamp
const IMAGE_RETRY_MS = 5000; // Retry failed images after 5 seconds

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

  return src;
}

function getCachedImage(src: string): HTMLImageElement | null {
  const resolvedPath = resolveImagePath(src);
  if (!resolvedPath) return null;

  let resolved: string;
  if (resolvedPath.startsWith("http") || resolvedPath.startsWith("data:")) {
    resolved = resolvedPath;
  } else if (convertFileSrc) {
    resolved = convertFileSrc(resolvedPath);
  } else {
    // Non-Tauri context (WebKitGTK wallpaper): serve via Vite asset proxy
    resolved = `/__klwp_assets${resolvedPath}`;
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
  img.crossOrigin = "anonymous";
  img.onload = () => {
    imageCache.set(resolved, img);
    imageLoadingSet.delete(resolved);
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

export function renderProject(ctx: CanvasRenderingContext2D, project: Project, selectedId: string | null, timestamp: number = 0) {
  initEngineTime(timestamp);
  const { width, height } = project.resolution;
  const container: ContainerSize = { width, height };

  currentAssetDir = project.assetDir;
  computedBounds = new Map();

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
    if (layer.visible === false) continue;
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
  // Formula-driven visibility: "always", "never", "remove", or a formula
  const resolved = resolve(vis as any, "always");
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
  const opacity = (resolveNumber(props.opacity, 255) / 255) * deltas.opacityMultiplier;
  const rotation = resolveNumber(props.rotation, 0) + deltas.dRotation;

  // Anchor-based positioning (local to container)
  const { x, y } = anchorPosition(offsetX, offsetY, w, h, props.anchor, container);

  // Store absolute bounds for selection outline and hit testing
  computedBounds.set(layer.id, { x: parentAbsX + x, y: parentAbsY + y, w: w || 100, h: h || 100 });

  ctx.globalAlpha *= opacity;

  if (deltas.blur > 0) {
    ctx.filter = `blur(${deltas.blur}px)`;
  }

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

  switch (layer.type) {
    case "text":
      renderText(ctx, layer, x, y, w, h);
      break;
    case "shape":
      renderShape(ctx, layer, x, y, w, h);
      break;
    case "image":
      renderImage(ctx, layer, x, y, w, h);
      break;
    case "progress":
      renderProgress(ctx, layer, x, y, w, h);
      break;
    case "overlap":
    case "group":
      renderOverlap(ctx, layer, x, y, container, parentAbsX, parentAbsY, timestamp);
      break;
    case "stack":
      renderStack(ctx, layer, x, y, container, parentAbsX, parentAbsY, timestamp);
      break;
    case "fonticon":
      renderFontIcon(ctx, layer, x, y, w, h);
      break;
  }

  // Color animation overlay: tint the already-drawn layer content
  if (deltas.colorOverride) {
    ctx.globalCompositeOperation = "source-atop";
    ctx.fillStyle = deltas.colorOverride;
    ctx.fillRect(x, y, w, h);
    ctx.globalCompositeOperation = "source-over";
  }

  ctx.restore();
}

function renderText(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, _w: number, _h: number) {
  const props = layer.properties;
  const fontSize = resolveNumber(props.fontSize, 24);
  const fontFamily = props.fontFamily || "sans-serif";
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
  if (align === "center") textX = x + resolveNumber(props.width, 100) / 2;
  else if (align === "right") textX = x + resolveNumber(props.width, 100);

  // Multi-line rendering
  const lines = text.split("\n");
  const maxLines = resolveNumber(props.maxLines, 0);
  const displayLines = maxLines > 0 ? lines.slice(0, maxLines) : lines;
  const lineSpacing = resolveNumber(props.lineSpacing, 0);
  const lineHeight = fontSize + lineSpacing;

  for (let i = 0; i < displayLines.length; i++) {
    ctx.fillText(displayLines[i], textX, y + i * lineHeight);
  }

  // Update computed bounds to reflect multi-line height
  const totalHeight = displayLines.length * lineHeight - lineSpacing;
  const boundsEntry = computedBounds.get(layer.id);
  if (boundsEntry && totalHeight > boundsEntry.h) {
    boundsEntry.h = totalHeight;
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
  const fill = resolve(props.fill, "#e94560");
  const stroke = props.stroke ? resolve(props.stroke) : undefined;
  const strokeWidth = resolveNumber(props.strokeWidth, 0);
  const cornerRadius = resolveNumber(props.cornerRadius, 0);

  ctx.fillStyle = fill;
  if (stroke) {
    ctx.strokeStyle = stroke;
    ctx.lineWidth = strokeWidth;
  }

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
      ctx.fill();
      if (stroke) ctx.stroke();
      break;

    case "circle": {
      const radius = Math.min(w, h) / 2;
      ctx.beginPath();
      ctx.arc(x + w / 2, y + h / 2, radius, 0, Math.PI * 2);
      ctx.fill();
      if (stroke) ctx.stroke();
      break;
    }

    case "oval":
      ctx.beginPath();
      ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
      ctx.fill();
      if (stroke) ctx.stroke();
      break;

    case "triangle":
      ctx.beginPath();
      ctx.moveTo(x + w / 2, y);
      ctx.lineTo(x + w, y + h);
      ctx.lineTo(x, y + h);
      ctx.closePath();
      ctx.fill();
      if (stroke) ctx.stroke();
      break;

    case "arc": {
      const r = Math.min(w, h) / 2;
      ctx.beginPath();
      ctx.arc(x + w / 2, y + h / 2, r, 0, Math.PI);
      ctx.fill();
      if (stroke) ctx.stroke();
      break;
    }
  }

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
  } else if (convertFileSrc) {
    resolved = convertFileSrc(resolvedPath);
  } else {
    resolved = `/__klwp_assets${resolvedPath}`;
  }
  if (imageFailedMap.has(resolved)) return "Failed to load";
  return "Loading...";
}

function renderImage(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const src = resolve(layer.properties.src, "");
  const img = getCachedImage(src);

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

  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, w, h);
  ctx.clip();

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

  ctx.drawImage(img, drawX, drawY, drawW, drawH);

  // Apply tint color overlay over the drawn image
  const tint = layer.properties.tint;
  if (tint && tint !== "#ffffff" && tint !== "#FFFFFF") {
    ctx.globalCompositeOperation = "source-atop";
    ctx.fillStyle = resolve(tint, "#ffffff");
    ctx.globalAlpha = 0.4;
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

function renderFontIcon(ctx: CanvasRenderingContext2D, layer: Layer, x: number, y: number, w: number, h: number) {
  const props = layer.properties;
  const glyphCode = props.glyphCode || "e88a";
  const color = resolve(props.color, "#ffffff");
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

  ctx.font = `${fontSize}px "${fontFamily}", sans-serif`;
  ctx.fillStyle = color;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(char, x + w / 2, y + h / 2);
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
  for (const child of layer.children) {
    renderLayer(ctx, child, childContainer, absX, absY, timestamp);
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
    if (child.visible === false) continue;
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
