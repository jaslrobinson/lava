<script lang="ts">
  import {
    getProject,
    getSelectedLayerId,
    setSelectedLayerId,
    removeLayer,
    isContainerType,
    moveLayer,
  } from "../stores/project.svelte";
  import type { Layer } from "../types/project";

  let collapsedIds = $state(new Set<string>());
  let dragSourceId = $state<string | null>(null);
  let dropTargetId = $state<string | null>(null);
  let dropPosition = $state<"before" | "after" | "inside" | null>(null);

  // Container drill-in navigation
  let focusedContainerId = $state<string | null>(null);

  function toggleCollapsed(id: string) {
    const next = new Set(collapsedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    collapsedIds = next;
  }

  const typeIcons: Record<string, string> = {
    text: "T",
    shape: "\u25A0",
    image: "\u{1F5BC}",
    progress: "\u25CB",
    group: "\u{1F4C1}",
    stack: "\u2261",
    overlap: "\u29C9",
    fonticon: "\u2606",
  };

  function stop(fn: (e: MouseEvent) => void) {
    return (e: MouseEvent) => { e.stopPropagation(); fn(e); };
  }

  /** Find a layer by ID in the tree */
  function findLayer(layers: Layer[], id: string): Layer | undefined {
    for (const l of layers) {
      if (l.id === id) return l;
      if (l.children) {
        const found = findLayer(l.children, id);
        if (found) return found;
      }
    }
    return undefined;
  }

  /** Build path from root to a given container ID */
  function buildBreadcrumbPath(layers: Layer[], targetId: string): { id: string; name: string }[] {
    const path: { id: string; name: string }[] = [];

    function search(items: Layer[], trail: { id: string; name: string }[]): boolean {
      for (const item of items) {
        if (item.id === targetId) {
          path.push(...trail, { id: item.id, name: item.name });
          return true;
        }
        if (item.children) {
          if (search(item.children, [...trail, { id: item.id, name: item.name }])) return true;
        }
      }
      return false;
    }

    search(layers, []);
    return path;
  }

  /** Get the layers to display based on focused container */
  function getDisplayLayers(): Layer[] {
    const project = getProject();
    if (!focusedContainerId) return project.layers;
    const container = findLayer(project.layers, focusedContainerId);
    if (!container || !container.children) {
      focusedContainerId = null;
      return project.layers;
    }
    return container.children;
  }

  function drillInto(id: string) {
    focusedContainerId = id;
  }

  function navigateTo(id: string | null) {
    focusedContainerId = id;
  }

  function handleDblClick(e: MouseEvent, layer: Layer) {
    e.stopPropagation();
    if (isContainerType(layer.type)) {
      drillInto(layer.id);
    }
  }

  function handleDragStart(e: DragEvent, id: string) {
    dragSourceId = id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", id);
    }
  }

  function handleDragOver(e: DragEvent, id: string, isContainer: boolean) {
    if (!dragSourceId || dragSourceId === id) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";

    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    const h = rect.height;

    if (isContainer && y > h * 0.25 && y < h * 0.75) {
      dropPosition = "inside";
    } else if (y < h / 2) {
      dropPosition = "before";
    } else {
      dropPosition = "after";
    }
    dropTargetId = id;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    if (dragSourceId && dropTargetId && dropPosition) {
      moveLayer(dragSourceId, dropTargetId, dropPosition);
    }
    dragSourceId = null;
    dropTargetId = null;
    dropPosition = null;
  }

  function handleDragEnd() {
    dragSourceId = null;
    dropTargetId = null;
    dropPosition = null;
  }

  function toggleVisibility(id: string, currentVisible: boolean | undefined) {
    const allLayers = getProject().layers;
    const layer = findLayer(allLayers, id);
    if (!layer) return;
    layer.visible = currentVisible === false ? true : false;
  }
</script>

<div class="layer-panel">
  {#if focusedContainerId}
    {@const breadcrumbs = buildBreadcrumbPath(getProject().layers, focusedContainerId)}
    <div class="breadcrumbs">
      <button class="breadcrumb-item" onclick={() => navigateTo(null)} title="Back to root">
        Root
      </button>
      {#each breadcrumbs as crumb, i}
        <span class="breadcrumb-sep">/</span>
        {#if i < breadcrumbs.length - 1}
          <button class="breadcrumb-item" onclick={() => navigateTo(crumb.id)}>
            {crumb.name}
          </button>
        {:else}
          <span class="breadcrumb-current">{crumb.name}</span>
        {/if}
      {/each}
    </div>
  {/if}

  <div class="layer-list">
    {#if getDisplayLayers().length === 0}
      <div class="empty-state">
        {focusedContainerId ? "Empty container." : "No layers yet. Add one from the toolbar."}
      </div>
    {:else}
      {#each getDisplayLayers().toReversed() as layer (layer.id)}
        {@render layerRow(layer, 0)}
      {/each}
    {/if}
  </div>
</div>

{#snippet layerRow(layer: Layer, depth: number)}
  {@const isSelected = getSelectedLayerId() === layer.id}
  {@const isContainer = isContainerType(layer.type)}
  {@const hasChildren = layer.children && layer.children.length > 0}
  {@const isDragOver = dropTargetId === layer.id}
  {@const isDropBefore = isDragOver && dropPosition === "before"}
  {@const isDropAfter = isDragOver && dropPosition === "after"}
  {@const isDropInside = isDragOver && dropPosition === "inside"}
  <div
    class="layer-item"
    class:selected={isSelected}
    class:container-target={isSelected && isContainer}
    class:hidden-layer={layer.visible === false}
    class:dragging={dragSourceId === layer.id}
    style="padding-left: {12 + depth * 16}px;{isDropBefore ? ' border-top: 2px solid var(--accent);' : ''}{isDropAfter ? ' border-bottom: 2px solid var(--accent);' : ''}{isDropInside ? ' background: var(--accent-dim); outline: 1px dashed var(--accent);' : ''}"
    draggable="true"
    onclick={() => setSelectedLayerId(layer.id)}
    ondblclick={(e) => handleDblClick(e, layer)}
    ondragstart={(e) => handleDragStart(e, layer.id)}
    ondragover={(e) => handleDragOver(e, layer.id, isContainer)}
    ondrop={handleDrop}
    ondragend={handleDragEnd}
    role="button"
    tabindex="0"
    onkeydown={(e) => { if (e.key === 'Enter') setSelectedLayerId(layer.id); }}
  >
    <span
      style="font-size: 12px; width: 20px; padding: 0; opacity: 0.6; cursor: pointer; text-align: center; display: inline-block;"
      title={layer.visible === false ? "Show layer" : "Hide layer"}
      role="button"
      tabindex="0"
      onclick={stop(() => toggleVisibility(layer.id, layer.visible))}
      onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); toggleVisibility(layer.id, layer.visible); } }}
    >
      {layer.visible === false ? "\u{1F441}\u200D\u{1F5E8}" : "\u{1F441}"}
    </span>
    {#if isContainer}
      <span
        style="font-size: 10px; color: var(--text-muted); width: 12px; text-align: center; cursor: pointer; display: inline-block;"
        title={collapsedIds.has(layer.id) ? "Expand" : "Collapse"}
        role="button"
        tabindex="0"
        onclick={stop(() => toggleCollapsed(layer.id))}
        onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); toggleCollapsed(layer.id); } }}
      >{collapsedIds.has(layer.id) ? "\u25B8" : (hasChildren ? "\u25BE" : "\u25B8")}</span>
      <span
        class="drill-btn"
        title="Enter container"
        role="button"
        tabindex="0"
        onclick={stop(() => drillInto(layer.id))}
        onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); drillInto(layer.id); } }}
      >{"\u{279C}"}</span>
    {/if}
    <span class="layer-type-icon">{typeIcons[layer.type] || "?"}</span>
    <span class="layer-name">{layer.name}</span>
    {#if isContainer && hasChildren}
      <span class="child-count">{layer.children!.length}</span>
    {/if}
    {#if isSelected && isContainer}
      <span class="container-hint">+ target</span>
    {/if}
    <span
      style="font-size: 16px; width: 20px; padding: 0; color: var(--text-muted); opacity: 0; cursor: pointer; text-align: center; display: inline-block;"
      class="delete-btn"
      title="Delete layer"
      role="button"
      tabindex="0"
      onclick={stop(() => removeLayer(layer.id))}
      onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); removeLayer(layer.id); } }}
    >
      \u00D7
    </span>
  </div>
  {#if hasChildren && !collapsedIds.has(layer.id)}
    {#each layer.children!.toReversed() as child (child.id)}
      {@render layerRow(child, depth + 1)}
    {/each}
  {/if}
{/snippet}

<style>
  .layer-panel {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }
  .breadcrumbs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px 10px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    flex-shrink: 0;
    overflow-x: auto;
    white-space: nowrap;
  }
  .breadcrumb-item {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 11px;
    cursor: pointer;
    padding: 1px 4px;
    border-radius: 3px;
  }
  .breadcrumb-item:hover {
    background: var(--accent-dim);
    text-decoration: underline;
  }
  .breadcrumb-sep {
    color: var(--text-muted);
    font-size: 10px;
  }
  .breadcrumb-current {
    color: var(--text-primary);
    font-weight: 600;
    font-size: 11px;
  }
  .layer-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .layer-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    cursor: pointer;
    border-left: 2px solid transparent;
    transition: background 0.1s;
    user-select: none;
  }
  .layer-item:hover {
    background: var(--bg-secondary);
  }
  .layer-item.dragging {
    opacity: 0.4;
  }
  .layer-item.selected {
    background: var(--accent-dim);
    border-left-color: var(--accent);
  }
  .layer-item.container-target {
    border-left-color: var(--accent);
    border-left-width: 3px;
  }
  .container-hint {
    font-size: 9px;
    color: var(--accent);
    opacity: 0.7;
    white-space: nowrap;
  }
  .child-count {
    font-size: 9px;
    color: var(--text-muted);
    background: var(--bg-input);
    padding: 0 4px;
    border-radius: 3px;
    margin-left: auto;
  }
  .layer-item.hidden-layer .layer-name,
  .layer-item.hidden-layer .layer-type-icon {
    opacity: 0.4;
  }
  .drill-btn {
    font-size: 10px;
    color: var(--text-muted);
    width: 14px;
    text-align: center;
    cursor: pointer;
    display: inline-block;
    opacity: 0.5;
    transition: opacity 0.1s, color 0.1s;
  }
  .drill-btn:hover {
    opacity: 1;
    color: var(--accent);
  }
  .layer-type-icon {
    font-size: 14px;
    width: 18px;
    text-align: center;
    color: var(--text-muted);
  }
  .layer-name {
    flex: 1;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .delete-btn {
    font-size: 16px;
    width: 20px;
    padding: 0;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity 0.1s, color 0.1s;
  }
  .layer-item:hover .delete-btn {
    opacity: 0.6;
  }
  .delete-btn:hover {
    opacity: 1 !important;
    color: var(--error);
  }
  .empty-state {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.6;
  }
</style>
