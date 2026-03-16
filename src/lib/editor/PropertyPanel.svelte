<script lang="ts">
  import { getSelectedLayer, updateLayerProperty, getProject } from "../stores/project.svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import FormulaHelper from "./FormulaHelper.svelte";
  import AnimationPanel from "./AnimationPanel.svelte";
  import IconPicker from "./IconPicker.svelte";
  import ColorField from "./ColorField.svelte";
  import { SYSTEM_FONTS } from "../data/fonts";
  import { getProjectFontNames, loadFont } from "../fonts/fontLoader";

  let formulaHelperOpen = $state(false);
  let iconPickerOpen = $state(false);
  let showCustomFont = $state(false);
  let appPickerOpen = $state(false);
  let appPickerSearch = $state("");
  let appPickerForPinned = $state(false);
  let appList = $state<{ name: string; exec: string; icon: string; categories: string }[]>([]);

  async function openAppPicker(forPinned = false) {
    appPickerForPinned = forPinned;
    appPickerOpen = true;
    if (appList.length === 0) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        appList = await invoke<typeof appList>("list_apps");
      } catch (e) {
        console.warn("list_apps failed:", e);
      }
    }
  }

  function selectApp(exec: string) {
    const layer = getSelectedLayer();
    if (!layer) { appPickerOpen = false; appPickerSearch = ""; return; }
    if (appPickerForPinned) {
      const current = (layer.properties.pinnedApps ?? []) as string[];
      if (!current.includes(exec)) {
        updateLayerProperty(layer.id, "pinnedApps", [...current, exec]);
      }
      appPickerSearch = ""; // keep picker open to add more
    } else {
      updateLayerProperty(layer.id, "clickAction", "app:" + exec);
      appPickerOpen = false;
      appPickerSearch = "";
    }
  }

  function removePinnedApp(exec: string) {
    const layer = getSelectedLayer();
    if (!layer) return;
    const current = (layer.properties.pinnedApps ?? []) as string[];
    updateLayerProperty(layer.id, "pinnedApps", current.filter(e => e !== exec));
  }

  function getIconSrc(iconPath: string): string {
    if (!iconPath) return "";
    // list_apps now returns resolved absolute paths — convert to asset URL
    try { return convertFileSrc(iconPath); } catch { return iconPath; }
  }

  function iconLetter(name: string): string { return name.charAt(0).toUpperCase(); }

  function iconBg(name: string): string {
    let h = 0;
    for (let i = 0; i < name.length; i++) h = name.charCodeAt(i) + ((h << 5) - h);
    return `hsl(${Math.abs(h) % 360},40%,36%)`;
  }

  // Derived state for click action UI
  let clickActionType = $derived.by(() => {
    const ca = getSelectedLayer()?.properties.clickAction ?? "";
    if (!ca) return "none";
    const prefix = ca.split(":")[0];
    return ["music", "app", "overlay", "set", "url"].includes(prefix) ? prefix : "none";
  });

  let clickActionArg = $derived.by(() => {
    const ca = getSelectedLayer()?.properties.clickAction ?? "";
    const idx = ca.indexOf(":");
    return idx >= 0 ? ca.slice(idx + 1) : "";
  });

  function handleFormulaInsert(formula: string) {
    const layer = getSelectedLayer();
    if (!layer) return;
    const current = String(layer.properties.text ?? "");
    updateLayerProperty(layer.id, "text", current + formula);
  }

  function getImageSrc(src: string): string {
    if (!src) return "";
    if (src.startsWith("http://") || src.startsWith("https://") || src.startsWith("data:")) {
      return src;
    }
    return convertFileSrc(src);
  }

  async function handleBrowseImage() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const path = await open({
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"] }],
        multiple: false,
      });
      if (path) {
        const layer = getSelectedLayer();
        if (layer) {
          updateLayerProperty(layer.id, "src", path);
        }
      }
    } catch (e) {
      console.error("Browse failed:", e);
    }
  }

  function onInput(key: string, e: Event) {
    const layer = getSelectedLayer();
    if (!layer) return;
    const target = e.target as HTMLInputElement | HTMLSelectElement;
    const raw = target.value;
    // Try to parse as number for numeric fields
    const numericKeys = ["x", "y", "width", "height", "rotation", "opacity", "fontSize", "strokeWidth", "cornerRadius", "min", "max", "value", "lineSpacing", "maxLines", "spacing", "scaleX", "scaleY", "barCount", "barSpacing", "sensitivity", "mapLat", "mapLng", "mapZoom", "launcherIconSize", "radarDotSize", "radarRingCount", "taskbarBgOpacity", "taskbarRadius"];
    if (numericKeys.includes(key)) {
      const num = Number(raw);
      updateLayerProperty(layer.id, key, isNaN(num) ? raw : num);
    } else {
      updateLayerProperty(layer.id, key, raw);
    }
  }

  function onSelectInput(key: string, e: Event) {
    const layer = getSelectedLayer();
    if (!layer) return;
    const target = e.target as HTMLSelectElement;
    updateLayerProperty(layer.id, key, target.value);
  }

  async function handleImportFont() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await open({
        filters: [{ name: "Fonts", extensions: ["ttf", "otf", "woff", "woff2"] }],
        multiple: false,
      });
      if (path) {
        const assetDir = getProject().assetDir;
        if (!assetDir) {
          alert("Save the project first to import fonts.");
          return;
        }
        const savedPath = await invoke<string>("copy_asset_to_project", { sourcePath: path, assetDir, subfolder: "fonts" });
        const name = String(path).split("/").pop()?.replace(/\.(ttf|otf|woff2?)$/i, "") ?? "Custom";
        await loadFont(name, savedPath);
        const layer = getSelectedLayer();
        if (layer) updateLayerProperty(layer.id, "fontFamily", name);
      }
    } catch (e) {
      console.error("Font import failed:", e);
    }
  }

  function handleIconSelect(iconSet: string, glyphCode: string, iconSrc?: string) {
    const layer = getSelectedLayer();
    if (!layer) return;
    if (iconSrc) {
      updateLayerProperty(layer.id, "iconSrc", iconSrc);
    }
    if (glyphCode) {
      updateLayerProperty(layer.id, "glyphCode", glyphCode);
    }
    if (iconSet && iconSet !== "apk") {
      updateLayerProperty(layer.id, "iconSet", iconSet);
    }
    iconPickerOpen = false;
  }

  function onShadowInput(field: "color" | "dx" | "dy" | "radius", e: Event) {
    const layer = getSelectedLayer();
    if (!layer) return;
    const target = e.target as HTMLInputElement;
    const existing = layer.properties.shadow ?? { color: "#00000080", dx: 2, dy: 2, radius: 4 };
    const updated = { ...existing };
    if (field === "color") {
      updated.color = target.value;
    } else {
      const num = Number(target.value);
      updated[field] = isNaN(num) ? 0 : num;
    }
    updateLayerProperty(layer.id, "shadow", updated);
  }

  function onColorProp(field: string, value: string) {
    const layer = getSelectedLayer();
    if (!layer) return;
    updateLayerProperty(layer.id, field, value);
  }
</script>

<div class="property-panel">
  <div class="panel-header">Properties</div>
  <div class="panel-body">
    {#if getSelectedLayer()}
      {@const layer = getSelectedLayer()!}
      {@const props = layer.properties}

      <section class="prop-section">
        <div class="section-title">Transform</div>
        <div class="prop-grid">
          <label>X</label>
          <input type="number" value={props.x} oninput={(e) => onInput("x", e)} />
          <label>Y</label>
          <input type="number" value={props.y} oninput={(e) => onInput("y", e)} />
          <label>W</label>
          <input type="number" value={props.width} oninput={(e) => onInput("width", e)} />
          <label>H</label>
          <input type="number" value={props.height} oninput={(e) => onInput("height", e)} />
          <label>Rot</label>
          <input type="number" value={props.rotation ?? 0} oninput={(e) => onInput("rotation", e)} />
          <label>Alpha</label>
          <input type="number" min="0" max="255" value={props.opacity ?? 255} oninput={(e) => onInput("opacity", e)} />
          <label>Anchor</label>
          <select value={props.anchor ?? "top-left"} onchange={(e) => onSelectInput("anchor", e)}>
            <option value="center">Center</option>
            <option value="top-left">Top Left</option>
            <option value="top-center">Top Center</option>
            <option value="top-right">Top Right</option>
            <option value="center-left">Center Left</option>
            <option value="center-right">Center Right</option>
            <option value="bottom-left">Bottom Left</option>
            <option value="bottom-center">Bottom Center</option>
            <option value="bottom-right">Bottom Right</option>
          </select>
          <label>SclX</label>
          <input type="number" step="0.1" value={props.scaleX ?? 1} oninput={(e) => onInput("scaleX", e)} />
          <label>SclY</label>
          <input type="number" step="0.1" value={props.scaleY ?? 1} oninput={(e) => onInput("scaleY", e)} />
        </div>
        <div class="prop-stack" style="margin-top: 6px;">
          <label>Visible (formula)</label>
          <input type="text" value={props.visible ?? ""} placeholder="e.g. $gv(myvar)$" oninput={(e) => onInput("visible", e)} />
          <label>Click Action</label>
          <select
            value={clickActionType}
            onchange={(e) => {
              const newType = (e.target as HTMLSelectElement).value;
              const layer = getSelectedLayer()!;
              if (newType === "none") {
                updateLayerProperty(layer.id, "clickAction", undefined);
              } else {
                updateLayerProperty(layer.id, "clickAction", newType + ":");
              }
            }}
          >
            <option value="none">None</option>
            <option value="music">Music Control</option>
            <option value="app">Launch App</option>
            <option value="overlay">Toggle Layer</option>
            <option value="set">Set Variable</option>
            <option value="url">Open URL</option>
          </select>
          {#if clickActionType === "music"}
            <select
              value={clickActionArg}
              onchange={(e) => {
                const newArg = (e.target as HTMLSelectElement).value;
                const layer = getSelectedLayer()!;
                updateLayerProperty(layer.id, "clickAction", "music:" + newArg);
              }}
            >
              <option value="play-pause">Play / Pause</option>
              <option value="next">Next Track</option>
              <option value="previous">Previous Track</option>
              <option value="play">Play</option>
              <option value="pause">Pause</option>
              <option value="stop">Stop</option>
            </select>
          {:else if clickActionType === "app"}
            <div class="input-with-browse">
              <input type="text" value={clickActionArg} placeholder="e.g. spotify, firefox, vlc" oninput={(e) => {
                const newArg = (e.target as HTMLInputElement).value;
                const layer = getSelectedLayer()!;
                updateLayerProperty(layer.id, "clickAction", "app:" + newArg);
              }} />
              <button class="browse-btn" title="Browse installed apps" onclick={openAppPicker}>...</button>
            </div>
          {:else if clickActionType === "overlay"}
            <input type="text" value={clickActionArg} placeholder="Layer name to show/hide" oninput={(e) => {
              const newArg = (e.target as HTMLInputElement).value;
              const layer = getSelectedLayer()!;
              updateLayerProperty(layer.id, "clickAction", "overlay:" + newArg);
            }} />
            <span class="prop-hint">Toggles visibility of the named layer</span>
          {:else if clickActionType === "set"}
            <input type="text" value={clickActionArg} placeholder="varName:value" oninput={(e) => {
              const newArg = (e.target as HTMLInputElement).value;
              const layer = getSelectedLayer()!;
              updateLayerProperty(layer.id, "clickAction", "set:" + newArg);
            }} />
          {:else if clickActionType === "url"}
            <input type="text" value={clickActionArg} placeholder="https://... or $gv(urlVar)$" oninput={(e) => {
              const newArg = (e.target as HTMLInputElement).value;
              const layer = getSelectedLayer()!;
              updateLayerProperty(layer.id, "clickAction", "url:" + newArg);
            }} />
          {/if}
          <label>Scroll Action</label>
          <select
            value={getSelectedLayer()?.properties.scrollAction ?? "none"}
            onchange={(e) => {
              const val = (e.target as HTMLSelectElement).value;
              const layer = getSelectedLayer()!;
              updateLayerProperty(layer.id, "scrollAction", val === "none" ? undefined : val);
            }}
          >
            <option value="none">None</option>
            <option value="volume:adjust">Volume</option>
            <option value="brightness:adjust">Brightness</option>
          </select>
        </div>
      </section>

      {#if layer.type === "text"}
        <section class="prop-section">
          <div class="section-title">Text</div>
          <div class="prop-stack">
            <label>Content</label>
            <div class="input-row">
              <input type="text" value={props.text ?? ""} oninput={(e) => onInput("text", e)} />
              <button class="fx-btn" title="Formula Helper" onclick={() => formulaHelperOpen = true}>fx</button>
            </div>
            <label>Font Size</label>
            <input type="number" value={props.fontSize ?? 24} oninput={(e) => onInput("fontSize", e)} />
            <label>Font Family</label>
            {#if showCustomFont}
              <div class="input-row">
                <input type="text" value={props.fontFamily ?? "sans-serif"} oninput={(e) => onInput("fontFamily", e)} placeholder="Custom font name" />
                <button class="browse-btn" title="Back to list" onclick={() => showCustomFont = false}>List</button>
              </div>
            {:else}
              <div class="input-row">
                <select value={props.fontFamily ?? "sans-serif"} onchange={(e) => onSelectInput("fontFamily", e)}>
                  <optgroup label="System Fonts">
                    {#each SYSTEM_FONTS as font}
                      <option value={font.family}>{font.name}</option>
                    {/each}
                  </optgroup>
                  {#if getProjectFontNames().length > 0}
                    <optgroup label="Project Fonts">
                      {#each getProjectFontNames() as name}
                        <option value={name}>{name}</option>
                      {/each}
                    </optgroup>
                  {/if}
                </select>
                <button class="browse-btn" title="Type custom font" onclick={() => showCustomFont = true}>...</button>
              </div>
            {/if}
            <button class="browse-btn" style="margin-top: 2px; width: 100%; text-align: center;" title="Import font file (.ttf, .otf, .woff2)" onclick={handleImportFont}>Import Font</button>
            <label>Color</label>
            <ColorField value={String(props.color ?? "#ffffff")} defaultColor="#ffffff" onChange={(v) => onColorProp("color", v)} />
            <label>Align</label>
            <select value={props.textAlign ?? "left"} onchange={(e) => onSelectInput("textAlign", e)}>
              <option value="left">Left</option>
              <option value="center">Center</option>
              <option value="right">Right</option>
            </select>
            <label>Max Lines</label>
            <input type="number" min="0" value={props.maxLines ?? 0} placeholder="0 = unlimited" oninput={(e) => onInput("maxLines", e)} />
            <label>Line Spacing</label>
            <input type="number" value={props.lineSpacing ?? 0} oninput={(e) => onInput("lineSpacing", e)} />
            <label style="margin-top: 6px; font-weight: 600;">Shadow</label>
            <label>Color</label>
            <input type="color" value={props.shadow?.color ?? "#00000080"} oninput={(e) => onShadowInput("color", e)} />
            <label>DX</label>
            <input type="number" value={props.shadow?.dx ?? 2} oninput={(e) => onShadowInput("dx", e)} />
            <label>DY</label>
            <input type="number" value={props.shadow?.dy ?? 2} oninput={(e) => onShadowInput("dy", e)} />
            <label>Radius</label>
            <input type="number" min="0" value={props.shadow?.radius ?? 4} oninput={(e) => onShadowInput("radius", e)} />
          </div>
        </section>
      {/if}

      {#if layer.type === "shape"}
        <section class="prop-section">
          <div class="section-title">Shape</div>
          <div class="prop-stack">
            <label>Kind</label>
            <select value={props.shapeKind ?? "rectangle"} onchange={(e) => onSelectInput("shapeKind", e)}>
              <option value="rectangle">Rectangle</option>
              <option value="circle">Circle</option>
              <option value="oval">Oval</option>
              <option value="triangle">Triangle</option>
              <option value="arc">Arc</option>
            </select>
            <label>Fill</label>
            <div class="input-row">
              <ColorField value={String(props.fill ?? "#e94560")} defaultColor="#e94560" onChange={(v) => onColorProp("fill", v)} />
              <button class="browse-btn" title="No fill (transparent)" onclick={() => { const l = getSelectedLayer(); if (l) updateLayerProperty(l.id, "fill", "none"); }} style="font-size:10px; width:auto; padding:2px 6px;">No Fill</button>
            </div>
            {#if props.fill === "none" || props.fill === "transparent"}
              <span class="prop-hint">Fill is transparent — stroke only</span>
            {/if}
            <label>Stroke</label>
            <ColorField value={String(props.stroke ?? "#000000")} defaultColor="#000000" onChange={(v) => onColorProp("stroke", v)} />
            <label>Stroke Width</label>
            <input type="number" value={props.strokeWidth ?? 0} oninput={(e) => onInput("strokeWidth", e)} />
            <label>Corner Radius</label>
            <input type="number" value={props.cornerRadius ?? 0} oninput={(e) => onInput("cornerRadius", e)} />
            <label style="margin-top: 6px; font-weight: 600;">Skew / Perspective</label>
            <label>Skew X (°)</label>
            <input type="number" value={props.skewX ?? 0} step="1" oninput={(e) => onInput("skewX", e)} />
            <label>Skew Y (°)</label>
            <input type="number" value={props.skewY ?? 0} step="1" oninput={(e) => onInput("skewY", e)} />
            <label style="margin-top: 6px; font-weight: 600;">Shadow</label>
            <label>Color</label>
            <input type="color" value={props.shadow?.color ?? "#00000080"} oninput={(e) => onShadowInput("color", e)} />
            <label>DX</label>
            <input type="number" value={props.shadow?.dx ?? 2} oninput={(e) => onShadowInput("dx", e)} />
            <label>DY</label>
            <input type="number" value={props.shadow?.dy ?? 2} oninput={(e) => onShadowInput("dy", e)} />
            <label>Radius</label>
            <input type="number" min="0" value={props.shadow?.radius ?? 4} oninput={(e) => onShadowInput("radius", e)} />
          </div>
        </section>
      {/if}

      {#if layer.type === "image"}
        <section class="prop-section">
          <div class="section-title">Image</div>
          <div class="prop-stack">
            <label>Source</label>
            <div class="input-row">
              <input type="text" value={props.src ?? ""} placeholder="Path, URL, or $mi(cover)$" oninput={(e) => onInput("src", e)} />
              <button class="browse-btn" title="Browse for image" onclick={handleBrowseImage}>...</button>
            </div>
            <span class="prop-hint">Tip: use <code>$mi(cover)$</code> for album art</span>
            {#if props.src}
              <div class="image-preview-small">
                <img src={getImageSrc(String(props.src))} alt="preview" />
              </div>
            {/if}
            <label>Scale Mode</label>
            <select value={props.scaleMode ?? "fit"} onchange={(e) => onSelectInput("scaleMode", e)}>
              <option value="fit">Fit</option>
              <option value="fill">Fill</option>
              <option value="crop">Crop</option>
              <option value="stretch">Stretch</option>
            </select>
            <label>Corner Radius</label>
            <input type="number" min="0" value={props.cornerRadius ?? 0} oninput={(e) => onInput("cornerRadius", e)} />
            <label>Tint</label>
            <ColorField value={String(props.tint ?? "")} defaultColor="#ffffff" onChange={(v) => onColorProp("tint", v)} />
            <label style="margin-top: 6px; font-weight: 600;">Shadow</label>
            <label>Color</label>
            <input type="color" value={props.shadow?.color ?? "#00000080"} oninput={(e) => onShadowInput("color", e)} />
            <label>DX</label>
            <input type="number" value={props.shadow?.dx ?? 2} oninput={(e) => onShadowInput("dx", e)} />
            <label>DY</label>
            <input type="number" value={props.shadow?.dy ?? 2} oninput={(e) => onShadowInput("dy", e)} />
            <label>Radius</label>
            <input type="number" min="0" value={props.shadow?.radius ?? 4} oninput={(e) => onShadowInput("radius", e)} />
          </div>
        </section>
      {/if}

      {#if layer.type === "progress"}
        <section class="prop-section">
          <div class="section-title">Progress</div>
          <div class="prop-stack">
            <label>Style</label>
            <select value={props.style ?? "arc"} onchange={(e) => onSelectInput("style", e)}>
              <option value="arc">Arc</option>
              <option value="bar">Bar</option>
              <option value="circle">Circle</option>
            </select>
            <label>Min</label>
            <input type="number" value={props.min ?? 0} oninput={(e) => onInput("min", e)} />
            <label>Max</label>
            <input type="number" value={props.max ?? 100} oninput={(e) => onInput("max", e)} />
            <label>Value</label>
            <input type="text" value={props.value ?? 50} placeholder="0–100 or $mi(percent)$" oninput={(e) => onInput("value", e)} />
            <span class="prop-hint">Tip: <code>$mi(percent)$</code> = music progress, <code>$bi(level)$</code> = battery</span>
            <label>Color</label>
            <ColorField value={String(props.color ?? "#e94560")} defaultColor="#e94560" onChange={(v) => onColorProp("color", v)} />
            <label>Track Color</label>
            <ColorField value={String(props.trackColor ?? "#333333")} defaultColor="#333333" onChange={(v) => onColorProp("trackColor", v)} />
            <label>Stroke Width</label>
            <input type="number" value={props.strokeWidth ?? 6} oninput={(e) => onInput("strokeWidth", e)} />
          </div>
        </section>
      {/if}

      {#if layer.type === "stack"}
        <section class="prop-section">
          <div class="section-title">Stack</div>
          <div class="prop-stack">
            <label>Orientation</label>
            <select value={props.orientation ?? "vertical"} onchange={(e) => onSelectInput("orientation", e)}>
              <option value="vertical">Vertical</option>
              <option value="horizontal">Horizontal</option>
            </select>
            <label>Spacing</label>
            <input type="number" value={props.spacing ?? 0} oninput={(e) => onInput("spacing", e)} />
            <label>Width</label>
            <input type="number" value={props.width} oninput={(e) => onInput("width", e)} />
            <label>Height</label>
            <input type="number" value={props.height} oninput={(e) => onInput("height", e)} />
            <div class="child-count">Children: {layer.children?.length ?? 0}</div>
          </div>
        </section>
      {/if}

      {#if layer.type === "overlap"}
        <section class="prop-section">
          <div class="section-title">Overlap</div>
          <div class="prop-stack">
            <label>Width</label>
            <input type="number" value={props.width} oninput={(e) => onInput("width", e)} />
            <label>Height</label>
            <input type="number" value={props.height} oninput={(e) => onInput("height", e)} />
            <div class="child-count">Children: {layer.children?.length ?? 0}</div>
          </div>
        </section>
      {/if}

      {#if layer.type === "visualizer"}
        <section class="prop-section">
          <div class="section-title">Visualizer</div>
          <div class="prop-stack">
            <label>Style</label>
            <select value={props.vizStyle ?? "bars"} onchange={(e) => updateLayerProperty(layer.id, "vizStyle", (e.target as HTMLSelectElement).value)}>
              <option value="bars">Bars</option>
              <option value="wave">Wave</option>
            </select>
            <label>{(props.vizStyle ?? "bars") === "wave" ? "Points" : "Bars"}</label>
            <input type="number" min="4" max="64" value={props.barCount ?? 24} oninput={(e) => onInput("barCount", e)} />
            <label>Spacing</label>
            <input type="number" min="0" max="20" value={props.barSpacing ?? 3} oninput={(e) => onInput("barSpacing", e)} />
            <label>Sensitivity</label>
            <input type="number" min="0.1" max="5" step="0.1" value={props.sensitivity ?? 1} oninput={(e) => onInput("sensitivity", e)} />
            <label>Top Color</label>
            <ColorField value={String(props.colorTop ?? "#88C0D0")} defaultColor="#88C0D0" onChange={(v) => onColorProp("colorTop", v)} />
            <label>Mid Color</label>
            <ColorField value={String(props.colorMid ?? "#5E81AC")} defaultColor="#5E81AC" onChange={(v) => onColorProp("colorMid", v)} />
            <label>Base Color</label>
            <ColorField value={String(props.colorBottom ?? "#2E3440")} defaultColor="#2E3440" onChange={(v) => onColorProp("colorBottom", v)} />
            <label>Peak Color</label>
            <ColorField value={String(props.peakColor ?? "#ECEFF4")} defaultColor="#ECEFF4" onChange={(v) => onColorProp("peakColor", v)} />
            <span class="prop-hint">Requires <code>parec</code> (PulseAudio/PipeWire). Reacts to playing audio.</span>
          </div>
        </section>
      {/if}

      {#if layer.type === "fonticon"}
        <section class="prop-section">
          <div class="section-title">Font Icon</div>
          <div class="prop-stack">
            <button class="browse-btn" style="width: 100%; text-align: center; padding: 6px; font-weight: 600;" onclick={() => iconPickerOpen = true}>Choose Icon</button>
            {#if props.iconSrc}
              <div class="image-preview-small">
                <img src={getImageSrc(String(props.iconSrc))} alt="icon preview" />
              </div>
            {:else if props.glyphCode}
              <div class="icon-preview">
                {#if (props.iconSet ?? "material") === "material"}
                  <span style="font-family: 'Material Icons'; font-size: 36px; color: {props.color ?? '#ffffff'};">{String.fromCodePoint(parseInt(props.glyphCode, 16) || 0x3f)}</span>
                {:else}
                  <span style="font-family: 'Font Awesome 6 Free'; font-weight: 900; font-size: 32px; color: {props.color ?? '#ffffff'};">{String.fromCodePoint(parseInt(props.glyphCode, 16) || 0x3f)}</span>
                {/if}
              </div>
            {/if}
            <label>Icon Set</label>
            <select value={props.iconSet ?? "material"} onchange={(e) => onSelectInput("iconSet", e)}>
              <option value="material">Material Icons</option>
              <option value="fontawesome">Font Awesome</option>
            </select>
            <label>Glyph Code (hex)</label>
            <input type="text" value={props.glyphCode ?? ""} placeholder="e.g. e88a" oninput={(e) => onInput("glyphCode", e)} />
            <label>Icon Source (path)</label>
            <input type="text" value={props.iconSrc ?? ""} placeholder="SVG/PNG path" oninput={(e) => onInput("iconSrc", e)} />
            <label>Color</label>
            <ColorField value={String(props.color ?? "#ffffff")} defaultColor="#ffffff" onChange={(v) => onColorProp("color", v)} />
            <label>Font Size</label>
            <input type="number" value={props.fontSize ?? 48} oninput={(e) => onInput("fontSize", e)} />
          </div>
        </section>
      {/if}
      {#if layer.type === "map"}
        <section class="prop-section">
          <div class="section-title">Map</div>
          <div class="prop-stack">
            <label>Latitude</label>
            <input type="number" step="0.1" value={props.mapLat ?? 40.7} oninput={(e) => onInput("mapLat", e)} />
            <label>Longitude</label>
            <input type="number" step="0.1" value={props.mapLng ?? -74.0} oninput={(e) => onInput("mapLng", e)} />
            <label>Zoom</label>
            <input type="number" min="0" max="18" value={props.mapZoom ?? 5} oninput={(e) => onInput("mapZoom", e)} />
            <label>Style</label>
            <select value={props.mapStyle ?? "dark"} onchange={(e) => onSelectInput("mapStyle", e)}>
              <option value="dark">Dark (CartoDB)</option>
              <option value="streets">Streets (OSM)</option>
              <option value="satellite">Satellite (ArcGIS)</option>
            </select>
            <label>Show Radar</label>
            <input type="checkbox" checked={props.mapShowRadar ?? true} onchange={(e) => updateLayerProperty(layer.id, "mapShowRadar", (e.target as HTMLInputElement).checked)} />
            <label>Animate Radar</label>
            <input type="checkbox" checked={props.mapRadarAnimate ?? false} onchange={(e) => updateLayerProperty(layer.id, "mapRadarAnimate", (e.target as HTMLInputElement).checked)} />
            <span class="prop-hint">Uses RainViewer (free, no API key). Requires internet.</span>
          </div>
        </section>
      {/if}

      {#if layer.type === "launcher"}
        <section class="prop-section">
          <div class="section-title">Launcher</div>
          <div class="prop-stack">
            <label>Style</label>
            <select value={props.launcherStyle ?? "win11"} onchange={(e) => onSelectInput("launcherStyle", e)}>
              <option value="win11">Windows 11</option>
              <option value="macos">macOS</option>
              <option value="deepin">Deepin</option>
            </select>
            <label>Icon Size</label>
            <input type="number" min="16" max="96" value={props.launcherIconSize ?? 36} oninput={(e) => onInput("launcherIconSize", e)} />
            <label style="margin-top: 6px; font-weight: 600;">Appearance</label>
            <label>Bar Background</label>
            <ColorField value={String(props.taskbarBg ?? "#141414")} defaultColor="#141414" onChange={(v) => onColorProp("taskbarBg", v)} />
            <label>Bar Opacity</label>
            <input type="range" min="0" max="255" value={props.taskbarBgOpacity ?? 235} oninput={(e) => onInput("taskbarBgOpacity", e)} style="width:100%" />
            <label>Corner Radius</label>
            <input type="number" min="0" max="32" value={props.taskbarRadius ?? 0} oninput={(e) => onInput("taskbarRadius", e)} />
            {#if (props.launcherStyle ?? "win11") === "win11"}
            <label style="margin-top: 6px; font-weight: 600;">Start Menu</label>
            <label>Menu Background</label>
            <ColorField value={String(props.smBg ?? "#1c1c1c")} defaultColor="#1c1c1c" onChange={(v) => onColorProp("smBg", v)} />
            <label>Accent Color</label>
            <ColorField value={String(props.smAccent ?? "#60cdff")} defaultColor="#60cdff" onChange={(v) => onColorProp("smAccent", v)} />
            {/if}
            <label style="margin-top: 6px; font-weight: 600;">Pinned Apps</label>
            <div class="pinned-list">
              {#each (props.pinnedApps ?? []) as exec (exec)}
                {@const appInfo = appList.find(a => a.exec === exec || a.exec.startsWith(exec + " ") || a.exec.split(/[\s/]/).pop() === exec)}
                <div class="pinned-item">
                  <span class="pinned-icon-wrap">
                    {#if appInfo}
                      <img src={getIconSrc(appInfo.icon)} alt="" class="pinned-icon"
                        onerror={(e) => { (e.target as HTMLImageElement).style.display='none'; ((e.target as HTMLImageElement).nextElementSibling as HTMLElement).style.display='flex'; }} />
                      <span class="pinned-letter" style="background:{iconBg(appInfo.name)};display:none">{iconLetter(appInfo.name)}</span>
                    {:else}
                      <span class="pinned-letter" style="background:{iconBg(exec)}">{iconLetter(exec)}</span>
                    {/if}
                  </span>
                  <span class="pinned-name">{appInfo?.name ?? exec}</span>
                  <button class="pinned-remove" onclick={() => removePinnedApp(exec)} title="Remove">×</button>
                </div>
              {/each}
              {#if (props.pinnedApps ?? []).length === 0}
                <span class="prop-hint" style="padding:6px 0;">No apps pinned yet</span>
              {/if}
            </div>
            <button class="browse-btn" style="width:100%;text-align:center;margin-top:4px;" onclick={() => openAppPicker(true)}>+ Add App</button>
          </div>
        </section>
      {/if}

      {#if layer.type === "radar"}
        <section class="prop-section">
          <div class="section-title">Radar</div>
          <div class="prop-stack">
            <label>Sweep Color</label>
            <ColorField value={String(props.radarSweepColor ?? "#00ff4480")} defaultColor="#00ff4480" onChange={(v) => onColorProp("radarSweepColor", v)} />
            <label>Ring Color</label>
            <ColorField value={String(props.radarRingColor ?? "#00ff4440")} defaultColor="#00ff4440" onChange={(v) => onColorProp("radarRingColor", v)} />
            <label>Dot Color</label>
            <ColorField value={String(props.radarDotColor ?? "#00ff44")} defaultColor="#00ff44" onChange={(v) => onColorProp("radarDotColor", v)} />
            <label>Dot Size</label>
            <input type="number" min="1" max="20" value={props.radarDotSize ?? 4} oninput={(e) => onInput("radarDotSize", e)} />
            <label>Ring Count</label>
            <input type="number" min="1" max="10" value={props.radarRingCount ?? 3} oninput={(e) => onInput("radarRingCount", e)} />
          </div>
        </section>
      {/if}

      <AnimationPanel />
    {:else}
      <div class="empty-state">Select a layer to edit its properties.</div>
    {/if}
  </div>
</div>

<FormulaHelper
  open={formulaHelperOpen}
  onInsert={handleFormulaInsert}
  onClose={() => formulaHelperOpen = false}
/>

<IconPicker
  open={iconPickerOpen}
  onSelect={handleIconSelect}
  onClose={() => iconPickerOpen = false}
  assetDir={getProject().assetDir ?? ""}
/>

{#if appPickerOpen}
  <div class="modal-backdrop" onclick={() => { appPickerOpen = false; appPickerSearch = ""; }}>
    <div class="app-picker-modal" onclick={(e) => e.stopPropagation()}>
      <div class="app-picker-header">
        <span>{appPickerForPinned ? "Add Pinned App" : "Select App"}</span>
        <span style="cursor:pointer;font-size:18px;line-height:1;" onclick={() => { appPickerOpen = false; appPickerSearch = ""; }}>&times;</span>
      </div>
      <input
        class="app-picker-search"
        type="text"
        placeholder="Search apps..."
        value={appPickerSearch}
        oninput={(e) => appPickerSearch = (e.target as HTMLInputElement).value}
      />
      <div class="app-picker-list">
        {#each appList.filter(a => a.name.toLowerCase().includes(appPickerSearch.toLowerCase())) as app (app.exec)}
          <div class="app-picker-item" onclick={() => selectApp(app.exec)} role="button" tabindex="0"
            onkeydown={(e) => { if (e.key === 'Enter') selectApp(app.exec); }}>
            <span class="app-picker-icon-wrap">
              <img src={getIconSrc(app.icon)} alt="" class="app-picker-icon"
                onerror={(e) => { (e.target as HTMLImageElement).style.display='none'; ((e.target as HTMLImageElement).nextElementSibling as HTMLElement).style.display='flex'; }} />
              <span class="app-picker-icon-letter" style="background:{iconBg(app.name)};display:none">{iconLetter(app.name)}</span>
            </span>
            <span class="app-picker-info">
              <span class="app-picker-name">{app.name}</span>
              <span class="app-picker-exec">{app.exec.split(" ")[0]}</span>
            </span>
            {#if appPickerForPinned}
              <span class="app-picker-add">+</span>
            {/if}
          </div>
        {/each}
        {#if appList.length === 0}
          <div style="padding:16px;text-align:center;color:var(--text-muted);font-size:12px;">Loading apps...</div>
        {/if}
      </div>
      {#if appPickerForPinned}
        <div style="padding:8px;border-top:1px solid var(--border);">
          <button class="browse-btn" style="width:100%;text-align:center;" onclick={() => { appPickerOpen = false; appPickerSearch = ""; }}>Done</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .property-panel {
    width: 280px;
    min-width: 280px;
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .panel-header {
    padding: 8px 12px;
    font-weight: 600;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }
  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }
  .prop-section {
    padding: 0 12px;
    margin-bottom: 12px;
  }
  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: 6px;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border);
  }
  .prop-grid {
    display: grid;
    grid-template-columns: 40px 1fr;
    gap: 4px 6px;
    align-items: center;
  }
  .prop-grid label {
    font-size: 11px;
    color: var(--text-secondary);
    text-align: right;
  }
  .prop-grid input,
  .prop-grid select {
    width: 100%;
    font-size: 12px;
    padding: 3px 6px;
  }
  .prop-stack {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .prop-stack label {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 4px;
  }
  .prop-stack label:first-child {
    margin-top: 0;
  }
  .prop-stack input,
  .prop-stack select {
    width: 100%;
    font-size: 12px;
    padding: 3px 6px;
  }
  .prop-stack input[type="color"] {
    height: 28px;
    padding: 2px;
    cursor: pointer;
  }
  .input-row {
    display: flex;
    gap: 4px;
  }
  .input-row input {
    flex: 1;
    min-width: 0;
  }
  .browse-btn {
    padding: 3px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
  }
  .browse-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }
  .fx-btn {
    padding: 3px 6px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--accent);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-mono);
    cursor: pointer;
    flex-shrink: 0;
  }
  .fx-btn:hover {
    background: var(--accent-dim);
    border-color: var(--accent);
  }
  .image-preview-small {
    width: 100%;
    height: 80px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .image-preview-small img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .icon-preview {
    width: 100%;
    height: 60px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .child-count {
    font-size: 11px;
    color: var(--text-muted);
    padding: 4px 0;
    margin-top: 4px;
    border-top: 1px solid var(--border);
  }
  .empty-state {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.6;
  }
  .prop-hint {
    font-size: 10px;
    color: var(--text-dim, #6a7a8a);
    line-height: 1.4;
    margin-top: 2px;
  }
  .prop-hint code {
    background: var(--bg-secondary);
    padding: 1px 3px;
    border-radius: 2px;
    font-family: monospace;
    color: var(--accent);
  }
  .input-with-browse {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .input-with-browse input { flex: 1; }
  .browse-btn {
    font-size: 12px;
    font-weight: 700;
    padding: 3px 7px;
    border-radius: 3px;
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    cursor: pointer;
    flex-shrink: 0;
  }
  .browse-btn:hover { background: var(--bg-secondary); color: var(--accent); }
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .app-picker-modal {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 420px;
    max-height: 520px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0,0,0,0.4);
  }
  .app-picker-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    font-weight: 600;
    font-size: 13px;
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
  }
  .app-picker-search {
    margin: 8px;
    padding: 6px 10px;
    font-size: 13px;
    border-radius: 4px;
    width: calc(100% - 16px);
    box-sizing: border-box;
  }
  .app-picker-list {
    overflow-y: auto;
    flex: 1;
  }
  /* Pinned apps list */
  .pinned-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 200px;
    overflow-y: auto;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px;
  }
  .pinned-item {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 3px 4px;
    border-radius: 3px;
    min-height: 28px;
  }
  .pinned-item:hover { background: var(--bg-primary); }
  .pinned-icon-wrap {
    width: 22px; height: 22px;
    flex-shrink: 0;
    display: flex; align-items: center; justify-content: center;
  }
  .pinned-icon { width: 22px; height: 22px; object-fit: contain; border-radius: 3px; }
  .pinned-letter {
    width: 22px; height: 22px;
    display: flex; align-items: center; justify-content: center;
    border-radius: 3px;
    color: #fff; font-size: 11px; font-weight: bold;
  }
  .pinned-name { flex: 1; font-size: 12px; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pinned-remove {
    background: none; border: none; cursor: pointer;
    color: var(--text-muted); font-size: 15px; line-height: 1;
    padding: 0 2px; opacity: 0.6;
  }
  .pinned-remove:hover { color: #e06060; opacity: 1; }
  /* App picker icons */
  .app-picker-icon-wrap {
    width: 28px; height: 28px;
    flex-shrink: 0;
    display: flex; align-items: center; justify-content: center;
  }
  .app-picker-icon { width: 28px; height: 28px; object-fit: contain; border-radius: 4px; }
  .app-picker-icon-letter {
    width: 28px; height: 28px;
    display: flex; align-items: center; justify-content: center;
    border-radius: 4px; color: #fff; font-size: 13px; font-weight: bold;
  }
  .app-picker-info { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
  .app-picker-add { color: var(--accent, #60cdff); font-size: 16px; font-weight: bold; padding-right: 4px; }

  .app-picker-item {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 10px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-bottom: 1px solid var(--border);
  }
  .app-picker-item:hover { background: var(--bg-secondary); }
  .app-picker-name { font-size: 12px; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .app-picker-exec { font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
