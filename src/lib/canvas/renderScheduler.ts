/**
 * Render scheduler for wallpaper mode.
 * Two-tier system:
 *   - dirty window (dirtyUntil) → 60fps for 500ms (user interaction, animations)
 *   - repaint flag → render on next idle tick (formula/provider changes)
 * When neither is set, idle ticks skip rendering entirely (~0% CPU).
 */

import type { Layer } from "../types/project";

let dirtyUntil = 0; // timestamp: stay in 60fps mode until this time
let repaint = true; // content changed, render on next opportunity (no fps boost)
let wakeCallback: (() => void) | null = null; // called when transitioning from idle to active
let idleTimeoutId: ReturnType<typeof setTimeout> | null = null;

const DIRTY_COOLDOWN_MS = 500; // stay in 60fps for 500ms after last dirty signal
export const IDLE_INTERVAL_MS = 250; // ~4fps idle ticks

/**
 * Mark the renderer as dirty — need full-fps rendering.
 * Call this from any code that changes visual state (user interaction, animations).
 */
export function markDirty() {
  repaint = true;
  dirtyUntil = performance.now() + DIRTY_COOLDOWN_MS;

  // If we're in idle mode (setTimeout pending), cancel it and wake up immediately
  if (idleTimeoutId !== null) {
    clearTimeout(idleTimeoutId);
    idleTimeoutId = null;
    if (wakeCallback) wakeCallback();
  }
}

/**
 * Mark that content changed (formula values, provider data) — render on next tick.
 * Unlike markDirty, does NOT boost to 60fps. The next idle tick will render.
 */
export function markRepaint() {
  repaint = true;
}

/** Check if a repaint is needed (for idle-mode skip logic). */
export function needsRepaint(): boolean {
  return repaint;
}

/** Clear the repaint flag after rendering. */
export function clearRepaint() {
  repaint = false;
}

/**
 * Register a callback that's called when transitioning from idle to active.
 * The callback should call requestAnimationFrame(renderLoop).
 * Pass null to unregister (e.g. on component teardown).
 */
export function setWakeCallback(cb: (() => void) | null) {
  wakeCallback = cb;
}

/**
 * Schedule cb to run after IDLE_INTERVAL_MS, tracking the timer so markDirty()
 * can cancel it and wake early. Clears the tracked id itself right before cb
 * runs, so a timer that fires naturally is never mistaken by a later
 * markDirty() call for one that's still pending (which would otherwise fire
 * a spurious extra wakeCallback and spawn a duplicate render loop).
 */
export function scheduleIdle(cb: () => void) {
  idleTimeoutId = setTimeout(() => {
    idleTimeoutId = null;
    cb();
  }, IDLE_INTERVAL_MS);
}

/**
 * Cancel any pending idle timer without waking (e.g. on component teardown),
 * so it can't fire against a torn-down renderer.
 */
export function cancelIdle() {
  if (idleTimeoutId !== null) {
    clearTimeout(idleTimeoutId);
    idleTimeoutId = null;
  }
}

/**
 * Check if the renderer should run at full fps.
 * Called after each frame render to decide: rAF (60fps) vs setTimeout (idle).
 */
export function shouldRenderFullFps(
  timestamp: number,
  hasLoopingAnims: boolean
): boolean {
  if (timestamp < dirtyUntil) return true;
  if (hasLoopingAnims) return true;
  return false;
}

/**
 * Walk the layer tree to detect animations that never settle:
 * - time trigger with loop restart/reverse
 * - jiggle/flash (always loop)
 * Only checks visible layers.
 */
export function hasActiveLoopingAnimations(layers: Layer[]): boolean {
  for (const layer of layers) {
    // Skip hidden layers (editor-hidden, or literally hidden via properties.visible).
    // Note: a formula string in properties.visible that *resolves* to hidden isn't
    // caught here (no globals/resolve() available at this layer) — only the
    // literal-false cases are, which still avoids the worst always-on-60fps case.
    if (layer.visible === false) continue;
    if (layer.properties.visible === false) continue;

    if (layer.animations) {
      for (const anim of layer.animations) {
        // Time-based looping animations never settle
        if (
          anim.trigger === "time" &&
          anim.loop &&
          anim.loop !== "none"
        ) {
          return true;
        }
        // Jiggle and flash are inherently continuous
        if (
          anim.type === "jiggle" ||
          anim.type === "flash"
        ) {
          return true;
        }
      }
    }

    // Recurse into children
    if (layer.children && hasActiveLoopingAnimations(layer.children)) {
      return true;
    }
  }
  return false;
}
