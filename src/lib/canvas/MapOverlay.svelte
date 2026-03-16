<!-- src/lib/canvas/MapOverlay.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { Layer } from "../types/project";

  interface Props {
    layer: Layer;
    baseScale: number;
    zoom: number;     // canvas editor zoom (1 = 100%)
    panX: number;     // canvas pan offset in project coords
    panY: number;
    canvasOffsetX: number;  // CSS px offset of canvas left edge within container
    canvasOffsetY: number;  // CSS px offset of canvas top edge within container
    containerW?: number;    // project canvas width (for anchor calculations)
    containerH?: number;    // project canvas height
    interactive?: boolean;  // false in editor mode so clicks pass through to canvas
  }
  let { layer, baseScale, zoom, panX, panY, canvasOffsetX, canvasOffsetY, containerW = 1920, containerH = 1080, interactive = true }: Props = $props();

  let mapEl: HTMLDivElement;
  let leafletMap: any = null;
  let radarLayer: any = null;
  let radarFrames: Array<{ time: number; path: string }> = [];
  let radarFrameIdx = 0;
  let animInterval: ReturnType<typeof setInterval> | null = null;

  // Derived: position of this overlay in CSS pixels within the container.
  // Must mirror anchorPosition() in renderer.ts exactly.
  function computeStyle() {
    const px = Number(layer.properties.x) || 0;
    const py = Number(layer.properties.y) || 0;
    const pw = Number(layer.properties.width) || 600;
    const ph = Number(layer.properties.height) || 400;
    const anchor = layer.properties.anchor ?? "top-left";

    // Anchor-adjusted top-left in project coords (same logic as anchorPosition())
    let ax = px, ay = py;
    switch (anchor) {
      case "center":        ax = containerW / 2 + px - pw / 2; ay = containerH / 2 + py - ph / 2; break;
      case "top-center":    ax = containerW / 2 + px - pw / 2; ay = py; break;
      case "top-right":     ax = containerW + px - pw;          ay = py; break;
      case "center-left":   ax = px;                            ay = containerH / 2 + py - ph / 2; break;
      case "center-right":  ax = containerW + px - pw;          ay = containerH / 2 + py - ph / 2; break;
      case "bottom-left":   ax = px;                            ay = containerH + py - ph; break;
      case "bottom-center": ax = containerW / 2 + px - pw / 2; ay = containerH + py - ph; break;
      case "bottom-right":  ax = containerW + px - pw;          ay = containerH + py - ph; break;
      // "top-left" / undefined: use raw px, py
    }

    const left = canvasOffsetX + (ax * zoom + panX) * baseScale;
    const top  = canvasOffsetY + (ay * zoom + panY) * baseScale;
    const width  = pw * zoom * baseScale;
    const height = ph * zoom * baseScale;

    // In editor mode pointer-events must be none so the canvas receives
    // mouse clicks for hit-testing / selection / drag.
    return `position:absolute;left:${left}px;top:${top}px;width:${width}px;height:${height}px;z-index:5;overflow:hidden;border-radius:8px;pointer-events:${interactive ? "auto" : "none"};`;
  }

  let overlayStyle = $derived(computeStyle());

  async function initLeaflet() {
    if (!mapEl || leafletMap) return;
    const L = (await import("leaflet")).default;
    // Import Leaflet CSS
    await import("leaflet/dist/leaflet.css");

    const props = layer.properties;
    leafletMap = L.map(mapEl, {
      center: [props.mapLat ?? 40.7, props.mapLng ?? -74.0],
      zoom: props.mapZoom ?? 5,
      zoomControl: false,
      attributionControl: false,
    });

    const tileStyle = props.mapStyle ?? "dark";
    let tileUrl = "https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png";
    if (tileStyle === "streets") {
      tileUrl = "https://tile.openstreetmap.org/{z}/{x}/{y}.png";
    } else if (tileStyle === "satellite") {
      tileUrl = "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}";
    }

    L.tileLayer(tileUrl, { maxZoom: 18 }).addTo(leafletMap);

    if (props.mapShowRadar) {
      await loadRadar(L);
    }
  }

  async function loadRadar(L: any) {
    if (!leafletMap) return;
    try {
      const res = await fetch("https://api.rainviewer.com/public/weather-maps.json");
      const data = await res.json();
      const frames = [
        ...(data.radar?.past ?? []),
        ...(data.radar?.nowcast ?? []),
      ];
      if (!frames.length) return;
      radarFrames = frames;
      radarFrameIdx = frames.length - 1;
      showRadarFrame(L, radarFrameIdx);

      if (layer.properties.mapRadarAnimate) {
        animInterval = setInterval(() => {
          radarFrameIdx = (radarFrameIdx + 1) % radarFrames.length;
          showRadarFrame(L, radarFrameIdx);
        }, 600);
      }
    } catch (e) {
      console.warn("RainViewer fetch failed:", e);
    }
  }

  function showRadarFrame(L: any, idx: number) {
    if (!leafletMap || !radarFrames[idx]) return;
    if (radarLayer) { radarLayer.remove(); radarLayer = null; }
    const frame = radarFrames[idx];
    const tileUrl = `https://tilecache.rainviewer.com${frame.path}/256/{z}/{x}/{y}/2/1_1.png`;
    radarLayer = L.tileLayer(tileUrl, { opacity: 0.65, zIndex: 10 });
    radarLayer.addTo(leafletMap);
  }

  onMount(async () => {
    // Wait one tick for the div to be in the DOM at its final size
    setTimeout(() => initLeaflet(), 50);
  });

  onDestroy(() => {
    if (animInterval) clearInterval(animInterval);
    if (leafletMap) { leafletMap.remove(); leafletMap = null; }
  });

  // Invalidate map size when dimensions change
  $effect(() => {
    // Track all positional props to trigger reactive update
    void layer.properties.x;
    void layer.properties.y;
    void layer.properties.width;
    void layer.properties.height;
    void baseScale;
    void zoom;
    if (leafletMap) {
      setTimeout(() => leafletMap?.invalidateSize(), 10);
    }
  });
</script>

<div bind:this={mapEl} style={overlayStyle}></div>
