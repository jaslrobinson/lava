import type { BrushType } from '../types/paint';

export interface BrushPreset {
    name: string;
    type: BrushType;
    size: number;
    opacity: number;
    scatter: number;      // 0-1, randomness of placement
    softness: number;     // 0-1, edge falloff
    spacing: number;      // 0-1, distance between dabs as fraction of size
    density: number;      // for spray: number of particles per dab
}

export const brushPresets: BrushPreset[] = [
    {
        name: 'Round Brush',
        type: 'solid',
        size: 20,
        opacity: 1.0,
        scatter: 0,
        softness: 0,
        spacing: 0.15,
        density: 1,
    },
    {
        name: 'Soft Brush',
        type: 'solid',
        size: 30,
        opacity: 0.8,
        scatter: 0,
        softness: 0.8,
        spacing: 0.1,
        density: 1,
    },
    {
        name: 'Spray Can',
        type: 'spray',
        size: 40,
        opacity: 0.3,
        scatter: 0.8,
        softness: 0,
        spacing: 0.05,
        density: 30,
    },
    {
        name: 'Airbrush',
        type: 'airbrush',
        size: 50,
        opacity: 0.15,
        scatter: 0.2,
        softness: 1.0,
        spacing: 0.05,
        density: 1,
    },
    {
        name: 'Splatter',
        type: 'splatter',
        size: 60,
        opacity: 0.9,
        scatter: 1.0,
        softness: 0,
        spacing: 0.5,
        density: 15,
    },
    {
        name: 'Fine Pen',
        type: 'solid',
        size: 3,
        opacity: 1.0,
        scatter: 0,
        softness: 0,
        spacing: 0.05,
        density: 1,
    },
];

// Generate splatter points around a center
export function generateSplatterPoints(
    centerX: number,
    centerY: number,
    radius: number,
    count: number,
    seed: number
): { x: number; y: number; size: number }[] {
    const points: { x: number; y: number; size: number }[] = [];
    // Simple seeded random
    let s = seed;
    const rand = () => {
        s = (s * 16807 + 0) % 2147483647;
        return s / 2147483647;
    };
    for (let i = 0; i < count; i++) {
        const angle = rand() * Math.PI * 2;
        const dist = rand() * radius;
        const size = 1 + rand() * (radius * 0.3);
        points.push({
            x: centerX + Math.cos(angle) * dist,
            y: centerY + Math.sin(angle) * dist,
            size,
        });
    }
    return points;
}
