export type LayerType = "text" | "shape" | "image" | "stack" | "overlap" | "progress" | "fonticon" | "visualizer" | "map" | "launcher";

export type ShapeKind = "rectangle" | "circle" | "oval" | "triangle" | "arc";

export type AnchorPoint = "center" | "top-left" | "top-center" | "top-right" | "center-left" | "center-right" | "bottom-left" | "bottom-center" | "bottom-right";

export type AnimationTrigger = "time" | "scroll" | "reactive" | "tap" | "show" | "hover";

export type AnimationType = "fade" | "rotate" | "scale" | "translate" | "color" | "blur" | "jiggle" | "flash";

export type EasingType = "linear" | "ease-in" | "ease-out" | "ease-in-out" | "bounce" | "elastic";

export type GlobalVarType = "text" | "number" | "color" | "switch" | "list" | "image";

export interface GlobalVariable {
  name: string;
  type: GlobalVarType;
  value: string | number | boolean;
  options?: string[];
}

export interface Shadow {
  color: string;
  dx: number;
  dy: number;
  radius: number;
}

export interface Animation {
  type: AnimationType;
  trigger: AnimationTrigger;
  rule: string;
  amount: number;
  speed?: number;
  easing?: EasingType;
  delay?: number;
  loop?: "none" | "restart" | "reverse";
  /** Target color for color-type animations when trigger=reactive (rule holds the formula) */
  colorTarget?: string;
}

export interface LayerProperties {
  x: number | string;
  y: number | string;
  width: number | string;
  height: number | string;
  rotation?: number | string;
  scaleX?: number | string;
  scaleY?: number | string;
  opacity?: number | string;
  anchor?: AnchorPoint;
  visible?: boolean | string;

  // Text
  text?: string;
  fontSize?: number | string;
  fontFamily?: string;
  color?: string;
  textAlign?: "left" | "center" | "right";
  maxLines?: number;
  lineSpacing?: number;
  textStroke?: string;      // stroke/outline color (hex)
  textStrokeWidth?: number; // stroke width in pixels
  textFillEnabled?: boolean; // whether to fill text (default true)
  shadow?: Shadow;

  // Shape
  shapeKind?: ShapeKind;
  fill?: string;
  stroke?: string;
  strokeWidth?: number;
  cornerRadius?: number;
  skewX?: number | string;  // degrees horizontal skew
  skewY?: number | string;  // degrees vertical skew

  // Image
  src?: string;
  scaleMode?: "fit" | "fill" | "crop" | "stretch";
  tint?: string;

  // Progress
  style?: "arc" | "bar" | "circle";
  min?: number;
  max?: number;
  value?: number | string;
  trackColor?: string;

  // FontIcon
  iconSet?: string;
  glyphCode?: string;

  // Stack/Group
  orientation?: "horizontal" | "vertical";
  spacing?: number;

  // Click action (for fullscreen/wallpaper mode)
  clickAction?: string; // e.g. "overlay:news", "url:https://..."

  // Scroll action (mouse wheel/touchpad gesture)
  scrollAction?: string; // e.g. "volume:adjust", "brightness:adjust"

  // Icon source (SVG/PNG path for imported icons)
  iconSrc?: string;

  // Visualizer
  vizStyle?: "bars" | "wave";
  barCount?: number;
  barSpacing?: number;
  sensitivity?: number;
  colorTop?: string;
  colorMid?: string;
  colorBottom?: string;
  peakColor?: string;

  // Map
  mapLat?: number;       // center latitude (default 40.7)
  mapLng?: number;       // center longitude (default -74.0)
  mapZoom?: number;      // zoom level 0-18 (default 5)
  mapShowRadar?: boolean; // show RainViewer radar overlay
  mapRadarAnimate?: boolean; // animate radar frames
  mapStyle?: "dark" | "streets" | "satellite"; // base tile style

  // Launcher
  launcherStyle?: "win11" | "macos" | "deepin"; // visual style
  pinnedApps?: string[]; // list of app exec commands e.g. ["firefox", "thunar"]
  launcherIconSize?: number; // icon size in pixels (default 36)

}

export interface Layer {
  id: string;
  name: string;
  type: LayerType;
  properties: LayerProperties;
  animations?: Animation[];
  children?: Layer[];
  locked?: boolean;
  visible?: boolean;
}

export interface Shortcut {
  id: string;
  keys: string;       // e.g. "Super+1", "Ctrl+Shift+M"
  action: string;     // e.g. "music:play-pause", "app:firefox", "overlay:panel"
  label?: string;     // optional display name
}

export interface Project {
  version: string;
  name: string;
  resolution: { width: number; height: number };
  background: { type: "color" | "image"; value: string };
  globals: GlobalVariable[];
  layers: Layer[];
  shortcuts: Shortcut[];
  assetDir?: string;
}

export function createDefaultProject(): Project {
  return {
    version: "0.1.0",
    name: "Untitled",
    resolution: { width: 1920, height: 1080 },
    background: { type: "color", value: "#1a1a2e" },
    globals: [],
    layers: [],
    shortcuts: [],
  };
}

let nextId = 0;
export function generateId(): string {
  return `layer_${Date.now()}_${nextId++}`;
}

/** Deep clone a layer tree, assigning fresh IDs to every node */
export function cloneLayerWithNewIds(layer: Layer, newName?: string): Layer {
  return {
    ...layer,
    id: generateId(),
    name: newName ?? layer.name,
    properties: { ...layer.properties },
    animations: layer.animations?.map(a => ({ ...a })),
    children: layer.children?.map(c => cloneLayerWithNewIds(c)),
  };
}

export function createLayer(type: LayerType, name: string): Layer {
  const base: Layer = {
    id: generateId(),
    name,
    type,
    properties: {
      x: 100,
      y: 100,
      width: 200,
      height: 50,
      opacity: 255,
      anchor: "top-left",
    },
    visible: true,
    locked: false,
  };

  switch (type) {
    case "text":
      base.properties.text = "Hello";
      base.properties.fontSize = 24;
      base.properties.fontFamily = "sans-serif";
      base.properties.color = "#ffffff";
      base.properties.textAlign = "left";
      break;
    case "shape":
      base.properties.shapeKind = "rectangle";
      base.properties.fill = "#e94560";
      base.properties.width = 200;
      base.properties.height = 200;
      break;
    case "image":
      base.properties.src = "";
      base.properties.scaleMode = "fit";
      base.properties.width = 200;
      base.properties.height = 200;
      break;
    case "progress":
      base.properties.style = "arc";
      base.properties.min = 0;
      base.properties.max = 100;
      base.properties.value = 50;
      base.properties.color = "#e94560";
      base.properties.trackColor = "#ffffff20";
      base.properties.strokeWidth = 6;
      base.properties.width = 80;
      base.properties.height = 80;
      break;
    case "stack":
      base.children = [];
      base.properties.orientation = "vertical";
      base.properties.spacing = 0;
      base.properties.width = 400;
      base.properties.height = 400;
      break;
    case "overlap":
      base.children = [];
      base.properties.width = 400;
      base.properties.height = 400;
      break;
    case "fonticon":
      base.properties.iconSet = "material";
      base.properties.glyphCode = "e88a";
      base.properties.color = "#ffffff";
      base.properties.fontSize = 48;
      base.properties.width = 60;
      base.properties.height = 60;
      break;
    case "visualizer":
      base.properties.width = 400;
      base.properties.height = 120;
      base.properties.barCount = 24;
      base.properties.barSpacing = 3;
      base.properties.sensitivity = 1.2;
      break;
    case "map":
      base.properties.mapLat = 40.7;
      base.properties.mapLng = -74.0;
      base.properties.mapZoom = 5;
      base.properties.mapShowRadar = true;
      base.properties.mapRadarAnimate = false;
      base.properties.mapStyle = "dark";
      base.properties.width = 600;
      base.properties.height = 400;
      break;
    case "launcher":
      base.properties.launcherStyle = "win11";
      base.properties.pinnedApps = ["firefox", "thunar", "gnome-terminal"];
      base.properties.launcherIconSize = 36;
      base.properties.x = 0;
      base.properties.y = 1032;
      base.properties.width = 1920;
      base.properties.height = 48;
      base.properties.anchor = "top-left";
      break;
  }

  return base;
}
