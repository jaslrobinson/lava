export interface IconifySearchResult {
  prefix: string;  // icon set prefix, e.g. "mdi", "fa6-solid"
  name: string;    // icon name within the set
  title: string;   // human readable name
}

export interface IconifySvgResult {
  svg: string;     // full SVG markup
  width: number;
  height: number;
}

const API_BASE = "https://api.iconify.design";

let searchTimeout: ReturnType<typeof setTimeout> | null = null;

/**
 * Search icons across all Iconify icon sets
 * Uses the Iconify API: https://iconify.design/docs/api/search.html
 */
export async function searchIcons(query: string, limit = 60): Promise<IconifySearchResult[]> {
  if (!query || query.length < 2) return [];

  try {
    const response = await fetch(`${API_BASE}/search?query=${encodeURIComponent(query)}&limit=${limit}`);
    if (!response.ok) throw new Error(`API error: ${response.status}`);

    const data = await response.json();
    // API returns { icons: ["prefix:name", ...], total: number }
    const results: IconifySearchResult[] = [];

    if (data.icons && Array.isArray(data.icons)) {
      for (const icon of data.icons) {
        const [prefix, name] = icon.split(":");
        if (prefix && name) {
          results.push({
            prefix,
            name,
            title: name.replace(/-/g, " "),
          });
        }
      }
    }

    return results;
  } catch (e) {
    console.warn("Iconify search failed:", e);
    return [];
  }
}

/**
 * Debounced search - cancels previous pending search
 */
export function searchIconsDebounced(query: string, limit = 60): Promise<IconifySearchResult[]> {
  return new Promise((resolve) => {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(async () => {
      const results = await searchIcons(query, limit);
      resolve(results);
    }, 300);
  });
}

/**
 * Get SVG data for a specific icon
 */
export async function getIconSvg(prefix: string, name: string): Promise<IconifySvgResult | null> {
  try {
    const response = await fetch(`${API_BASE}/${prefix}/${name}.svg`);
    if (!response.ok) return null;

    const svg = await response.text();

    // Extract width/height from SVG viewBox or attributes
    const viewBoxMatch = svg.match(/viewBox="(\d+)\s+(\d+)\s+(\d+)\s+(\d+)"/);
    const width = viewBoxMatch ? parseInt(viewBoxMatch[3]) : 24;
    const height = viewBoxMatch ? parseInt(viewBoxMatch[4]) : 24;

    return { svg, width, height };
  } catch (e) {
    console.warn("Failed to get icon SVG:", e);
    return null;
  }
}

/**
 * Download an icon SVG and save it to the project's asset directory
 * Returns the saved file path
 */
export async function downloadIconToProject(prefix: string, name: string, assetDir: string): Promise<string | null> {
  const result = await getIconSvg(prefix, name);
  if (!result) return null;

  try {
    const { invoke } = await import("@tauri-apps/api/core");

    // Write SVG to a temp file, then copy to project
    const filename = `${prefix}--${name}.svg`;
    const iconsDir = `${assetDir}/icons`;

    // Use invoke to write the file from Rust side
    await invoke("write_icon_file", {
      assetDir,
      filename,
      content: result.svg,
    });

    return `${iconsDir}/${filename}`;
  } catch (e) {
    console.warn("Failed to save icon:", e);
    return null;
  }
}
