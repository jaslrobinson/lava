<!-- StartMenuOverlay.svelte — Win11 Start Menu rendered as HTML overlay above the canvas -->
<script lang="ts">
  import { onMount } from "svelte";
  import { markDirty } from "./renderScheduler";
  import { getSettings } from "../stores/settings.svelte";

  interface Props {
    anchorBounds?: { x: number; y: number; w: number; h: number } | null;
    baseScale: number;
    zoom: number;
    panX: number;
    panY: number;
    canvasOffsetX: number;
    canvasOffsetY: number;
    containerW?: number;
    containerH?: number;
    interactive?: boolean;  // false in editor mode
    visible: boolean;
    smBg?: string;
    smAccent?: string;
    onclose: () => void;
  }
  let { anchorBounds = null, baseScale, zoom, panX, panY, canvasOffsetX, canvasOffsetY, containerW = 1920, containerH = 1080, interactive = true, visible, smBg = "#1c1c1c", smAccent = "#60cdff", onclose }: Props = $props();

  const SM_W = 660;
  const SM_H = 720;

  let apps: Array<{ name: string; exec: string; icon: string; categories: string }> = $state([]);
  let searchQuery = $state("");
  let viewAllApps = $state(false);
  let searchInputEl: HTMLInputElement | undefined = $state(undefined);

  // Position the panel above the trigger element (anchorBounds) or default to bottom-center.
  let panelStyle = $derived.by(() => {
    let panelProjX: number;
    let panelProjY: number;

    // Always position from bottom of canvas upward (above taskbar)
    // X: left-align with trigger, clamped to canvas; default: centered
    const idealX = anchorBounds
      ? anchorBounds.x + anchorBounds.w / 2 - SM_W / 2
      : containerW / 2 - SM_W / 2;
    panelProjX = Math.max(0, Math.min(containerW - SM_W, idealX));
    // Offset: editor needs panel higher (above taskbar), wallpaper needs it lower
    const offsetPx = interactive ? 57 : -38;
    panelProjY = containerH - SM_H + offsetPx;

    const left = canvasOffsetX + (panelProjX * zoom + panX) * baseScale;
    const top = canvasOffsetY + (panelProjY * zoom + panY) * baseScale;
    const width = SM_W * zoom * baseScale;
    const height = SM_H * zoom * baseScale;

    return `left:${left}px;top:${top}px;width:${width}px;height:${height}px;font-size:${zoom * baseScale * 13}px;`;
  });

  // Notify wallpaper process about keyboard interactivity needs
  $effect(() => {
    if ((window as any).webkit?.messageHandlers?.lava) {
      (window as any).webkit.messageHandlers.lava.postMessage(
        JSON.stringify({ type: visible ? "start_menu_open" : "start_menu_close" })
      );
    }
  });

  // Notify Rust of start menu visibility so it can manage keyboard focus
  $effect(() => {
    const wk = (window as any).webkit?.messageHandlers?.lava;
    if (wk) wk.postMessage(JSON.stringify({ type: visible ? "start_menu_open" : "start_menu_close" }));
  });

  // Focus search input when opened
  $effect(() => {
    if (!visible) {
      searchQuery = "";
      viewAllApps = false;
      return;
    }

    // Try programmatic focus (works in browser, may silently fail in WebKitGTK)
    const t1 = setTimeout(() => searchInputEl?.focus(), 100);
    const t2 = setTimeout(() => searchInputEl?.focus(), 400);

    // Fallback for WebKitGTK: capture keydown at document level and manually
    // update searchQuery. Works as long as the WebKit widget has GTK focus
    // (ensured by grab_focus() + KeyboardMode::Exclusive on the Rust side).
    const onKey = (e: KeyboardEvent) => {
      // If the input already has DOM focus, let it handle events normally
      if (document.activeElement === searchInputEl) return;
      if (e.key === 'Escape') { visible = false; return; }
      if (e.key === 'Backspace') { e.preventDefault(); searchQuery = searchQuery.slice(0, -1); return; }
      if (e.key === 'Delete') { e.preventDefault(); searchQuery = ""; return; }
      if (e.key.length === 1 && !e.ctrlKey && !e.altKey && !e.metaKey) {
        e.preventDefault();
        searchQuery += e.key;
      }
    };
    document.addEventListener('keydown', onKey);

    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
      document.removeEventListener('keydown', onKey);
    };
  });

  // Expose window.__lavaKey so the Rust GTK key-press handler can inject
  // keystrokes via run_javascript() — bypasses WebKit's broken focus in layer-shell.
  onMount(() => {
    (window as any).__lavaKey = (key: string) => {
      if (key === 'Escape')    { visible = false; return; }
      if (key === 'Backspace') { searchQuery = searchQuery.slice(0, -1); return; }
      if (key === 'Delete')    { searchQuery = ""; return; }
      if (key === 'Enter')     { handleSearchEnter(); return; }
      if (key.length === 1)    { searchQuery += key; }
    };
    return () => { delete (window as any).__lavaKey; };
  });

  onMount(async () => {
    const isTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";
    if (isTauri) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        apps = await invoke<typeof apps>("list_apps");
        return;
      } catch (e) {
        console.warn("list_apps invoke failed:", e);
      }
    }
    // Wallpaper mode: use pre-injected apps list, retrying until available
    const checkInjected = () => {
      const injected = (window as any).__LAVA_APPS;
      if (Array.isArray(injected) && injected.length > 0) {
        apps = injected;
      } else {
        setTimeout(checkInjected, 150);
      }
    };
    checkInjected();
  });

  const searchUrls: Record<string, string> = {
    google: "https://www.google.com/search?q=",
    perplexity: "https://www.perplexity.ai/search?q=",
    bing: "https://www.bing.com/search?q=",
    chatgpt: "https://chatgpt.com/?q=",
    duckduckgo: "https://duckduckgo.com/?q=",
  };

  async function webSearch(query: string) {
    const engine = getSettings().searchEngine || "google";
    const url = (searchUrls[engine] || searchUrls.google) + encodeURIComponent(query);
    onclose();
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("open_url", { url });
    } catch {
      (window as any).webkit?.messageHandlers?.lava?.postMessage(
        JSON.stringify({ type: "open_url", url })
      );
    }
    markDirty();
  }

  async function launchApp(exec: string) {
    onclose();
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("launch_app", { command: exec });
    } catch {
      (window as any).webkit?.messageHandlers?.lava?.postMessage(
        JSON.stringify({ type: "launch_app", command: exec })
      );
    }
    markDirty();
  }

  function handleSearchEnter() {
    const q = searchQuery.trim();
    if (!q) return;
    if (filteredApps.length > 0) {
      launchApp(filteredApps[0].exec);
    } else {
      webSearch(q);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") { onclose(); e.preventDefault(); }
  }

  function handleOutsideClick(e: MouseEvent) {
    onclose();
    e.stopPropagation();
  }

  let filteredApps = $derived(
    searchQuery.trim().length > 0
      ? apps.filter(a =>
          a.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          a.exec.toLowerCase().includes(searchQuery.toLowerCase())
        ).slice(0, 10)
      : []
  );

  let pinnedApps = $derived(apps.slice(0, 18));
  let recommendedApps = $derived(apps.slice(18, 26));

  // Group apps alphabetically for all-apps view
  let groupedApps = $derived.by(() => {
    const groups: Map<string, typeof apps> = new Map();
    for (const app of apps) {
      const letter = app.name.charAt(0).toUpperCase();
      if (!groups.has(letter)) groups.set(letter, []);
      groups.get(letter)!.push(app);
    }
    return Array.from(groups.entries()).sort(([a], [b]) => a.localeCompare(b));
  });

  const _isTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";
  let _convertFileSrc: ((path: string) => string) | null = null;
  if (_isTauri) {
    import("@tauri-apps/api/core").then(m => { _convertFileSrc = m.convertFileSrc; });
  }

  function getIconSrc(iconPath: string): string {
    if (!iconPath) return "";
    if (!iconPath.startsWith("/")) return iconPath;
    if (_isTauri) {
      return _convertFileSrc ? _convertFileSrc(iconPath) : `asset://localhost${iconPath}`;
    }
    // Wallpaper mode: served via local HTTP asset proxy
    return `/__lava_assets${iconPath}`;
  }

  function iconFallbackLetter(name: string): string {
    return name.charAt(0).toUpperCase();
  }

  function stringToColor(str: string): string {
    let hash = 0;
    for (let i = 0; i < str.length; i++) hash = str.charCodeAt(i) + ((hash << 5) - hash);
    const hue = Math.abs(hash) % 360;
    return `hsl(${hue}, 45%, 38%)`;
  }

  function getCategoryLabel(categories: string): string {
    if (!categories) return "Application";
    const first = categories.split(";").filter(Boolean)[0] ?? "Application";
    return first;
  }

  function onImgError(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    img.style.display = "none";
    const sibling = img.nextElementSibling as HTMLElement | null;
    if (sibling) sibling.style.display = "flex";
  }
</script>

{#if visible}
<!-- Backdrop: captures outside clicks (editor mode: pointer-events:none, canvas onMouseDown handles close) -->
<div
  class="sm-backdrop"
  style={interactive ? "" : "pointer-events:none;"}
  onclick={handleOutsideClick}
  onkeydown={handleKeydown}
  role="presentation"
></div>

<!-- Panel -->
<div
  class="sm-panel"
  style={panelStyle + (interactive ? "" : "pointer-events:none;") + `--sm-bg:${smBg};--sm-accent:${smAccent};`}
  onclick={(e) => e.stopPropagation()}
  onkeydown={handleKeydown}
  role="dialog"
  aria-modal="true"
>
  <!-- Search bar -->
  <div class="sm-search-wrap">
    <div class="sm-search-icon">🔍</div>
    <input
      bind:this={searchInputEl}
      bind:value={searchQuery}
      class="sm-search-input"
      type="text"
      placeholder="Search apps or the web"
      autocomplete="off"
      spellcheck="false"
      onkeydown={(e) => { if (e.key === "Enter") { e.preventDefault(); handleSearchEnter(); } }}
    />
    {#if searchQuery}
      <button class="sm-search-clear" onclick={() => searchQuery = ""}>✕</button>
    {/if}
  </div>

  <!-- Content area -->
  <div class="sm-content">
    {#if searchQuery.trim()}
      <!-- Search results -->
      <div class="sm-section-label">Best match</div>
      {#if filteredApps.length === 0}
        <div class="sm-empty">No results found</div>
      {:else}
        {#each filteredApps as app, i (app.exec + i)}
          <button
            class="sm-search-result"
            class:sm-top={i === 0}
            onclick={() => launchApp(app.exec)}
          >
            <span class="sm-app-icon-wrap sm-app-icon-sm">
              <img src={getIconSrc(app.icon)} alt="" onerror={onImgError} />
              <span class="sm-app-letter" style="background:{stringToColor(app.name)};display:none">{iconFallbackLetter(app.name)}</span>
            </span>
            <span class="sm-search-result-text">
              <span class="sm-app-name">{app.name}</span>
              <span class="sm-app-category">App</span>
            </span>
          </button>
        {/each}
      {/if}

    {:else if viewAllApps}
      <!-- All apps alphabetical -->
      <div class="sm-all-apps-header">
        <button class="sm-back-btn" onclick={() => viewAllApps = false}>‹ Back</button>
        <span class="sm-section-label-inline">All apps</span>
      </div>
      <div class="sm-all-apps-list">
        {#each groupedApps as [letter, letterApps] (letter)}
          <div class="sm-letter-header">{letter}</div>
          {#each letterApps as app (app.exec)}
            <button class="sm-all-app-row" onclick={() => launchApp(app.exec)}>
              <span class="sm-app-icon-wrap sm-app-icon-xs">
                <img src={getIconSrc(app.icon)} alt="" onerror={onImgError} />
                <span class="sm-app-letter sm-app-letter-xs" style="background:{stringToColor(app.name)};display:none">{iconFallbackLetter(app.name)}</span>
              </span>
              <span class="sm-app-name">{app.name}</span>
            </button>
          {/each}
        {/each}
      </div>

    {:else}
      <!-- Default: pinned + recommended -->
      <div class="sm-row-header">
        <span class="sm-section-label-inline">Pinned</span>
        <button class="sm-all-apps-btn" onclick={() => viewAllApps = true}>All apps  ›</button>
      </div>
      <div class="sm-pinned-grid">
        {#each pinnedApps as app (app.exec)}
          <button class="sm-pinned-item" onclick={() => launchApp(app.exec)}>
            <span class="sm-app-icon-wrap sm-app-icon-lg">
              <img src={getIconSrc(app.icon)} alt="" onerror={onImgError} />
              <span class="sm-app-letter sm-app-letter-lg" style="background:{stringToColor(app.name)};display:none">{iconFallbackLetter(app.name)}</span>
            </span>
            <span class="sm-pinned-label">{app.name.length > 13 ? app.name.slice(0, 12) + '…' : app.name}</span>
          </button>
        {/each}
      </div>

      <div class="sm-separator"></div>

      <div class="sm-section-label-inline sm-rec-label">Recommended</div>
      <div class="sm-recommended-grid">
        {#each recommendedApps as app (app.exec)}
          <button class="sm-rec-item" onclick={() => launchApp(app.exec)}>
            <span class="sm-app-icon-wrap sm-app-icon-sm">
              <img src={getIconSrc(app.icon)} alt="" onerror={onImgError} />
              <span class="sm-app-letter" style="background:{stringToColor(app.name)};display:none">{iconFallbackLetter(app.name)}</span>
            </span>
            <span class="sm-rec-text">
              <span class="sm-app-name">{app.name.length > 22 ? app.name.slice(0,21) + '…' : app.name}</span>
              <span class="sm-app-category">{getCategoryLabel(app.categories)}</span>
            </span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Bottom bar -->
  <div class="sm-bottom-bar">
    <button class="sm-user-btn">
      <span class="sm-avatar">👤</span>
      <span>Account</span>
    </button>
    <button class="sm-power-btn" title="Power" onclick={() => launchApp("systemctl poweroff")}>⏻</button>
  </div>
</div>
{/if}

<style>
  .sm-backdrop {
    position: absolute;
    inset: 0;
    z-index: 9;
    background: transparent;
    pointer-events: auto;
  }

  .sm-panel {
    position: absolute;
    z-index: 10;
    background: color-mix(in srgb, var(--sm-bg, #1c1c1c) 97%, transparent);
    border: 1px solid rgba(255,255,255,0.10);
    border-radius: 12px;
    box-shadow: 0 16px 48px rgba(0,0,0,0.6);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    pointer-events: auto;
    font-family: "Segoe UI", system-ui, sans-serif;
    color: rgba(255,255,255,0.88);
    box-sizing: border-box;
  }

  .sm-search-wrap {
    display: flex;
    align-items: center;
    margin: 20px 24px 12px;
    background: rgba(255,255,255,0.09);
    border: 1px solid color-mix(in srgb, var(--sm-accent, #60cdff) 45%, transparent);
    border-radius: 22px;
    padding: 0 14px;
    height: 40px;
    flex-shrink: 0;
  }

  .sm-search-icon { opacity: 0.55; font-size: 0.95em; margin-right: 8px; }

  .sm-search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: rgba(255,255,255,0.92);
    font-size: 1em;
    font-family: inherit;
  }
  .sm-search-input::placeholder { color: rgba(255,255,255,0.38); }

  .sm-search-clear {
    background: none; border: none; color: rgba(255,255,255,0.45);
    cursor: pointer; padding: 0; font-size: 0.85em;
  }
  .sm-search-clear:hover { color: rgba(255,255,255,0.8); }

  .sm-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    padding: 0 24px;
    min-height: 0;
  }

  .sm-section-label {
    font-size: 0.85em;
    color: rgba(255,255,255,0.45);
    margin-bottom: 6px;
    margin-top: 4px;
    padding-left: 2px;
  }

  .sm-empty {
    text-align: center;
    color: rgba(255,255,255,0.38);
    padding: 32px 0;
    font-size: 0.95em;
  }

  /* Search results */
  .sm-search-result {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    background: none;
    border: none;
    border-radius: 6px;
    padding: 8px 10px;
    cursor: pointer;
    color: rgba(255,255,255,0.88);
    font-family: inherit;
    font-size: 1em;
    text-align: left;
    margin-bottom: 2px;
  }
  .sm-search-result:hover, .sm-search-result.sm-top { background: rgba(255,255,255,0.11); }

  .sm-search-result-text {
    display: flex;
    flex-direction: column;
  }

  /* App icon wrappers */
  .sm-app-icon-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .sm-app-icon-wrap img {
    width: 100%; height: 100%;
    object-fit: contain;
    border-radius: 6px;
  }
  .sm-app-letter {
    width: 100%; height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    color: #fff;
    font-weight: bold;
    font-size: 0.85em;
  }
  .sm-app-letter-lg { font-size: 1.1em; }
  .sm-app-letter-xs { font-size: 0.7em; }

  .sm-app-icon-lg { width: 44px; height: 44px; }
  .sm-app-icon-sm { width: 28px; height: 28px; }
  .sm-app-icon-xs { width: 24px; height: 24px; }

  .sm-app-name { font-size: 0.9em; color: rgba(255,255,255,0.88); }
  .sm-app-category { font-size: 0.8em; color: rgba(255,255,255,0.42); }

  /* All apps header */
  .sm-all-apps-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
    margin-top: 2px;
    flex-shrink: 0;
  }
  .sm-back-btn {
    background: rgba(255,255,255,0.07);
    border: none;
    border-radius: 4px;
    color: rgba(255,255,255,0.7);
    cursor: pointer;
    padding: 4px 12px;
    font-size: 0.9em;
    font-family: inherit;
  }
  .sm-back-btn:hover { background: rgba(255,255,255,0.12); }

  .sm-section-label-inline {
    font-weight: 600;
    font-size: 0.95em;
    color: rgba(255,255,255,0.88);
  }

  .sm-all-apps-list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: rgba(255,255,255,0.2) transparent;
  }
  .sm-all-apps-list::-webkit-scrollbar { width: 5px; }
  .sm-all-apps-list::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.2); border-radius: 3px; }

  .sm-letter-header {
    font-size: 0.8em;
    font-weight: 600;
    color: rgba(255,255,255,0.32);
    padding: 4px 6px 2px;
    border-bottom: 1px solid rgba(255,255,255,0.06);
    margin-bottom: 2px;
    margin-top: 4px;
  }

  .sm-all-app-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    background: none;
    border: none;
    border-radius: 4px;
    padding: 6px 6px;
    cursor: pointer;
    color: rgba(255,255,255,0.85);
    font-family: inherit;
    font-size: 1em;
    text-align: left;
  }
  .sm-all-app-row:hover { background: rgba(255,255,255,0.09); }

  /* Pinned grid */
  .sm-row-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
    margin-top: 4px;
    flex-shrink: 0;
  }
  .sm-all-apps-btn {
    background: rgba(255,255,255,0.07);
    border: none;
    border-radius: 13px;
    color: rgba(255,255,255,0.78);
    cursor: pointer;
    padding: 3px 14px;
    font-size: 0.85em;
    font-family: inherit;
  }
  .sm-all-apps-btn:hover { background: rgba(255,255,255,0.13); }

  .sm-pinned-grid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 4px;
    flex-shrink: 0;
  }

  .sm-pinned-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    background: none;
    border: none;
    border-radius: 6px;
    padding: 8px 4px 6px;
    cursor: pointer;
    color: rgba(255,255,255,0.78);
    font-family: inherit;
    font-size: 0.85em;
    text-align: center;
  }
  .sm-pinned-item:hover { background: rgba(255,255,255,0.09); }

  .sm-pinned-label {
    font-size: 0.85em;
    line-height: 1.2;
    word-break: break-word;
  }

  .sm-separator {
    height: 1px;
    background: rgba(255,255,255,0.06);
    margin: 10px 0 10px;
    flex-shrink: 0;
  }

  .sm-rec-label { margin-bottom: 8px; flex-shrink: 0; }

  .sm-recommended-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2px;
    flex-shrink: 0;
  }

  .sm-rec-item {
    display: flex;
    align-items: center;
    gap: 10px;
    background: none;
    border: none;
    border-radius: 4px;
    padding: 7px 8px;
    cursor: pointer;
    color: rgba(255,255,255,0.85);
    font-family: inherit;
    font-size: 1em;
    text-align: left;
  }
  .sm-rec-item:hover { background: rgba(255,255,255,0.09); }

  .sm-rec-text { display: flex; flex-direction: column; }

  /* Bottom bar */
  .sm-bottom-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 24px;
    border-top: 1px solid rgba(255,255,255,0.06);
    flex-shrink: 0;
    height: 56px;
    box-sizing: border-box;
  }

  .sm-user-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    background: none;
    border: none;
    border-radius: 5px;
    color: rgba(255,255,255,0.85);
    cursor: pointer;
    padding: 6px 10px;
    font-family: inherit;
    font-size: 1em;
  }
  .sm-user-btn:hover { background: rgba(255,255,255,0.08); }

  .sm-avatar { font-size: 1.3em; }

  .sm-power-btn {
    background: none;
    border: none;
    border-radius: 5px;
    color: rgba(255,255,255,0.7);
    cursor: pointer;
    padding: 8px 10px;
    font-size: 1.2em;
  }
  .sm-power-btn:hover { background: rgba(255,80,80,0.2); color: #ff8080; }
</style>
