import type { PaintStroke, PaintPoint } from '../types/paint';
import { generateSplatterPoints } from '../data/brushPresets';

/**
 * Interpolate between two points to fill gaps during fast mouse movement.
 * Returns additional points between prev and curr, spaced by `spacing` pixels.
 */
export function interpolatePoints(
    prev: PaintPoint,
    curr: PaintPoint,
    spacing: number
): PaintPoint[] {
    const dx = curr.x - prev.x;
    const dy = curr.y - prev.y;
    const dist = Math.sqrt(dx * dx + dy * dy);
    if (dist < spacing) return [];

    const steps = Math.floor(dist / spacing);
    const points: PaintPoint[] = [];
    for (let i = 1; i <= steps; i++) {
        const t = i / (steps + 1);
        points.push({
            x: prev.x + dx * t,
            y: prev.y + dy * t,
            pressure: prev.pressure + (curr.pressure - prev.pressure) * t,
        });
    }
    return points;
}

/**
 * Draw a single brush dab at a given position
 */
function drawDab(
    ctx: CanvasRenderingContext2D,
    x: number,
    y: number,
    size: number,
    color: string,
    opacity: number,
    brushType: string,
    softness: number = 0,
    seed: number = 0
): void {
    const radius = size / 2;

    switch (brushType) {
        case 'solid': {
            ctx.globalAlpha = opacity;
            if (softness > 0) {
                // Soft brush: radial gradient falloff
                const gradient = ctx.createRadialGradient(x, y, 0, x, y, radius);
                gradient.addColorStop(0, color);
                gradient.addColorStop(1 - softness * 0.5, color);
                gradient.addColorStop(1, 'transparent');
                ctx.fillStyle = gradient;
            } else {
                ctx.fillStyle = color;
            }
            ctx.beginPath();
            ctx.arc(x, y, radius, 0, Math.PI * 2);
            ctx.fill();
            break;
        }
        case 'spray': {
            // Scatter random dots within the radius
            ctx.fillStyle = color;
            const count = Math.max(5, Math.floor(size * 0.8));
            let s = seed + x * 7 + y * 13;
            const rand = () => {
                s = (s * 16807 + 0) % 2147483647;
                return s / 2147483647;
            };
            for (let i = 0; i < count; i++) {
                const angle = rand() * Math.PI * 2;
                const dist = rand() * radius;
                const dotSize = 0.5 + rand() * 1.5;
                ctx.globalAlpha = opacity * (0.3 + rand() * 0.7);
                ctx.beginPath();
                ctx.arc(
                    x + Math.cos(angle) * dist,
                    y + Math.sin(angle) * dist,
                    dotSize,
                    0,
                    Math.PI * 2
                );
                ctx.fill();
            }
            break;
        }
        case 'airbrush': {
            // Gaussian-like soft circle with very low opacity, builds up
            const gradient = ctx.createRadialGradient(x, y, 0, x, y, radius);
            gradient.addColorStop(0, color);
            gradient.addColorStop(0.3, color);
            gradient.addColorStop(1, 'transparent');
            ctx.globalAlpha = opacity * 0.3;
            ctx.fillStyle = gradient;
            ctx.beginPath();
            ctx.arc(x, y, radius, 0, Math.PI * 2);
            ctx.fill();
            break;
        }
        case 'splatter': {
            // Multiple varied-size dots in a random pattern
            const points = generateSplatterPoints(x, y, radius, Math.max(8, Math.floor(size * 0.4)), seed);
            ctx.fillStyle = color;
            for (const p of points) {
                ctx.globalAlpha = opacity * (0.5 + Math.random() * 0.5);
                ctx.beginPath();
                ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
                ctx.fill();
            }
            break;
        }
    }
}

/**
 * Render a complete stroke to the canvas context.
 * Walks the points array and stamps a brush dab at each point.
 */
export function renderStroke(
    ctx: CanvasRenderingContext2D,
    stroke: PaintStroke
): void {
    if (stroke.points.length === 0) return;

    ctx.save();

    const spacing = Math.max(1, stroke.brushSize * 0.15);
    let prevPoint = stroke.points[0];

    // Draw first dab
    drawDab(
        ctx, prevPoint.x, prevPoint.y,
        stroke.brushSize * prevPoint.pressure,
        stroke.color, stroke.opacity,
        stroke.brushType, 0,
        stroke.splatterSeed || 0
    );

    // Draw along all points with interpolation
    for (let i = 1; i < stroke.points.length; i++) {
        const currPoint = stroke.points[i];
        const interpolated = interpolatePoints(prevPoint, currPoint, spacing);

        // Draw interpolated points
        for (const p of interpolated) {
            drawDab(
                ctx, p.x, p.y,
                stroke.brushSize * p.pressure,
                stroke.color, stroke.opacity,
                stroke.brushType, 0,
                (stroke.splatterSeed || 0) + i
            );
        }

        // Draw the actual recorded point
        drawDab(
            ctx, currPoint.x, currPoint.y,
            stroke.brushSize * currPoint.pressure,
            stroke.color, stroke.opacity,
            stroke.brushType, 0,
            (stroke.splatterSeed || 0) + i
        );

        prevPoint = currPoint;
    }

    ctx.restore();
}

/**
 * Render all strokes in a paint layer
 */
export function renderPaintStrokes(
    ctx: CanvasRenderingContext2D,
    strokes: PaintStroke[]
): void {
    for (const stroke of strokes) {
        renderStroke(ctx, stroke);
    }
}
