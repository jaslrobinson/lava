<script lang="ts">
  import { getSettings, updateSetting } from "../stores/settings.svelte";
  import { getProject, insertWidget } from "../stores/project.svelte";
  import { createLayer } from "../types/project";

  let { onClose }: { onClose: () => void } = $props();

  let imagePath = $state("");
  let imagePreviewUrl = $state("");
  let layerName = $state("");
  let apiKey = $state(getSettings().replicateApiKey ?? "");
  let status = $state<"idle" | "extracting" | "done" | "error">("idle");
  let statusMessage = $state("");
  let resultPath = $state("");
  let resultPreviewUrl = $state("");

  async function pickImage() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] }],
      multiple: false,
    });
    if (path) {
      imagePath = path as string;
      const { convertFileSrc } = await import("@tauri-apps/api/core");
      imagePreviewUrl = convertFileSrc(imagePath);
    }
  }

  async function handleExtract() {
    if (!imagePath) { alert("Please select a source image first."); return; }
    if (!apiKey.trim()) { alert("Please enter your Replicate API key."); return; }

    // Persist API key if changed
    if (apiKey.trim() !== (getSettings().replicateApiKey ?? "")) {
      updateSetting("replicateApiKey", apiKey.trim());
    }

    status = "extracting";
    statusMessage = "Removing background… this may take 10–30 seconds.";
    resultPath = "";
    resultPreviewUrl = "";

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const proj = getProject();
      const assetDir = proj.assetDir ?? "";

      const path = await invoke<string>("extract_image_layer", {
        imagePath,
        prompt: "",
        assetDir,
        apiKey: apiKey.trim(),
      });

      resultPath = path;
      const { convertFileSrc } = await import("@tauri-apps/api/core");
      resultPreviewUrl = convertFileSrc(path);
      status = "done";
      statusMessage = "Background removed!";
    } catch (e) {
      status = "error";
      statusMessage = String(e);
    }
  }

  function handleAddLayer() {
    if (!resultPath) return;
    const name = layerName.trim() || "Extracted Subject";
    const layer = createLayer("image", name);
    layer.properties.src = resultPath;
    layer.properties.width = 400;
    layer.properties.height = 300;
    layer.properties.scaleMode = "fit";
    insertWidget(layer);
    onClose();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}></div>

<div class="modal">
  <div class="modal-header">
    <span class="modal-title">✂ AI Background Remover</span>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span class="modal-close" onclick={onClose}>✕</span>
  </div>

  <div class="modal-body">
    <!-- API Key -->
    <div class="field">
      <label class="field-label">Replicate API Key</label>
      <input
        class="field-input"
        type="password"
        placeholder="r8_..."
        bind:value={apiKey}
      />
    </div>

    <!-- Source image -->
    <div class="field">
      <label class="field-label">Source Image</label>
      <div class="image-pick-row">
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span class="pick-btn" onclick={pickImage}>
          {imagePath ? "Change Image" : "Choose Image…"}
        </span>
        {#if imagePath}
          <span class="file-name">{imagePath.split("/").pop()}</span>
        {/if}
      </div>
      {#if imagePreviewUrl}
        <img class="preview" src={imagePreviewUrl} alt="Source" />
      {/if}
    </div>

    <!-- Layer name -->
    <div class="field">
      <label class="field-label">Layer name (optional)</label>
      <input
        class="field-input"
        type="text"
        placeholder="Mountains, Person, Logo…"
        bind:value={layerName}
      />
      <span class="field-hint">AI removes the background and produces a transparent PNG.</span>
    </div>

    <!-- Extract button -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      class="extract-btn"
      class:extract-btn-disabled={status === "extracting"}
      onclick={status !== "extracting" ? handleExtract : undefined}
    >
      {status === "extracting" ? "Removing background…" : "Remove Background"}
    </span>

    <!-- Status -->
    {#if statusMessage}
      <div class="status" class:status-error={status === "error"}>
        {statusMessage}
      </div>
    {/if}

    <!-- Result -->
    {#if resultPreviewUrl}
      <div class="result-section">
        <label class="field-label">Result</label>
        <img class="preview result-preview" src={resultPreviewUrl} alt="Extracted" />
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span class="add-btn" onclick={handleAddLayer}>
          Add as Image Layer
        </span>
      </div>
    {/if}
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    z-index: 200;
  }
  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 480px;
    max-height: 80vh;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 16px 48px rgba(0,0,0,0.5);
    z-index: 201;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .modal-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .modal-close {
    font-size: 14px;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 3px;
  }
  .modal-close:hover { color: var(--text-primary); background: var(--bg-secondary); }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .field { display: flex; flex-direction: column; gap: 4px; }
  .field-label { font-size: 11px; font-weight: 500; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.4px; }
  .field-input {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 12px;
    color: var(--text-primary);
    width: 100%;
    box-sizing: border-box;
  }
  .field-input:focus { outline: none; border-color: var(--accent); }
  .field-hint { font-size: 10px; color: var(--text-muted); }

  .image-pick-row { display: flex; align-items: center; gap: 8px; }
  .pick-btn {
    font-size: 12px;
    color: var(--accent);
    background: var(--accent-dim);
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 4px 10px;
    cursor: pointer;
    white-space: nowrap;
  }
  .pick-btn:hover { background: var(--accent); color: #fff; }
  .file-name { font-size: 11px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .preview {
    width: 100%;
    max-height: 160px;
    object-fit: contain;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: #111;
    margin-top: 4px;
  }
  .result-preview { max-height: 200px; }

  .extract-btn {
    display: inline-block;
    text-align: center;
    padding: 8px 16px;
    background: var(--accent);
    color: #fff;
    border-radius: 5px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
    user-select: none;
  }
  .extract-btn:hover { background: #c0392b; }
  .extract-btn-disabled { opacity: 0.5; cursor: not-allowed; }

  .status {
    font-size: 12px;
    color: var(--accent);
    background: var(--accent-dim);
    border-radius: 4px;
    padding: 8px 10px;
  }
  .status-error { color: #e74c3c; background: rgba(231,76,60,0.12); }

  .result-section { display: flex; flex-direction: column; gap: 8px; }
  .add-btn {
    display: inline-block;
    text-align: center;
    padding: 7px 14px;
    background: #2d7d46;
    color: #fff;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    user-select: none;
    transition: background 0.15s;
  }
  .add-btn:hover { background: #27ae60; }
</style>
