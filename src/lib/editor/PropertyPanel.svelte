<script lang="ts">
  import { getSelectedLayer, updateLayerProperty, getProject } from "../stores/project.svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import FormulaHelper from "./FormulaHelper.svelte";
  import AnimationPanel from "./AnimationPanel.svelte";
  import IconPicker from "./IconPicker.svelte";
  import { SYSTEM_FONTS } from "../data/fonts";
  import { getProjectFontNames, loadFont } from "../fonts/fontLoader";

  let formulaHelperOpen = $state(false);
  let iconPickerOpen = $state(false);
  let showCustomFont = $state(false);

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
    const numericKeys = ["x", "y", "width", "height", "rotation", "opacity", "fontSize", "strokeWidth", "cornerRadius", "min", "max", "value", "lineSpacing", "maxLines", "spacing", "scaleX", "scaleY"];
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
          <input type="text" value={props.clickAction ?? ""} placeholder="set:var:val / inc:var:amt / url:..." oninput={(e) => onInput("clickAction", e)} />
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
            <input type="color" value={props.color ?? "#ffffff"} oninput={(e) => onInput("color", e)} />
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
            <input type="color" value={props.fill ?? "#e94560"} oninput={(e) => onInput("fill", e)} />
            <label>Stroke</label>
            <input type="color" value={props.stroke ?? "#000000"} oninput={(e) => onInput("stroke", e)} />
            <label>Stroke Width</label>
            <input type="number" value={props.strokeWidth ?? 0} oninput={(e) => onInput("strokeWidth", e)} />
            <label>Corner Radius</label>
            <input type="number" value={props.cornerRadius ?? 0} oninput={(e) => onInput("cornerRadius", e)} />
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
              <input type="text" value={props.src ?? ""} placeholder="Path or URL" oninput={(e) => onInput("src", e)} />
              <button class="browse-btn" title="Browse for image" onclick={handleBrowseImage}>...</button>
            </div>
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
            <input type="color" value={props.tint ?? "#ffffff"} oninput={(e) => onInput("tint", e)} />
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
            <input type="number" value={props.value ?? 50} oninput={(e) => onInput("value", e)} />
            <label>Color</label>
            <input type="color" value={props.color ?? "#e94560"} oninput={(e) => onInput("color", e)} />
            <label>Track Color</label>
            <input type="color" value={props.trackColor ?? "#333333"} oninput={(e) => onInput("trackColor", e)} />
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

      {#if layer.type === "group" || layer.type === "overlap"}
        <section class="prop-section">
          <div class="section-title">{layer.type === "group" ? "Group" : "Overlap"}</div>
          <div class="prop-stack">
            <label>Width</label>
            <input type="number" value={props.width} oninput={(e) => onInput("width", e)} />
            <label>Height</label>
            <input type="number" value={props.height} oninput={(e) => onInput("height", e)} />
            <div class="child-count">Children: {layer.children?.length ?? 0}</div>
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
            <input type="color" value={props.color ?? "#ffffff"} oninput={(e) => onInput("color", e)} />
            <label>Font Size</label>
            <input type="number" value={props.fontSize ?? 48} oninput={(e) => onInput("fontSize", e)} />
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
</style>
