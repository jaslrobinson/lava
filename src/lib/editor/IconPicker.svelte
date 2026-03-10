<script lang="ts">
  import { MATERIAL_ICONS, type IconEntry } from "../data/materialIcons";
  import { FONTAWESOME_ICONS, type FAIconEntry } from "../data/fontawesomeIcons";
  import {
    searchIconsDebounced,
    getIconSvg,
    downloadIconToProject,
    type IconifySearchResult,
  } from "../icons/iconifyService";

  let {
    open,
    onSelect,
    onClose,
    assetDir = "",
  }: {
    open: boolean;
    onSelect: (iconSet: string, glyphCode: string, iconSrc?: string) => void;
    onClose: () => void;
    assetDir?: string;
  } = $props();

  let activeTab = $state<"material" | "fontawesome" | "web" | "apk">("material");
  let searchQuery = $state("");
  let displayLimit = $state(100);

  // Web search state
  let webResults = $state<IconifySearchResult[]>([]);
  let webSearching = $state(false);
  let svgPreviews = $state<Map<string, string>>(new Map());

  // APK state
  let apkIconPath = $state<string | null>(null);
  let apkLoading = $state(false);
  let apkError = $state<string | null>(null);

  // Debounce timer for local search
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let debouncedQuery = $state("");

  $effect(() => {
    return () => {
      if (searchTimer) clearTimeout(searchTimer);
    };
  });

  // Debounce local search
  function handleSearchInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    searchQuery = value;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      debouncedQuery = value.toLowerCase().trim();
      displayLimit = 100;
    }, 200);
  }

  // Filter Material Icons by search query
  let filteredMaterial = $derived.by(() => {
    if (!debouncedQuery) return MATERIAL_ICONS;
    return MATERIAL_ICONS.filter(
      (icon) =>
        icon.name.includes(debouncedQuery) ||
        icon.tags.some((t) => t.includes(debouncedQuery))
    );
  });

  // Filter Font Awesome Icons by search query
  let filteredFA = $derived.by(() => {
    if (!debouncedQuery) return FONTAWESOME_ICONS;
    return FONTAWESOME_ICONS.filter(
      (icon) =>
        icon.name.includes(debouncedQuery) ||
        icon.tags.some((t) => t.includes(debouncedQuery))
    );
  });

  // Visible subset (pagination)
  let visibleMaterial = $derived(filteredMaterial.slice(0, displayLimit));
  let visibleFA = $derived(filteredFA.slice(0, displayLimit));
  let visibleWeb = $derived(webResults.slice(0, displayLimit));

  // Whether there are more to show
  let hasMoreMaterial = $derived(filteredMaterial.length > displayLimit);
  let hasMoreFA = $derived(filteredFA.length > displayLimit);
  let hasMoreWeb = $derived(webResults.length > displayLimit);

  // Trigger web search when query changes and tab is "web"
  $effect(() => {
    if (activeTab === "web" && debouncedQuery && debouncedQuery.length >= 2) {
      triggerWebSearch(debouncedQuery);
    }
  });

  async function triggerWebSearch(query: string) {
    webSearching = true;
    svgPreviews = new Map();
    try {
      webResults = await searchIconsDebounced(query, 120);
      // Load SVG previews for first batch
      loadSvgPreviews(webResults.slice(0, displayLimit));
    } catch {
      webResults = [];
    }
    webSearching = false;
  }

  async function loadSvgPreviews(results: IconifySearchResult[]) {
    for (const r of results) {
      const key = `${r.prefix}:${r.name}`;
      if (svgPreviews.has(key)) continue;
      const svg = await getIconSvg(r.prefix, r.name);
      if (svg) {
        // Inject currentColor as white for dark theme visibility
        const colored = svg.svg.replace(/currentColor/g, "var(--text-primary, #e0e0e0)");
        svgPreviews = new Map(svgPreviews).set(key, colored);
      }
    }
  }

  function loadMore() {
    displayLimit += 100;
    // Load more web SVG previews if needed
    if (activeTab === "web" && webResults.length > displayLimit - 100) {
      loadSvgPreviews(webResults.slice(displayLimit - 100, displayLimit));
    }
  }

  function selectMaterial(icon: IconEntry) {
    onSelect("material", icon.code);
  }

  function selectFA(icon: FAIconEntry) {
    onSelect("fontawesome", icon.code);
  }

  async function selectWebIcon(result: IconifySearchResult) {
    if (!assetDir) {
      // No asset dir - use inline SVG approach
      const svg = await getIconSvg(result.prefix, result.name);
      if (svg) {
        const dataUrl = `data:image/svg+xml;base64,${btoa(svg.svg)}`;
        onSelect(result.prefix, "", dataUrl);
      }
      return;
    }
    const path = await downloadIconToProject(result.prefix, result.name, assetDir);
    if (path) {
      onSelect(result.prefix, "", path);
    }
  }

  async function handleApkBrowse() {
    apkError = null;
    apkLoading = true;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const path = await open({
        filters: [{ name: "Android APK", extensions: ["apk"] }],
        multiple: false,
      });
      if (!path) {
        apkLoading = false;
        return;
      }
      const filePath = String(path);
      const targetDir = assetDir || "/tmp";
      const { invoke } = await import("@tauri-apps/api/core");
      const iconPath = await invoke<string>("extract_apk_icon", {
        apkPath: filePath,
        assetDir: targetDir,
      });
      apkIconPath = iconPath;
    } catch (e: any) {
      apkError = e?.message || String(e);
      apkIconPath = null;
    }
    apkLoading = false;
  }

  function selectApkIcon() {
    if (apkIconPath) {
      onSelect("apk", "", apkIconPath);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    }
  }

  function handleOverlayClick(e: MouseEvent) {
    if ((e.target as HTMLElement).dataset.overlay === "true") {
      onClose();
    }
  }

  function switchTab(tab: typeof activeTab) {
    activeTab = tab;
    displayLimit = 100;
  }

  function getApkPreviewSrc(path: string): string {
    try {
      // Use dynamic import approach - convertFileSrc may not be available at module level
      if (typeof (window as any).__TAURI_INTERNALS__ !== "undefined") {
        // We cannot use top-level await here, so use the asset protocol directly
        return `asset://localhost/${encodeURIComponent(path)}`;
      }
    } catch {
      // fallback
    }
    return path;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    data-overlay="true"
    onclick={handleOverlayClick}
    style="position:fixed;inset:0;z-index:1000;background:rgba(0,0,0,0.6);display:flex;align-items:center;justify-content:center;"
  >
    <div style="
      width:620px;max-width:90vw;height:560px;max-height:85vh;
      background:var(--bg-panel);border:1px solid var(--border);border-radius:8px;
      display:flex;flex-direction:column;box-shadow:0 8px 32px rgba(0,0,0,0.5);
    ">
      <!-- Header -->
      <div style="
        display:flex;align-items:center;justify-content:space-between;
        padding:10px 16px;border-bottom:1px solid var(--border);flex-shrink:0;
      ">
        <span style="font-size:14px;font-weight:600;color:var(--text-primary);">Choose Icon</span>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span
          role="button"
          tabindex="0"
          onclick={onClose}
          onkeydown={(e) => { if (e.key === 'Enter') onClose(); }}
          style="
            width:28px;height:28px;display:flex;align-items:center;justify-content:center;
            font-size:18px;color:var(--text-secondary);border-radius:4px;cursor:pointer;
            background:transparent;
          "
          title="Close"
        >&times;</span>
      </div>

      <!-- Search -->
      <div style="padding:8px 16px;flex-shrink:0;">
        <input
          type="text"
          placeholder="Search icons..."
          value={searchQuery}
          oninput={handleSearchInput}
          style="
            width:100%;padding:6px 10px;font-size:13px;
            background:var(--bg-input);color:var(--text-primary);
            border:1px solid var(--border);border-radius:4px;
            outline:none;box-sizing:border-box;
          "
        />
      </div>

      <!-- Tabs -->
      <div style="
        display:flex;gap:0;padding:0 16px;flex-shrink:0;
        border-bottom:1px solid var(--border);
      ">
        {#each [
          { key: "material", label: "Material" },
          { key: "fontawesome", label: "Font Awesome" },
          { key: "web", label: "Web" },
          { key: "apk", label: "APK" },
        ] as tab}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span
            role="button"
            tabindex="0"
            onclick={() => switchTab(tab.key as typeof activeTab)}
            onkeydown={(e) => { if (e.key === 'Enter') switchTab(tab.key as typeof activeTab); }}
            style="
              padding:6px 14px;font-size:12px;font-weight:500;cursor:pointer;
              color:{activeTab === tab.key ? 'var(--accent)' : 'var(--text-secondary)'};
              border-bottom:2px solid {activeTab === tab.key ? 'var(--accent)' : 'transparent'};
              margin-bottom:-1px;transition:color 0.15s;
              background:transparent;user-select:none;
            "
          >{tab.label}</span>
        {/each}
      </div>

      <!-- Content area -->
      <div style="flex:1;overflow-y:auto;padding:12px 16px;">
        {#if activeTab === "material"}
          {#if visibleMaterial.length === 0}
            <div style="font-size:12px;color:var(--text-muted);padding:20px 0;text-align:center;">
              No matching Material icons.
            </div>
          {:else}
            <div style="
              display:grid;grid-template-columns:repeat(auto-fill, minmax(72px, 1fr));
              gap:4px;
            ">
              {#each visibleMaterial as icon}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  role="button"
                  tabindex="0"
                  onclick={() => selectMaterial(icon)}
                  onkeydown={(e) => { if (e.key === 'Enter') selectMaterial(icon); }}
                  style="
                    display:flex;flex-direction:column;align-items:center;gap:2px;
                    padding:8px 4px;border-radius:4px;cursor:pointer;
                    background:var(--bg-input);border:1px solid transparent;
                    transition:border-color 0.15s, background 0.15s;
                  "
                  onmouseenter={(e) => {
                    (e.currentTarget as HTMLElement).style.borderColor = 'var(--accent)';
                    (e.currentTarget as HTMLElement).style.background = 'var(--bg-secondary)';
                  }}
                  onmouseleave={(e) => {
                    (e.currentTarget as HTMLElement).style.borderColor = 'transparent';
                    (e.currentTarget as HTMLElement).style.background = 'var(--bg-input)';
                  }}
                  title={icon.name}
                >
                  <span style="font-family:'Material Icons';font-size:28px;color:var(--text-primary);line-height:1;">
                    {String.fromCodePoint(parseInt(icon.code, 16))}
                  </span>
                  <span style="font-size:9px;color:var(--text-muted);text-align:center;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:100%;">
                    {icon.name}
                  </span>
                </span>
              {/each}
            </div>
            {#if hasMoreMaterial}
              <div style="text-align:center;padding:12px 0;">
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  role="button"
                  tabindex="0"
                  onclick={loadMore}
                  onkeydown={(e) => { if (e.key === 'Enter') loadMore(); }}
                  style="
                    display:inline-block;padding:6px 16px;font-size:11px;font-weight:500;
                    color:var(--accent);background:var(--accent-dim);border-radius:4px;
                    cursor:pointer;user-select:none;
                  "
                >Load More ({filteredMaterial.length - displayLimit} remaining)</span>
              </div>
            {/if}
          {/if}

        {:else if activeTab === "fontawesome"}
          {#if visibleFA.length === 0}
            <div style="font-size:12px;color:var(--text-muted);padding:20px 0;text-align:center;">
              No matching Font Awesome icons.
            </div>
          {:else}
            <div style="
              display:grid;grid-template-columns:repeat(auto-fill, minmax(72px, 1fr));
              gap:4px;
            ">
              {#each visibleFA as icon}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  role="button"
                  tabindex="0"
                  onclick={() => selectFA(icon)}
                  onkeydown={(e) => { if (e.key === 'Enter') selectFA(icon); }}
                  style="
                    display:flex;flex-direction:column;align-items:center;gap:2px;
                    padding:8px 4px;border-radius:4px;cursor:pointer;
                    background:var(--bg-input);border:1px solid transparent;
                    transition:border-color 0.15s, background 0.15s;
                  "
                  onmouseenter={(e) => {
                    (e.currentTarget as HTMLElement).style.borderColor = 'var(--accent)';
                    (e.currentTarget as HTMLElement).style.background = 'var(--bg-secondary)';
                  }}
                  onmouseleave={(e) => {
                    (e.currentTarget as HTMLElement).style.borderColor = 'transparent';
                    (e.currentTarget as HTMLElement).style.background = 'var(--bg-input)';
                  }}
                  title={icon.name}
                >
                  <span style="font-family:'Font Awesome 6 Free';font-weight:900;font-size:24px;color:var(--text-primary);line-height:1;">
                    {String.fromCodePoint(parseInt(icon.code, 16))}
                  </span>
                  <span style="font-size:9px;color:var(--text-muted);text-align:center;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:100%;">
                    {icon.name}
                  </span>
                </span>
              {/each}
            </div>
            {#if hasMoreFA}
              <div style="text-align:center;padding:12px 0;">
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  role="button"
                  tabindex="0"
                  onclick={loadMore}
                  onkeydown={(e) => { if (e.key === 'Enter') loadMore(); }}
                  style="
                    display:inline-block;padding:6px 16px;font-size:11px;font-weight:500;
                    color:var(--accent);background:var(--accent-dim);border-radius:4px;
                    cursor:pointer;user-select:none;
                  "
                >Load More ({filteredFA.length - displayLimit} remaining)</span>
              </div>
            {/if}
          {/if}

        {:else if activeTab === "web"}
          {#if webSearching}
            <div style="font-size:12px;color:var(--text-muted);padding:20px 0;text-align:center;">
              Searching Iconify...
            </div>
          {:else if !debouncedQuery || debouncedQuery.length < 2}
            <div style="font-size:12px;color:var(--text-muted);padding:20px 0;text-align:center;">
              Type at least 2 characters to search the Iconify library.
            </div>
          {:else if visibleWeb.length === 0}
            <div style="font-size:12px;color:var(--text-muted);padding:20px 0;text-align:center;">
              No web icons found for "{debouncedQuery}".
            </div>
          {:else}
            <div style="
              display:grid;grid-template-columns:repeat(auto-fill, minmax(72px, 1fr));
              gap:4px;
            ">
              {#each visibleWeb as result}
                {@const svgKey = `${result.prefix}:${result.name}`}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  role="button"
                  tabindex="0"
                  onclick={() => selectWebIcon(result)}
                  onkeydown={(e) => { if (e.key === 'Enter') selectWebIcon(result); }}
                  style="
                    display:flex;flex-direction:column;align-items:center;gap:2px;
                    padding:8px 4px;border-radius:4px;cursor:pointer;
                    background:var(--bg-input);border:1px solid transparent;
                    transition:border-color 0.15s, background 0.15s;
                  "
                  onmouseenter={(e) => {
                    (e.currentTarget as HTMLElement).style.borderColor = 'var(--accent)';
                    (e.currentTarget as HTMLElement).style.background = 'var(--bg-secondary)';
                  }}
                  onmouseleave={(e) => {
                    (e.currentTarget as HTMLElement).style.borderColor = 'transparent';
                    (e.currentTarget as HTMLElement).style.background = 'var(--bg-input)';
                  }}
                  title="{result.prefix}:{result.name}"
                >
                  <span style="width:28px;height:28px;display:flex;align-items:center;justify-content:center;">
                    {#if svgPreviews.has(svgKey)}
                      {@html svgPreviews.get(svgKey)}
                    {:else}
                      <span style="font-size:10px;color:var(--text-muted);">...</span>
                    {/if}
                  </span>
                  <span style="font-size:9px;color:var(--text-muted);text-align:center;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:100%;">
                    {result.name}
                  </span>
                </span>
              {/each}
            </div>
            {#if hasMoreWeb}
              <div style="text-align:center;padding:12px 0;">
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  role="button"
                  tabindex="0"
                  onclick={loadMore}
                  onkeydown={(e) => { if (e.key === 'Enter') loadMore(); }}
                  style="
                    display:inline-block;padding:6px 16px;font-size:11px;font-weight:500;
                    color:var(--accent);background:var(--accent-dim);border-radius:4px;
                    cursor:pointer;user-select:none;
                  "
                >Load More ({webResults.length - displayLimit} remaining)</span>
              </div>
            {/if}
          {/if}

        {:else if activeTab === "apk"}
          <div style="display:flex;flex-direction:column;align-items:center;gap:16px;padding:20px 0;">
            <div style="font-size:12px;color:var(--text-secondary);text-align:center;max-width:360px;">
              Browse an Android APK file to extract its launcher icon.
            </div>
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <span
              role="button"
              tabindex="0"
              onclick={handleApkBrowse}
              onkeydown={(e) => { if (e.key === 'Enter') handleApkBrowse(); }}
              style="
                display:inline-block;padding:8px 20px;font-size:12px;font-weight:500;
                color:var(--accent);background:var(--accent-dim);border-radius:4px;
                cursor:pointer;user-select:none;
              "
            >{apkLoading ? "Extracting..." : "Browse APK..."}</span>

            {#if apkError}
              <div style="font-size:11px;color:#e74c3c;text-align:center;max-width:360px;">
                {apkError}
              </div>
            {/if}

            {#if apkIconPath}
              <div style="display:flex;flex-direction:column;align-items:center;gap:8px;">
                <div style="
                  width:96px;height:96px;border-radius:8px;overflow:hidden;
                  background:var(--bg-secondary);border:1px solid var(--border);
                  display:flex;align-items:center;justify-content:center;
                ">
                  <img
                    src={getApkPreviewSrc(apkIconPath)}
                    alt="APK Icon"
                    style="width:100%;height:100%;object-fit:contain;"
                  />
                </div>
                <span style="font-size:10px;color:var(--text-muted);max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">
                  {apkIconPath.split("/").pop()}
                </span>
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  role="button"
                  tabindex="0"
                  onclick={selectApkIcon}
                  onkeydown={(e) => { if (e.key === 'Enter') selectApkIcon(); }}
                  style="
                    display:inline-block;padding:6px 20px;font-size:12px;font-weight:600;
                    color:#fff;background:var(--accent);border-radius:4px;
                    cursor:pointer;user-select:none;
                  "
                >Use This Icon</span>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Footer with count -->
      <div style="
        padding:6px 16px;border-top:1px solid var(--border);flex-shrink:0;
        font-size:10px;color:var(--text-muted);
      ">
        {#if activeTab === "material"}
          {filteredMaterial.length} icons
          {#if debouncedQuery}(filtered from {MATERIAL_ICONS.length}){/if}
        {:else if activeTab === "fontawesome"}
          {filteredFA.length} icons
          {#if debouncedQuery}(filtered from {FONTAWESOME_ICONS.length}){/if}
        {:else if activeTab === "web"}
          {webResults.length} results from Iconify
        {:else if activeTab === "apk"}
          Extract icon from Android APK
        {/if}
      </div>
    </div>
  </div>
{/if}
