<script lang="ts">
  import { getProject } from "../stores/project.svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";

  function updateBgType(type: "color" | "image") {
    const project = getProject();
    project.background = { ...project.background, type };
    if (type === "color" && !project.background.value.startsWith("#")) {
      project.background.value = "#1a1a2e";
    }
  }

  function updateBgValue(value: string) {
    const project = getProject();
    project.background = { ...project.background, value };
  }

  function updateResolution(field: "width" | "height", value: number) {
    const project = getProject();
    project.resolution = { ...project.resolution, [field]: value };
  }

  function updateName(name: string) {
    const project = getProject();
    project.name = name;
  }

  async function browseBackground() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const path = await open({
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"] }],
        multiple: false,
      });
      if (path) {
        const project = getProject();
        project.background = { type: "image", value: String(path) };
      }
    } catch (e) {
      console.error("Browse failed:", e);
    }
  }

  function getPreviewSrc(value: string): string {
    if (!value || value.startsWith("#")) return "";
    if (value.startsWith("http") || value.startsWith("data:")) return value;
    return convertFileSrc(value);
  }
</script>

<div class="bg-panel">
  <div class="panel-header">
    <span>Background</span>
  </div>
  <div class="panel-body">
    <section class="prop-section">
      <div class="section-title">Project</div>
      <div class="prop-stack">
        <label>Name</label>
        <input type="text" value={getProject().name}
          oninput={(e) => updateName((e.target as HTMLInputElement).value)} />
        <label>Resolution</label>
        <div class="res-row">
          <input type="number" value={getProject().resolution.width}
            oninput={(e) => updateResolution("width", Number((e.target as HTMLInputElement).value))} />
          <span class="res-x">&times;</span>
          <input type="number" value={getProject().resolution.height}
            oninput={(e) => updateResolution("height", Number((e.target as HTMLInputElement).value))} />
        </div>
        <div class="preset-row">
          <button class="preset-btn" onclick={() => { updateResolution("width", 1920); updateResolution("height", 1080); }}>1080p</button>
          <button class="preset-btn" onclick={() => { updateResolution("width", 2560); updateResolution("height", 1440); }}>1440p</button>
          <button class="preset-btn" onclick={() => { updateResolution("width", 3840); updateResolution("height", 2160); }}>4K</button>
        </div>
      </div>
    </section>

    <section class="prop-section">
      <div class="section-title">Background</div>
      <div class="prop-stack">
        <label>Type</label>
        <div class="type-toggle">
          <button class="toggle-btn" class:active={getProject().background.type === "color"}
            onclick={() => updateBgType("color")}>Color</button>
          <button class="toggle-btn" class:active={getProject().background.type === "image"}
            onclick={() => updateBgType("image")}>Image</button>
        </div>

        {#if getProject().background.type === "color"}
          <label>Color</label>
          <input type="color" value={getProject().background.value}
            oninput={(e) => updateBgValue((e.target as HTMLInputElement).value)} />
        {:else}
          <label>Image</label>
          <div class="input-row">
            <input type="text" value={getProject().background.value} placeholder="Path or URL"
              oninput={(e) => updateBgValue((e.target as HTMLInputElement).value)} />
            <button class="browse-btn" onclick={browseBackground}>...</button>
          </div>
          {#if getProject().background.value && !getProject().background.value.startsWith("#")}
            <div class="bg-preview">
              <img src={getPreviewSrc(getProject().background.value)} alt="background" />
            </div>
          {/if}
        {/if}
      </div>
    </section>
  </div>
</div>

<style>
  .bg-panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
  }
  .panel-header {
    display: flex;
    align-items: center;
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
  .res-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .res-row input {
    flex: 1;
    min-width: 0;
  }
  .res-x {
    color: var(--text-muted);
    font-size: 12px;
  }
  .preset-row {
    display: flex;
    gap: 4px;
    margin-top: 4px;
  }
  .preset-btn {
    flex: 1;
    padding: 3px 6px;
    font-size: 11px;
    border-radius: 4px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }
  .preset-btn:hover {
    background: var(--accent-dim);
    color: var(--accent);
  }
  .type-toggle {
    display: flex;
    gap: 0;
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid var(--border);
  }
  .toggle-btn {
    flex: 1;
    padding: 4px 8px;
    font-size: 12px;
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }
  .toggle-btn.active {
    background: var(--accent-dim);
    color: var(--accent);
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
  .bg-preview {
    width: 100%;
    height: 100px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 4px;
  }
  .bg-preview img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
</style>
