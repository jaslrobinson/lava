import { markDirty } from "./renderScheduler";

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
  markDirty();
  const state = getLayerAnimState(layerId);
  state.tapTime = timestamp;
}

/** Called each frame with the set of hovered layer IDs (hit layer + all ancestors) */
export function updateHoverState(hoveredIds: Set<string>, timestamp: number) {
  // For each tracked layer, update hoverEnterTime/hoverExitTime based on whether it's hovered
  for (const [layerId, state] of layerStates) {
    const isHovered = hoveredIds.has(layerId);
    const wasHovered = state.hoverEnterTime !== null && state.hoverExitTime === null;
    if (isHovered && !wasHovered) {
      state.hoverEnterTime = timestamp;
      state.hoverExitTime = null;
      markDirty();
    } else if (!isHovered && wasHovered) {
      state.hoverExitTime = timestamp;
      markDirty();
    }
  }
  // Ensure all hovered layers have a state entry
  for (const layerId of hoveredIds) {
    const state = getLayerAnimState(layerId);
    if (state.hoverEnterTime === null) {
      state.hoverEnterTime = timestamp;
      state.hoverExitTime = null;
      markDirty();
    }
  }
}

export function resetAnimationState() {
  layerStates.clear();
  scrollX = 0;
  engineStartTime = null;
}

/** Check if a layer is currently hovered */
export function isLayerHovered(layerId: string): boolean {
  const state = layerStates.get(layerId);
  return state ? state.hoverEnterTime !== null && state.hoverExitTime === null : false;
}

/** Get hover progress (0-1) for a layer based on animation speed */
export function getHoverProgress(layerId: string, timestamp: number, speed: number = 200): number {
  const state = layerStates.get(layerId);
  if (!state || !state.hoverEnterTime) return 0;

  if (state.hoverExitTime === null) {
    // Currently hovered — animate toward 1
    const enterElapsed = timestamp - state.hoverEnterTime;
    return Math.min(1, enterElapsed / speed);
  } else {
    // Hover exited — freeze enter progress at exit time, then retreat
    const enterElapsed = state.hoverExitTime - state.hoverEnterTime;
    const enterProgress = Math.min(1, enterElapsed / speed);
    const exitElapsed = timestamp - state.hoverExitTime;
    const exitProgress = Math.min(enterProgress, exitElapsed / speed);
    return Math.max(0, enterProgress - exitProgress);
  }
}

/** Get raw hover state for formula evaluation */
export function getHoverState(layerId: string): { isHovered: boolean; enterTime: number | null; exitTime: number | null } {
  const state = layerStates.get(layerId);
  if (!state) {
    return { isHovered: false, enterTime: null, exitTime: null };
  }
  return {
    isHovered: state.hoverEnterTime !== null && state.hoverExitTime === null,
    enterTime: state.hoverEnterTime,
    exitTime: state.hoverExitTime
  };
}
