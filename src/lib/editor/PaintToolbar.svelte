<script lang="ts">
  import { getPaintBrushSettings, setPaintBrushSettings, setToolMode } from "../stores/project.svelte";
  import { brushPresets } from "../data/brushPresets";
  import type { BrushType } from "../types/paint";

  const brushTypes: { type: BrushType; label: string }[] = [
    { type: "solid", label: "Solid" },
    { type: "spray", label: "Spray" },
    { type: "airbrush", label: "Air" },
    { type: "splatter", label: "Splat" },
  ];

  function selectBrushType(type: BrushType) {
    setPaintBrushSettings({ brushType: type });
  }

  function onSizeChange(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    if (!isNaN(val)) setPaintBrushSettings({ brushSize: val });
  }

  function onOpacityChange(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    if (!isNaN(val)) setPaintBrushSettings({ opacity: val / 100 });
  }

  function onColorChange(e: Event) {
    setPaintBrushSettings({ color: (e.target as HTMLInputElement).value });
  }

  function applyPreset(preset: typeof brushPresets[0]) {
    setPaintBrushSettings({
      brushType: preset.type,
      brushSize: preset.size,
      opacity: preset.opacity,
    });
  }

  function done() {
    setToolMode("select");
  }
</script>

<div style="display:flex;align-items:center;gap:10px;padding:4px 12px;background:#1a1a2e;border-bottom:1px solid #333;height:36px;flex-shrink:0;user-select:none;overflow-x:auto;">
  <!-- Brush type buttons -->
  {#each brushTypes as bt}
    <span
      style="padding:3px 8px;border-radius:3px;font-size:11px;cursor:pointer;white-space:nowrap;
        background:{getPaintBrushSettings().brushType === bt.type ? '#4a9eff' : '#2a2a3e'};
        color:{getPaintBrushSettings().brushType === bt.type ? '#fff' : '#aab'};
        border:1px solid {getPaintBrushSettings().brushType === bt.type ? '#4a9eff' : '#444'};"
      onclick={() => selectBrushType(bt.type)}
    >{bt.label}</span>
  {/each}

  <span style="width:1px;height:20px;background:#444;flex-shrink:0;"></span>

  <!-- Size -->
  <span style="font-size:10px;color:#888;white-space:nowrap;">Size</span>
  <input
    type="range" min="1" max="100"
    value={getPaintBrushSettings().brushSize}
    oninput={onSizeChange}
    style="width:80px;accent-color:#4a9eff;"
  />
  <span style="font-size:10px;color:#aab;min-width:20px;">{getPaintBrushSettings().brushSize}</span>

  <span style="width:1px;height:20px;background:#444;flex-shrink:0;"></span>

  <!-- Opacity -->
  <span style="font-size:10px;color:#888;white-space:nowrap;">Opacity</span>
  <input
    type="range" min="1" max="100"
    value={Math.round(getPaintBrushSettings().opacity * 100)}
    oninput={onOpacityChange}
    style="width:80px;accent-color:#4a9eff;"
  />
  <span style="font-size:10px;color:#aab;min-width:26px;">{Math.round(getPaintBrushSettings().opacity * 100)}%</span>

  <span style="width:1px;height:20px;background:#444;flex-shrink:0;"></span>

  <!-- Color -->
  <input
    type="color"
    value={getPaintBrushSettings().color}
    oninput={onColorChange}
    style="width:28px;height:22px;padding:0;border:1px solid #555;border-radius:3px;cursor:pointer;background:transparent;"
  />

  <span style="width:1px;height:20px;background:#444;flex-shrink:0;"></span>

  <!-- Presets -->
  {#each brushPresets as preset}
    <span
      style="padding:2px 6px;border-radius:3px;font-size:10px;cursor:pointer;white-space:nowrap;background:#2a2a3e;color:#aab;border:1px solid #444;"
      title="{preset.name} ({preset.type}, {preset.size}px)"
      onclick={() => applyPreset(preset)}
    >{preset.name}</span>
  {/each}

  <span style="width:1px;height:20px;background:#444;flex-shrink:0;"></span>

  <!-- Done button -->
  <span
    style="padding:3px 10px;border-radius:3px;font-size:11px;font-weight:600;cursor:pointer;background:#2d7d46;color:#fff;border:1px solid #3a9a58;white-space:nowrap;"
    onclick={done}
  >Done</span>
</div>
