export interface LayerAnimState {
  firstSeenTime: number | null;
  tapTime: number | null;
  hoverEnterTime: number | null;
  hoverExitTime: number | null;
}

const layerStates = new Map<string, LayerAnimState>();

// Track which layers were rendered this frame vs last frame
let renderedThisFrame = new Set<string>();
let renderedLastFrame = new Set<string>();

// Global input state
let scrollX = 0;
let engineStartTime: number | null = null;

export function getEngineStartTime(): number {
  return engineStartTime ?? 0;
}

export function initEngineTime(timestamp: number) {
  if (engineStartTime === null) engineStartTime = timestamp;
}

export function getScrollX(): number {
  return scrollX;
}

export function setScrollPosition(x: number) {
  scrollX = Math.max(0, Math.min(1, x));
}

export function getLayerAnimState(layerId: string): LayerAnimState {
  let state = layerStates.get(layerId);
  if (!state) {
    state = { firstSeenTime: null, tapTime: null, hoverEnterTime: null, hoverExitTime: null };
    layerStates.set(layerId, state);
  }
  return state;
}

export function markLayerSeen(layerId: string, timestamp: number) {
  const state = getLayerAnimState(layerId);
  renderedThisFrame.add(layerId);
  // If layer wasn't rendered last frame, reset firstSeenTime so "show" replays
  if (!renderedLastFrame.has(layerId)) {
    state.firstSeenTime = timestamp;
  } else if (state.firstSeenTime === null) {
    state.firstSeenTime = timestamp;
  }
}

/** Call at the start of each render frame to begin tracking */
export function beginFrame() {
  renderedLastFrame = renderedThisFrame;
  renderedThisFrame = new Set<string>();
}

export function triggerTap(layerId: string, timestamp: number) {
  const state = getLayerAnimState(layerId);
  state.tapTime = timestamp;
}

/** Called each frame with the currently hovered layer ID */
export function updateHoverState(hoveredLayerId: string | null, timestamp: number) {
  // For each tracked layer, update hoverEnterTime/hoverExitTime based on whether it's hovered
  for (const [layerId, state] of layerStates) {
    const isHovered = layerId === hoveredLayerId;
    const wasHovered = state.hoverEnterTime !== null && state.hoverExitTime === null;
    if (isHovered && !wasHovered) {
      state.hoverEnterTime = timestamp;
      state.hoverExitTime = null;
    } else if (!isHovered && wasHovered) {
      state.hoverExitTime = timestamp;
    }
  }
  // Ensure the currently hovered layer has a state entry
  if (hoveredLayerId) {
    const state = getLayerAnimState(hoveredLayerId);
    if (state.hoverEnterTime === null) {
      state.hoverEnterTime = timestamp;
      state.hoverExitTime = null;
    }
  }
}

export function resetAnimationState() {
  layerStates.clear();
  scrollX = 0;
  engineStartTime = null;
}
