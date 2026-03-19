export type BrushType = 'solid' | 'spray' | 'airbrush' | 'splatter';

export interface PaintPoint {
    x: number;
    y: number;
    pressure: number;
}

export interface PaintStroke {
    points: PaintPoint[];
    brushType: BrushType;
    brushSize: number;
    color: string;
    opacity: number;
    splatterSeed?: number;
}
