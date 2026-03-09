export interface LayerAnimState {
  firstSeenTime: number | null;
  tapTime: number | null;
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
    state = { firstSeenTime: null, tapTime: null };
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

export function resetAnimationState() {
  layerStates.clear();
  scrollX = 0;
  engineStartTime = null;
}
