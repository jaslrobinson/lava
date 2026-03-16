export interface AppSettings {
  // Provider refresh intervals (seconds)
  providers: {
    dateTime: number;
    music: number;
    battery: number;
    resources: number;
    traffic: number;
    network: number;
    systemInfo: number;
    weather: number;
  };
  // Weather configuration
  weather: {
    enabled: boolean;
    source: "openweathermap" | "weatherapi" | "openmeteo";
    apiKey: string;
    location: string;
    units: "metric" | "imperial";
  };
  // Formula refresh
  formulaRefreshMs: number;
  // Web Get cache TTL (seconds)
  wgCacheTtl: number;
  // Behavior
  closeToTray: boolean;
  startMinimized: boolean;
  autoStartWallpaper: boolean;
  wallpaperFadeEnabled: boolean;
  wallpaperFadeOpacity: number;
  lastProjectPath: string;
  savedThemes: { name: string; path: string }[];
  replicateApiKey: string;
}

export const defaultSettings: AppSettings = {
  providers: {
    dateTime: 1,
    music: 1,
    battery: 10,
    resources: 2,
    traffic: 2,
    network: 5,
    systemInfo: 5,
    weather: 300,
  },
  weather: {
    enabled: false,
    source: "openweathermap",
    apiKey: "",
    location: "",
    units: "metric",
  },
  formulaRefreshMs: 1000,
  wgCacheTtl: 300,
  closeToTray: true,
  startMinimized: false,
  autoStartWallpaper: false,
  wallpaperFadeEnabled: true,
  wallpaperFadeOpacity: 0.3,
  lastProjectPath: "",
  savedThemes: [],
  replicateApiKey: "",
};
