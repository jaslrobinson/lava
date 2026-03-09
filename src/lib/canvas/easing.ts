import type { EasingType } from "../types/project";

const easingFunctions: Record<EasingType, (t: number) => number> = {
  linear: (t) => t,

  "ease-in": (t) => t * t * t,

  "ease-out": (t) => 1 - Math.pow(1 - t, 3),

  "ease-in-out": (t) =>
    t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2,

  bounce: (t) => {
    const n1 = 7.5625;
    const d1 = 2.75;
    if (t < 1 / d1) return n1 * t * t;
    if (t < 2 / d1) return n1 * (t -= 1.5 / d1) * t + 0.75;
    if (t < 2.5 / d1) return n1 * (t -= 2.25 / d1) * t + 0.9375;
    return n1 * (t -= 2.625 / d1) * t + 0.984375;
  },

  elastic: (t) => {
    if (t === 0 || t === 1) return t;
    return -Math.pow(2, 10 * t - 10) * Math.sin((t * 10 - 10.75) * ((2 * Math.PI) / 3));
  },
};

export function applyEasing(t: number, easing: EasingType = "linear"): number {
  const clamped = Math.max(0, Math.min(1, t));
  return (easingFunctions[easing] || easingFunctions.linear)(clamped);
}
