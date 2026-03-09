import { type Project, type Layer, type Animation, createDefaultProject, createLayer, type LayerType, type GlobalVarType } from "../types/project";
import { clearImageCache } from "../canvas/renderer";
import { invalidateGlobalsFormulas } from "../formula/service";
import { resetAnimationState } from "../canvas/animationState";

let project = $state<Project>(createDefaultProject());
let selectedLayerId = $state<string | null>(null);
let isDirty = $state(false);
let wallpaperMode = $state(false);

export function getProject() { return project; }
export function setProject(p: Project) { project = p; isDirty = false; clearImageCache(); resetAnimationState(); }
export function getSelectedLayerId() { return selectedLayerId; }
export function setSelectedLayerId(id: string | null) { selectedLayerId = id; }
export function getIsDirty() { return isDirty; }
export function getWallpaperMode() { return wallpaperMode; }
export function setWallpaperMode(active: boolean) { wallpaperMode = active; }

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
  return type === "group" || type === "stack" || type === "overlap";
}

export function addLayer(type: LayerType) {
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
  project.layers = removeFromLayers(project.layers, id);
  if (selectedLayerId === id) selectedLayerId = null;
  isDirty = true;
}

export function updateLayerProperty(id: string, key: string, value: any) {
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
  project.layers = updateLayerAnimations(project.layers, layerId, (anims) => [...anims, anim]);
  isDirty = true;
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
  project.layers = [...project.layers, layer];
  selectedLayerId = layer.id;
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
