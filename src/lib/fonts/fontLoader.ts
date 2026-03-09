import { convertFileSrc } from "@tauri-apps/api/core";

const loadedFonts = new Map<string, FontFace>();
let projectFontNames: string[] = [];

/** Load a font from a file path (absolute) and register it */
export async function loadFont(name: string, filePath: string): Promise<boolean> {
  if (loadedFonts.has(name)) return true;
  try {
    const url = convertFileSrc(filePath);
    const face = new FontFace(name, `url(${url})`);
    await face.load();
    document.fonts.add(face);
    loadedFonts.set(name, face);
    return true;
  } catch (e) {
    console.warn(`Failed to load font "${name}" from ${filePath}:`, e);
    return false;
  }
}

/** Load a font from a URL (for bundled fonts) */
export async function loadFontFromUrl(name: string, url: string, weight?: string): Promise<boolean> {
  const key = weight ? `${name}:${weight}` : name;
  if (loadedFonts.has(key)) return true;
  try {
    const descriptors: FontFaceDescriptors = {};
    if (weight) descriptors.weight = weight;
    const face = new FontFace(name, `url(${url})`, descriptors);
    await face.load();
    document.fonts.add(face);
    loadedFonts.set(key, face);
    return true;
  } catch (e) {
    console.warn(`Failed to load font "${name}" from URL:`, e);
    return false;
  }
}

/** Scan project asset dir for fonts and load them all */
export async function loadProjectFonts(assetDir: string): Promise<string[]> {
  // Unload previous project fonts first
  unloadProjectFonts();

  if (!assetDir) return [];

  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const fonts = await invoke<string[]>("list_project_fonts", { assetDir });
    const loaded: string[] = [];
    for (const fontPath of fonts) {
      // Derive name from filename (strip extension)
      const name = fontPath.split("/").pop()?.replace(/\.(ttf|otf|woff2?|TTF|OTF|WOFF2?)$/, "") ?? fontPath;
      if (await loadFont(name, fontPath)) {
        loaded.push(name);
      }
    }
    projectFontNames = loaded;
    return loaded;
  } catch (e) {
    console.warn("Failed to load project fonts:", e);
    return [];
  }
}

/** Unload all project fonts */
export function unloadProjectFonts() {
  for (const name of projectFontNames) {
    const face = loadedFonts.get(name);
    if (face) {
      document.fonts.delete(face);
      loadedFonts.delete(name);
    }
  }
  projectFontNames = [];
}

/** Get list of loaded project font names */
export function getProjectFontNames(): string[] {
  return [...projectFontNames];
}

/** Check if a font is loaded/available */
export function isFontLoaded(name: string): boolean {
  return loadedFonts.has(name);
}

/** Load bundled icon fonts (Material Icons, Font Awesome) */
export async function loadBundledIconFonts(): Promise<void> {
  // These will be loaded from bundled assets
  // The actual font files will be imported as URLs by Vite
  try {
    const materialUrl = new URL("../assets/fonts/MaterialIcons-Regular.woff2", import.meta.url).href;
    await loadFontFromUrl("Material Icons", materialUrl);
  } catch (e) {
    console.warn("Material Icons font not bundled yet:", e);
  }

  try {
    const faUrl = new URL("../assets/fonts/fa-solid-900.woff2", import.meta.url).href;
    await loadFontFromUrl("Font Awesome 6 Free", faUrl, "900");
  } catch (e) {
    console.warn("Font Awesome font not bundled yet:", e);
  }

  try {
    const faRegUrl = new URL("../assets/fonts/fa-regular-400.woff2", import.meta.url).href;
    await loadFontFromUrl("Font Awesome 6 Free Regular", faRegUrl, "400");
  } catch (e) {
    console.warn("Font Awesome Regular font not bundled yet:", e);
  }
}
