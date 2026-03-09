export interface LayerAnimState {
  firstSeenTime: number | null;
  tapTime: number | null;
}

const layerStates = new Map<string, LayerAnimState>();

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
  if (state.firstSeenTime === null) {
    state.firstSeenTime = timestamp;
  }
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
