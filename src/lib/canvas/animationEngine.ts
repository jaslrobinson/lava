import type { Animation, Layer } from "../types/project";
import { applyEasing } from "./easing";
import { getScrollX, getLayerAnimState, getEngineStartTime, markLayerSeen } from "./animationState";
import { resolveFormula, hasFormula } from "../formula/service";

export interface AnimatedDeltas {
  dx: number;
  dy: number;
  dRotation: number;
  opacityMultiplier: number;
  opacityOverride: number | null; // absolute 0-255 opacity (overrides base)
  scaleX: number;
  scaleY: number;
  blur: number;
  colorOverride: string | null;
}

function emptyDeltas(): AnimatedDeltas {
  return {
    dx: 0,
    dy: 0,
    dRotation: 0,
    opacityMultiplier: 1,
    opacityOverride: null,
    scaleX: 1,
    scaleY: 1,
    blur: 0,
    colorOverride: null,
  };
}

/** Compute the raw progress (0-1) for an animation based on its trigger */
function computeProgress(anim: Animation, layerId: string, timestamp: number): number | null {
  const speed = anim.speed || 1000;
  const delay = anim.delay || 0;

  switch (anim.trigger) {
    case "scroll":
      return getScrollX();

    case "tap": {
      const state = getLayerAnimState(layerId);
      if (!state.tapTime) return null;
      const elapsed = timestamp - state.tapTime - delay;
      if (elapsed < 0) return null;
      if (anim.loop === "none" || !anim.loop) {
        // One-shot: animate over `speed` ms then hold at end
        return Math.min(1, elapsed / speed);
      }
      return applyLoop(elapsed / speed, anim.loop);
    }

    case "show": {
      const state = getLayerAnimState(layerId);
      markLayerSeen(layerId, timestamp);
      if (!state.firstSeenTime) return null;
      const elapsed = timestamp - state.firstSeenTime - delay;
      if (elapsed < 0) return null;
      if (anim.loop === "none" || !anim.loop) {
        return Math.min(1, elapsed / speed);
      }
      return applyLoop(elapsed / speed, anim.loop);
    }

    case "reactive": {
      // Rule is a formula that resolves to 0-1
      if (!anim.rule) return 0;
      const str = hasFormula(anim.rule) ? resolveFormula(anim.rule) : anim.rule;
      const val = parseFloat(str);
      return isNaN(val) ? 0 : Math.max(0, Math.min(1, val));
    }

    case "time": {
      const elapsed = timestamp - getEngineStartTime() - delay;
      if (elapsed < 0) return null;
      return applyLoop(elapsed / speed, anim.loop || "restart");
    }

    default:
      return null;
  }
}

/** Apply loop mode to a raw cycle value */
function applyLoop(cycles: number, mode: "none" | "restart" | "reverse"): number {
  if (mode === "restart") {
    return cycles % 1;
  }
  if (mode === "reverse") {
    const phase = cycles % 2;
    return phase <= 1 ? phase : 2 - phase;
  }
  // "none" — clamp to end
  return Math.min(1, cycles);
}

/** Apply a single animation's effect to the accumulated deltas */
function applyAnimation(anim: Animation, progress: number, deltas: AnimatedDeltas, trigger: string) {
  const t = applyEasing(progress, anim.easing || "linear");
  const amount = anim.amount;

  switch (anim.type) {
    case "fade":
      // For show/tap triggers: fade IN from 0 to amount (absolute opacity)
      // For other triggers: fade as multiplier toward amount/255
      if (trigger === "show" || trigger === "tap") {
        deltas.opacityOverride = (amount / 255) * t * 255;
      } else {
        deltas.opacityMultiplier *= 1 - t * (1 - amount / 255);
      }
      break;

    case "rotate":
      deltas.dRotation += amount * t;
      break;

    case "scale": {
      const s = 1 + (amount - 1) * t;
      deltas.scaleX *= s;
      deltas.scaleY *= s;
      break;
    }

    case "translate": {
      // rule determines direction: "x", "y", or "x,y" (default: x)
      const dir = (anim.rule || "x").toLowerCase();
      if (dir.includes("x")) deltas.dx += amount * t;
      if (dir.includes("y")) deltas.dy += amount * t;
      break;
    }

    case "blur":
      deltas.blur += amount * t;
      break;

    case "color":
      // rule is target color hex, amount is interpolation strength
      if (anim.rule && t > 0) {
        deltas.colorOverride = anim.rule;
      }
      break;
  }
}

/** Compute all animation deltas for a layer at the given timestamp */
export function computeAnimatedDeltas(layer: Layer, timestamp: number): AnimatedDeltas {
  const deltas = emptyDeltas();
  if (!layer.animations || layer.animations.length === 0) return deltas;

  for (const anim of layer.animations) {
    const progress = computeProgress(anim, layer.id, timestamp);
    if (progress === null) continue;
    applyAnimation(anim, progress, deltas, anim.trigger);
  }

  return deltas;
}
