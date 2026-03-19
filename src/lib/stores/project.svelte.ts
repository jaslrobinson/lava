import { type Project, type Layer, type Animation, createDefaultProject, createLayer, cloneLayerWithNewIds, type LayerType, type GlobalVarType } from "../types/project";
import type { BrushType } from "../types/paint";
import { clearImageCache } from "../canvas/renderer";
import { invalidateGlobalsFormulas, clearFormulaCache } from "../formula/service";
import { resetAnimationState } from "../canvas/animationState";

let project = $state<Project>(createDefaultProject());
let selectedLayerId = $state<string | null>(null);
let isDirty = $state(false);
let wallpaperMode = $state(false);
let activeOverlay = $state<string | null>(null);
let interactiveMode = $state(false);
let copiedLayer = $state<Layer | null>(null);
let currentProjectPath = $state<string>("");
let toolMode = $state<'select' | 'paint'>('select');
let paintBrushSettings = $state({
  brushType: 'solid' as BrushType,
  brushSize: 20,
  color: '#FF0000',
  opacity: 1.0,
});

// Undo history
const MAX_UNDO = 30;
let undoStack = $state<string[]>([]);
let redoStack = $state<string[]>([]);

function pushUndo() {
  undoStack = [...undoStack.slice(-(MAX_UNDO - 1)), JSON.stringify(project.layers)];
  redoStack = [];
}

export function undo() {
  if (undoStack.length === 0) return;
  const current = JSON.stringify(project.layers);
  redoStack = [...redoStack.slice(-(MAX_UNDO - 1)), current];
  const prev = undoStack[undoStack.length - 1];
  undoStack = undoStack.slice(0, -1);
  project.layers = JSON.parse(prev);
  isDirty = true;
}

export function redo() {
  if (redoStack.length === 0) return;
  const current = JSON.stringify(project.layers);
  undoStack = [...undoStack, current];
  const next = redoStack[redoStack.length - 1];
  redoStack = redoStack.slice(0, -1);
  project.layers = JSON.parse(next);
  isDirty = true;
}

export function canUndo() { return undoStack.length > 0; }
export function canRedo() { return redoStack.length > 0; }

export function getProject() { return project; }
export function getProjectSnapshot() { return $state.snapshot(project) as Project; }
export function setProject(p: Project) { project = p; isDirty = false; clearImageCache(); clearFormulaCache(); resetAnimationState(); }
export function getSelectedLayerId() { return selectedLayerId; }
export function setSelectedLayerId(id: string | null) { selectedLayerId = id; }
export function getIsDirty() { return isDirty; }
export function getWallpaperMode() { return wallpaperMode; }
export function setWallpaperMode(active: boolean) { wallpaperMode = active; }
export function getActiveOverlay() { return activeOverlay; }
export function setActiveOverlay(overlay: string | null) { activeOverlay = overlay; }
export function getInteractiveMode() { return interactiveMode; }
export function setInteractiveMode(active: boolean) { interactiveMode = active; }
export function getCopiedLayer() { return copiedLayer; }
export function getCurrentProjectPath() { return currentProjectPath; }
export function setCurrentProjectPath(path: string) { currentProjectPath = path; }
export function getToolMode() { return toolMode; }
export function setToolMode(mode: 'select' | 'paint') { toolMode = mode; }
export function getPaintBrushSettings() { return paintBrushSettings; }
export function setPaintBrushSettings(settings: Partial<typeof paintBrushSettings>) {
  paintBrushSettings = { ...paintBrushSettings, ...settings };
}

export function copySelectedLayer() {
  const layer = getSelectedLayer();
  if (layer) copiedLayer = JSON.parse(JSON.stringify(layer));
}

export function pasteLayer(newName?: string) {
  if (!copiedLayer) return;
  pushUndo();
  const cloned = cloneLayerWithNewIds(copiedLayer, newName ?? copiedLayer.name + " copy");
  // Paste into selected container, or at root
  const selected = getSelectedLayer();
  if (selected && isContainerType(selected.type)) {
    project.layers = addChildToParent(project.layers, selected.id, cloned);
  } else {
    project.layers = [...project.layers, cloned];
  }
  selectedLayerId = cloned.id;
  isDirty = true;
}

/** Recursively find a layer by ID in a layer tree */
function findInLayers(layers: Layer[], id: string): Layer | undefined {
  for (const l of layers) {
    if (l.id === id) return l;
    if (l.children) {
      const found = findInLayers(l.children, id);
      if (found) return found;
    }
  }
  return undefined;
}

/** Recursively update a layer property */
function updateInLayers(layers: Layer[], id: string, key: string, value: any): Layer[] {
  return layers.map(l => {
    if (l.id === id) {
      return { ...l, properties: { ...l.properties, [key]: value } };
    }
    if (l.children) {
      return { ...l, children: updateInLayers(l.children, id, key, value) };
    }
    return l;
  });
}

/** Recursively remove a layer */
function removeFromLayers(layers: Layer[], id: string): Layer[] {
  return layers
    .filter(l => l.id !== id)
    .map(l => {
      if (l.children) {
        return { ...l, children: removeFromLayers(l.children, id) };
      }
      return l;
    });
}

/** Flatten all layers for counting */
export function flattenLayers(layers: Layer[]): Layer[] {
  const result: Layer[] = [];
  for (const l of layers) {
    result.push(l);
    if (l.children) {
      result.push(...flattenLayers(l.children));
    }
  }
  return result;
}

/** Check if a layer type is a container that can hold children */
export function isContainerType(type: LayerType): boolean {
  return type === "stack" || type === "overlap";
}

export function addLayer(type: LayerType) {
  pushUndo();
  const allLayers = flattenLayers(project.layers);
  const name = `${type}_${allLayers.length + 1}`;
  const layer = createLayer(type, name);

  // If a container layer is selected, add as child of that container
  const selected = getSelectedLayer();
  if (selected && isContainerType(selected.type)) {
    project.layers = addChildToParent(project.layers, selected.id, layer);
  } else {
    project.layers = [...project.layers, layer];
  }

  selectedLayerId = layer.id;
  isDirty = true;
}

/** Add a child layer to a specific parent container by ID */
function addChildToParent(layers: Layer[], parentId: string, child: Layer): Layer[] {
  return layers.map(l => {
    if (l.id === parentId) {
      return { ...l, children: [...(l.children || []), child] };
    }
    if (l.children) {
      return { ...l, children: addChildToParent(l.children, parentId, child) };
    }
    return l;
  });
}

export function removeLayer(id: string) {
  pushUndo();
  project.layers = removeFromLayers(project.layers, id);
  if (selectedLayerId === id) selectedLayerId = null;
  isDirty = true;
}

/** Rename a layer by ID */
export function renameLayer(id: string, name: string) {
  function renameInLayers(layers: Layer[]): Layer[] {
    return layers.map(l => {
      if (l.id === id) return { ...l, name };
      if (l.children) return { ...l, children: renameInLayers(l.children) };
      return l;
    });
  }
  pushUndo();
  project.layers = renameInLayers(project.layers);
  isDirty = true;
}

/** Rename the project */
export function renameProject(name: string) {
  project.name = name;
  isDirty = true;
}

let lastUndoPropTime = 0;
export function updateLayerProperty(id: string, key: string, value: any) {
  // Batch property edits: only push undo if >500ms since last property change
  const now = Date.now();
  if (now - lastUndoPropTime > 500) pushUndo();
  lastUndoPropTime = now;
  project.layers = updateInLayers(project.layers, id, key, value);
  isDirty = true;
}

/** Update a layer's animations array by ID */
function updateLayerAnimations(layers: Layer[], id: string, updater: (anims: Animation[]) => Animation[]): Layer[] {
  return layers.map(l => {
    if (l.id === id) {
      return { ...l, animations: updater(l.animations || []) };
    }
    if (l.children) {
      return { ...l, children: updateLayerAnimations(l.children, id, updater) };
    }
    return l;
  });
}

export function addAnimation(layerId: string, anim: Animation) {
  console.log('addAnimation called for layer', layerId, 'anim:', anim);
  pushUndo();
  project.layers = updateLayerAnimations(project.layers, layerId, (anims) => [...anims, anim]);
  isDirty = true;
  console.log('Project after adding animation:', JSON.stringify(project, null, 2).substring(0, 500));
}

export function updateAnimation(layerId: string, index: number, anim: Animation) {
  project.layers = updateLayerAnimations(project.layers, layerId, (anims) => {
    const copy = [...anims];
    copy[index] = anim;
    return copy;
  });
  isDirty = true;
}

export function removeAnimation(layerId: string, index: number) {
  pushUndo();
  project.layers = updateLayerAnimations(project.layers, layerId, (anims) =>
    anims.filter((_, i) => i !== index)
  );
  isDirty = true;
}

/** Check if targetId is a descendant of ancestorId */
function isDescendant(layers: Layer[], ancestorId: string, targetId: string): boolean {
  const ancestor = findInLayers(layers, ancestorId);
  if (!ancestor?.children) return false;
  for (const child of ancestor.children) {
    if (child.id === targetId) return true;
    if (child.children && isDescendant([child], child.id, targetId)) return true;
  }
  return false;
}

/** Find the parent array and index of a layer */
function findParentArray(layers: Layer[], id: string): { arr: Layer[]; idx: number; parent: Layer | null } | null {
  for (let i = 0; i < layers.length; i++) {
    if (layers[i].id === id) return { arr: layers, idx: i, parent: null };
    if (layers[i].children) {
      const found = findParentArrayInner(layers[i].children!, id, layers[i]);
      if (found) return found;
    }
  }
  return null;
}
function findParentArrayInner(layers: Layer[], id: string, parent: Layer): { arr: Layer[]; idx: number; parent: Layer } | null {
  for (let i = 0; i < layers.length; i++) {
    if (layers[i].id === id) return { arr: layers, idx: i, parent };
    if (layers[i].children) {
      const found = findParentArrayInner(layers[i].children!, id, layers[i]);
      if (found) return found;
    }
  }
  return null;
}

/**
 * Move a layer to a new position in the tree.
 * @param layerId - the layer to move
 * @param targetId - the layer to drop near/into
 * @param position - "before", "after", or "inside" (for containers)
 */
export function moveLayer(layerId: string, targetId: string, position: "before" | "after" | "inside") {
  if (layerId === targetId) return;
  pushUndo();
  // Prevent circular: can't drop a layer into its own descendants
  if (isDescendant(project.layers, layerId, targetId)) return;

  // Remove layer from its current position
  const layer = findInLayers(project.layers, layerId);
  if (!layer) return;
  let newLayers = removeFromLayers(project.layers, layerId);

  if (position === "inside") {
    // Drop into a container
    const target = findInLayers(newLayers, targetId);
    if (!target || !isContainerType(target.type)) return;
    newLayers = addChildToParent(newLayers, targetId, layer);
  } else {
    // Drop before or after a sibling
    const loc = findParentArray(newLayers, targetId);
    if (!loc) return;
    const insertIdx = position === "after" ? loc.idx + 1 : loc.idx;
    loc.arr.splice(insertIdx, 0, layer);
  }

  project.layers = newLayers;
  isDirty = true;
}

export function getSelectedLayer(): Layer | undefined {
  if (!selectedLayerId) return undefined;
  return findInLayers(project.layers, selectedLayerId);
}

export function addGlobal(type: GlobalVarType = "text") {
  const name = `var_${project.globals.length + 1}`;
  const defaults: Record<GlobalVarType, string | number | boolean> = {
    text: "",
    number: 0,
    color: "#ffffff",
    switch: false,
    list: "",
    image: "",
  };
  project.globals = [...project.globals, { name, type, value: defaults[type] }];
  isDirty = true;
  return name;
}

export function removeGlobal(name: string) {
  project.globals = project.globals.filter(g => g.name !== name);
  isDirty = true;
}

/** Insert a fully-constructed layer tree (widget preset) into the project */
export function insertWidget(layer: Layer) {
  pushUndo();
  const selected = getSelectedLayer();
  if (selected && isContainerType(selected.type)) {
    project.layers = addChildToParent(project.layers, selected.id, layer);
  } else {
    project.layers = [...project.layers, layer];
  }
  selectedLayerId = layer.id;
  isDirty = true;
}

/** Ensure a global variable exists; if not, create it with the given defaults */
export function ensureGlobal(name: string, type: GlobalVarType, defaultValue: string | number | boolean) {
  const existing = project.globals.find(g => g.name === name);
  if (existing) {
    // Update the value if the widget provides a newer formula
    if (existing.value !== defaultValue) {
      project.globals = project.globals.map(g =>
        g.name === name ? { ...g, type, value: defaultValue } : g
      );
      isDirty = true;
    }
    return;
  }
  project.globals = [...project.globals, { name, type, value: defaultValue }];
  isDirty = true;
}

export function updateGlobal(name: string, field: string, value: any) {
  project.globals = project.globals.map(g => {
    if (g.name !== name) return g;
    return { ...g, [field]: value };
  });
  isDirty = true;
  invalidateGlobalsFormulas();
}

export function addShortcut(keys: string, action: string, label?: string): string {
  const id = `sc_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
  project.shortcuts = [...project.shortcuts, { id, keys, action, label }];
  isDirty = true;
  return id;
}

export function updateShortcut(id: string, field: string, value: any) {
  project.shortcuts = project.shortcuts.map(s =>
    s.id === id ? { ...s, [field]: value } : s
  );
  isDirty = true;
}

export function removeShortcut(id: string) {
  project.shortcuts = project.shortcuts.filter(s => s.id !== id);
  isDirty = true;
}
