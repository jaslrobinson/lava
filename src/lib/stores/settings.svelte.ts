import { type AppSettings, defaultSettings } from "../types/settings";

const isTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";

let settings = $state<AppSettings>(structuredClone(defaultSettings));
// Track whether initial load is done (used by consumers to avoid flash)

export function getSettings(): AppSettings { return settings; }

export function updateSetting(path: string, value: any) {
  const keys = path.split(".");
  const copy: any = $state.snapshot(settings);
  let target = copy;
  for (let i = 0; i < keys.length - 1; i++) {
    target = target[keys[i]];
  }
  target[keys[keys.length - 1]] = value;
  settings = copy;
  saveSettings();
}

export async function loadSettings() {
  try {
    let raw: string | null = null;
    if (isTauri) {
      const { invoke } = await import("@tauri-apps/api/core");
      raw = await invoke<string>("load_settings");
    } else {
      raw = localStorage.getItem("kllw-settings");
    }
    if (raw && raw !== "{}") {
      const parsed = JSON.parse(raw);
      settings = deepMerge(structuredClone(defaultSettings), parsed);
    }
  } catch {
    settings = structuredClone(defaultSettings);
  }
  // loaded
}

export async function saveSettings() {
  const json = JSON.stringify($state.snapshot(settings), null, 2);
  try {
    if (isTauri) {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("save_settings", { data: json });
    } else {
      localStorage.setItem("kllw-settings", json);
    }
  } catch {
    // Silently fail — config dir may not be writable
  }
}

export function resetSettings() {
  settings = structuredClone(defaultSettings);
  saveSettings();
}

export function addTheme(name: string, path: string) {
  const copy: any = $state.snapshot(settings);
  const themes: { name: string; path: string }[] = copy.savedThemes ?? [];
  if (!themes.find((t: any) => t.path === path)) {
    copy.savedThemes = [...themes, { name, path }];
    settings = copy;
    saveSettings();
  }
}

export function removeTheme(path: string) {
  const copy: any = $state.snapshot(settings);
  copy.savedThemes = (copy.savedThemes ?? []).filter((t: any) => t.path !== path);
  settings = copy;
  saveSettings();
}

/** Deep merge source into target, keeping target's structure as the template */
function deepMerge(target: any, source: any): any {
  if (source === null || source === undefined) return target;
  if (typeof target !== "object" || typeof source !== "object") return source;
  if (Array.isArray(target)) return source;
  const result: any = { ...target };
  for (const key of Object.keys(target)) {
    if (key in source) {
      result[key] = deepMerge(target[key], source[key]);
    }
  }
  return result;
}

// Load on module init
loadSettings();
