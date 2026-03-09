<script lang="ts">
  import { getSettings, updateSetting, resetSettings } from "../stores/settings.svelte";

  let showApiKey = $state(false);

  const providerLabels: Record<string, string> = {
    dateTime: "Date/Time",
    music: "Music",
    battery: "Battery",
    resources: "Resources",
    traffic: "Traffic",
    network: "Network",
    systemInfo: "System Info",
    weather: "Weather",
  };
</script>

<div class="settings-panel">
  <div class="panel-header">
    <span>Settings</span>
  </div>
  <div class="settings-scroll">

    <!-- General -->
    <div class="section">
      <div class="section-header">General</div>
      <label class="toggle-row">
        <span>Close to tray</span>
        <input type="checkbox" checked={getSettings().closeToTray}
          onchange={(e) => updateSetting("closeToTray", (e.target as HTMLInputElement).checked)} />
      </label>
      <label class="toggle-row">
        <span>Start minimized</span>
        <input type="checkbox" checked={getSettings().startMinimized}
          onchange={(e) => updateSetting("startMinimized", (e.target as HTMLInputElement).checked)} />
      </label>
      <label class="toggle-row">
        <span>Auto-start wallpaper on launch</span>
        <input type="checkbox" checked={getSettings().autoStartWallpaper}
          onchange={(e) => updateSetting("autoStartWallpaper", (e.target as HTMLInputElement).checked)} />
      </label>
      <label class="field-row">
        <span>Formula refresh rate (ms)</span>
        <input type="number" min="100" step="100" value={getSettings().formulaRefreshMs}
          oninput={(e) => updateSetting("formulaRefreshMs", Number((e.target as HTMLInputElement).value))} />
      </label>
    </div>

    <!-- Provider Intervals -->
    <div class="section">
      <div class="section-header">Provider Intervals (seconds)</div>
      {#each Object.entries(getSettings().providers) as [key, val] (key)}
        <label class="field-row">
          <span>{providerLabels[key] ?? key}</span>
          <input type="number" min="1" step="1" value={val}
            oninput={(e) => updateSetting(`providers.${key}`, Number((e.target as HTMLInputElement).value))} />
        </label>
      {/each}
    </div>

    <!-- Weather -->
    <div class="section">
      <div class="section-header">Weather</div>
      <label class="toggle-row">
        <span>Enabled</span>
        <input type="checkbox" checked={getSettings().weather.enabled}
          onchange={(e) => updateSetting("weather.enabled", (e.target as HTMLInputElement).checked)} />
      </label>
      <label class="field-row">
        <span>Source</span>
        <select value={getSettings().weather.source}
          onchange={(e) => updateSetting("weather.source", (e.target as HTMLSelectElement).value)}>
          <option value="openweathermap">OpenWeatherMap</option>
          <option value="weatherapi">WeatherAPI</option>
          <option value="openmeteo">Open-Meteo</option>
        </select>
      </label>
      <label class="field-row">
        <span>API Key</span>
        <div class="input-with-btn">
          {#if showApiKey}
            <input type="text" value={getSettings().weather.apiKey} placeholder="Enter API key"
              oninput={(e) => updateSetting("weather.apiKey", (e.target as HTMLInputElement).value)} />
          {:else}
            <input type="password" value={getSettings().weather.apiKey} placeholder="Enter API key"
              oninput={(e) => updateSetting("weather.apiKey", (e.target as HTMLInputElement).value)} />
          {/if}
          <button class="show-btn" onclick={() => showApiKey = !showApiKey}>
            {showApiKey ? "Hide" : "Show"}
          </button>
        </div>
      </label>
      <label class="field-row">
        <span>Location</span>
        <input type="text" value={getSettings().weather.location} placeholder="City or lat,lon"
          oninput={(e) => updateSetting("weather.location", (e.target as HTMLInputElement).value)} />
      </label>
      <label class="field-row">
        <span>Units</span>
        <select value={getSettings().weather.units}
          onchange={(e) => updateSetting("weather.units", (e.target as HTMLSelectElement).value)}>
          <option value="metric">Metric (C)</option>
          <option value="imperial">Imperial (F)</option>
        </select>
      </label>
    </div>

    <!-- Cache -->
    <div class="section">
      <div class="section-header">Cache</div>
      <label class="field-row">
        <span>Web Get cache TTL (seconds)</span>
        <input type="number" min="0" step="10" value={getSettings().wgCacheTtl}
          oninput={(e) => updateSetting("wgCacheTtl", Number((e.target as HTMLInputElement).value))} />
      </label>
    </div>

    <!-- Actions -->
    <div class="section">
      <div class="section-header">Actions</div>
      <button class="reset-btn" onclick={resetSettings}>Reset to Defaults</button>
    </div>

  </div>
</div>

<style>
  .settings-panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    font-weight: 600;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }
  .settings-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .section {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .section-header {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-muted);
    margin-bottom: 8px;
  }
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 0;
    font-size: 12px;
    color: var(--text-primary);
    cursor: pointer;
  }
  .toggle-row input[type="checkbox"] {
    accent-color: var(--accent);
    cursor: pointer;
  }
  .field-row {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 4px 0;
    font-size: 12px;
    color: var(--text-primary);
  }
  .field-row span {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .field-row input,
  .field-row select {
    width: 100%;
    font-size: 12px;
    padding: 3px 6px;
  }
  .field-row input[type="number"] {
    width: 80px;
  }
  .input-with-btn {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .input-with-btn input {
    flex: 1;
  }
  .show-btn {
    font-size: 10px;
    padding: 3px 6px;
    border-radius: 3px;
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .show-btn:hover {
    background: var(--accent-dim);
    color: var(--accent);
    border-color: var(--accent);
  }
  .reset-btn {
    width: 100%;
    padding: 6px 12px;
    font-size: 12px;
    border-radius: 4px;
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }
  .reset-btn:hover {
    background: var(--error, #e74c3c);
    color: #fff;
    border-color: var(--error, #e74c3c);
  }
</style>
