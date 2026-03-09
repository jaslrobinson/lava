const formulaPattern = /\$[^$]+\$/g;

// Detect if we're running inside Tauri (vs plain WebKitGTK wallpaper view)
const isTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) throw new Error("Not in Tauri context");
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// Cache of formula string -> evaluated result
const cache = new Map<string, string>();
// Formulas queued for re-evaluation (keeps stale value in cache while pending)
const pending = new Set<string>();
// Whether an evaluation batch is in flight
let evaluating = false;

// --- Web Get (wg) cache ---
const wgRawCache = new Map<string, string>(); // URL -> raw response text
const wgResultCache = new Map<string, { val: string; ts: number }>(); // cacheKey -> extracted value
const wgFetching = new Set<string>(); // URLs currently being fetched
const WG_CACHE_TTL = 300_000; // 5 minutes

// --- Provider data for non-Tauri (wallpaper) view ---
// Structure: { "mi": { "title": "Song", "artist": "Artist" }, "bi": { "level": "85" }, ... }
let providerData: Record<string, Record<string, string>> = {};
let providerFetching = false;

async function fetchProviderData() {
  if (isTauri || providerFetching) return;
  providerFetching = true;
  try {
    const resp = await fetch("/__klwp_providers");
    if (resp.ok) {
      const data = await resp.json();
      // Only update if we got actual provider data (don't overwrite valid data with {})
      if (data && Object.keys(data).length > 0) {
        providerData = data;
      }
    }
  } catch {
    // Endpoint not available or partial JSON — keep existing data
  }
  providerFetching = false;
}

/** Check if a string contains any $formula$ expressions */
export function hasFormula(text: string): boolean {
  return typeof text === "string" && text.includes("$");
}

/**
 * Resolve a string that may contain $formula$ expressions.
 * Returns the string with formulas replaced by their cached values.
 * First-time formulas show "…" until evaluated; refreshes use stale value until updated.
 */
export function resolveFormula(text: string): string {
  if (!hasFormula(text)) return text;

  return text.replace(formulaPattern, (match) => {
    if (cache.has(match)) {
      return cache.get(match)!;
    }
    // First time seeing this formula — queue it
    pending.add(match);
    return "\u2026";
  });
}

/** Split wg() arguments respecting quoted strings */
function splitWgArgs(s: string): string[] {
  const args: string[] = [];
  let current = "";
  let inQuote = false;
  let qChar = "";
  let depth = 0;
  for (const ch of s) {
    if (inQuote) {
      if (ch === qChar) inQuote = false;
      else current += ch;
    } else if (ch === '"' || ch === "'") {
      inQuote = true;
      qChar = ch;
    } else if (ch === "(") {
      depth++;
      current += ch;
    } else if (ch === ")") {
      depth--;
      current += ch;
    } else if (ch === "," && depth === 0) {
      args.push(current.trim());
      current = "";
    } else {
      current += ch;
    }
  }
  args.push(current.trim());
  return args;
}

/** Extract data from a fetched response based on type/path/index */
function extractWgData(raw: string, type: string, path: string, index: number): string {
  if (type === "rss" || type === "xml") {
    try {
      const parser = new DOMParser();
      const doc = parser.parseFromString(raw, "text/xml");
      const items = doc.querySelectorAll("item");
      const entries = items.length > 0 ? items : doc.querySelectorAll("entry"); // Atom feeds

      const p = path.toLowerCase();
      if (p === "feed" || p === "ttitle" || p === "title" || p === "") {
        return entries[index]?.querySelector("title")?.textContent?.trim() || "";
      }
      if (p === "tdesc" || p === "desc" || p === "description" || p === "summary") {
        const el = entries[index]?.querySelector("description, summary, content");
        const text = el?.textContent?.trim() || "";
        // Strip HTML tags from description
        return text.replace(/<[^>]*>/g, "").trim();
      }
      if (p === "tlink" || p === "link") {
        const item = entries[index];
        if (!item) return "";
        const linkEl = item.querySelector("link");
        return linkEl?.getAttribute("href") || linkEl?.textContent?.trim() || "";
      }
      if (p === "tdate" || p === "date" || p === "pubdate") {
        const item = entries[index];
        if (!item) return "";
        const dateEl = item.querySelector("pubDate, published, updated");
        return dateEl?.textContent?.trim() || "";
      }
      if (p === "timg" || p === "image" || p === "img") {
        const item = entries[index];
        if (!item) return "";
        const enclosure = item.querySelector('enclosure[type^="image"]');
        if (enclosure) return enclosure.getAttribute("url") || "";
        const mediaContent = item.querySelector("content[url]");
        if (mediaContent) return mediaContent.getAttribute("url") || "";
        // Try extracting image from description/content HTML
        const descEl = item.querySelector("description, summary, content");
        const descHtml = descEl?.textContent || "";
        const imgMatch = descHtml.match(/<img[^>]+src=["']([^"']+)["']/);
        if (imgMatch) return imgMatch[1];
        return "";
      }
      // Feed-level metadata
      if (p === "ftitle") {
        return doc.querySelector("channel > title, feed > title")?.textContent?.trim() || "";
      }
      if (p === "fdesc") {
        return doc.querySelector("channel > description, feed > subtitle")?.textContent?.trim() || "";
      }
      if (p === "fimg") {
        return doc.querySelector("channel > image > url")?.textContent?.trim() || "";
      }
      if (p === "cnt" || p === "count") {
        return String(entries.length);
      }
      // Default: item title
      return entries[index]?.querySelector("title")?.textContent?.trim() || "";
    } catch {
      return "[rss parse error]";
    }
  }

  if (type === "json") {
    try {
      let data: any = JSON.parse(raw);
      if (path) {
        for (const part of path.split(".")) {
          if (data == null) break;
          data = Array.isArray(data) ? data[parseInt(part)] : data[part];
        }
      }
      if (Array.isArray(data)) data = data[index];
      if (data == null) return "";
      return typeof data === "object" ? JSON.stringify(data) : String(data);
    } catch {
      return "[json parse error]";
    }
  }

  // txt / html — return lines or raw
  if (type === "txt" || type === "text" || type === "html") {
    const lines = raw.split("\n").filter((l) => l.trim());
    if (index > 0 || path === "line") return lines[index] || "";
    return raw.substring(0, 500);
  }

  return raw.substring(0, 500);
}

/** Evaluate a $wg(...)$ formula — async fetch with cache */
function evaluateWg(argsStr: string): string {
  const args = splitWgArgs(argsStr);
  const url = args[0] || "";
  const type = (args[1] || "txt").toLowerCase();
  const path = args[2] || "";
  const index = parseInt(args[3] || "0");

  if (!url) return "";

  const cacheKey = JSON.stringify([url, type, path, index]);
  const cached = wgResultCache.get(cacheKey);
  if (cached && Date.now() - cached.ts < WG_CACHE_TTL) return cached.val;

  // Start async fetch if not already in progress
  if (!wgFetching.has(url)) {
    wgFetching.add(url);
    fetch(url)
      .then((r) => r.text())
      .then((text) => {
        wgRawCache.set(url, text);
        wgFetching.delete(url);
        // Clear result cache entries for this URL so they re-extract
        for (const [k] of wgResultCache) {
          if (k.startsWith('["' + url + '"') || k.startsWith("[\"" + url + '"')) {
            wgResultCache.delete(k);
          }
        }
        // Re-queue all wg formulas for re-evaluation
        for (const [k] of cache) {
          if (k.includes("wg(")) pending.add(k);
        }
      })
      .catch((e) => {
        console.error("[wg] fetch error:", url, e);
        wgFetching.delete(url);
      });
  }

  // Try to extract from raw cache (may exist from a previous fetch)
  const raw = wgRawCache.get(url);
  if (raw) {
    const val = extractWgData(raw, type, path, index);
    wgResultCache.set(cacheKey, { val, ts: Date.now() });
    return val;
  }

  return cached?.val ?? "";
}

/** Split formula arguments respecting nested parentheses and quotes */
function splitArgs(s: string): string[] {
  const args: string[] = [];
  let current = "";
  let depth = 0;
  let inQuote = false;
  let qChar = "";
  for (const ch of s) {
    if (inQuote) {
      if (ch === qChar) inQuote = false;
      else current += ch;
    } else if (ch === '"' || ch === "'") {
      inQuote = true;
      qChar = ch;
    } else if (ch === "(") {
      depth++;
      current += ch;
    } else if (ch === ")") {
      depth--;
      current += ch;
    } else if (ch === "," && depth === 0) {
      args.push(current.trim());
      current = "";
    } else {
      current += ch;
    }
  }
  args.push(current.trim());
  return args;
}

/** Parse a hex color string (with or without #) to [r, g, b, a] */
function parseColor(c: string): [number, number, number, number] {
  c = c.replace(/^#/, "");
  if (c.length === 3) c = c[0]+c[0]+c[1]+c[1]+c[2]+c[2];
  if (c.length === 6) c = "FF" + c;  // KLWP uses AARRGGBB
  if (c.length === 8) {
    const a = parseInt(c.slice(0, 2), 16);
    const r = parseInt(c.slice(2, 4), 16);
    const g = parseInt(c.slice(4, 6), 16);
    const b = parseInt(c.slice(6, 8), 16);
    return [r, g, b, a];
  }
  return [0, 0, 0, 255];
}

/** Convert [r, g, b, a] to AARRGGBB hex string */
function toHexColor(r: number, g: number, b: number, a: number): string {
  const clamp = (v: number) => Math.max(0, Math.min(255, Math.round(v)));
  return "#" + [a, r, g, b].map(v => clamp(v).toString(16).padStart(2, "0")).join("").toUpperCase();
}

/** Convert RGB to HSL (returns h 0-360, s 0-100, l 0-100) */
function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l * 100];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return [h * 360, s * 100, l * 100];
}

/** Convert HSL (h 0-360, s 0-100, l 0-100) to RGB */
function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  h /= 360; s /= 100; l /= 100;
  if (s === 0) { const v = Math.round(l * 255); return [v, v, v]; }
  const hue2rgb = (p: number, q: number, t: number) => {
    if (t < 0) t += 1; if (t > 1) t -= 1;
    if (t < 1/6) return p + (q - p) * 6 * t;
    if (t < 1/2) return q;
    if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
    return p;
  };
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  return [
    Math.round(hue2rgb(p, q, h + 1/3) * 255),
    Math.round(hue2rgb(p, q, h) * 255),
    Math.round(hue2rgb(p, q, h - 1/3) * 255),
  ];
}

/** Evaluate $dp(part)$ — date part */
function evaluateDp(part: string): string {
  const now = new Date();
  switch (part) {
    case "h": return String(now.getHours());
    case "m": return String(now.getMinutes());
    case "s": return String(now.getSeconds());
    case "d": return String(now.getDate());
    case "M": return String(now.getMonth() + 1);
    case "y": return String(now.getFullYear());
    case "w": return String(now.getDay() + 1); // Sun=1
    default: return "";
  }
}

/** Evaluate $tu(mode)$ — unix timestamp */
function evaluateTu(mode: string): string {
  if (mode === "ms") return String(Date.now());
  return String(Math.floor(Date.now() / 1000));
}

/** Evaluate $tf(secs, format?)$ — time format */
function evaluateTf(argsStr: string): string {
  const args = splitArgs(argsStr);
  const secs = parseInt(args[0]) || 0;
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;

  if (args.length < 2 || !args[1]) {
    return h > 0
      ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
      : `${m}:${String(s).padStart(2, "0")}`;
  }

  const fmt = args[1];
  // Replace longest tokens first
  return fmt
    .replace(/hh/g, String(h).padStart(2, "0"))
    .replace(/mm/g, String(m).padStart(2, "0"))
    .replace(/ss/g, String(s).padStart(2, "0"))
    .replace(/\bh\b/g, String(h))
    .replace(/\bm\b/g, String(m))
    .replace(/\bs\b/g, String(s));
}

/** Evaluate $mu(func, args...)$ — math utilities */
function evaluateMu(argsStr: string): string {
  const args = splitArgs(argsStr);
  const fn = args[0]?.toLowerCase() ?? "";
  const nums = args.slice(1).map(Number);
  const deg2rad = (d: number) => (d * Math.PI) / 180;
  const rad2deg = (r: number) => (r * 180) / Math.PI;

  switch (fn) {
    case "ceil": return String(Math.ceil(nums[0]));
    case "floor": return String(Math.floor(nums[0]));
    case "round": return String(Math.round(nums[0]));
    case "abs": return String(Math.abs(nums[0]));
    case "sin": return String(Math.sin(deg2rad(nums[0])));
    case "cos": return String(Math.cos(deg2rad(nums[0])));
    case "tan": return String(Math.tan(deg2rad(nums[0])));
    case "asin": return String(rad2deg(Math.asin(nums[0])));
    case "acos": return String(rad2deg(Math.acos(nums[0])));
    case "atan": return String(rad2deg(Math.atan(nums[0])));
    case "log": return String(Math.log10(nums[0]));
    case "ln": return String(Math.log(nums[0]));
    case "pow": return String(Math.pow(nums[0], nums[1]));
    case "sqrt": return String(Math.sqrt(nums[0]));
    case "min": return String(Math.min(...nums));
    case "max": return String(Math.max(...nums));
    case "rnd": {
      const lo = nums[0] ?? 0, hi = nums[1] ?? 1;
      return String(Math.floor(Math.random() * (hi - lo + 1)) + lo);
    }
    case "add": return String((nums[0] || 0) + (nums[1] || 0));
    case "sub": return String((nums[0] || 0) - (nums[1] || 0));
    case "mul": return String((nums[0] || 0) * (nums[1] || 0));
    case "div": return String(nums[1] ? nums[0] / nums[1] : 0);
    case "mod": return String(nums[1] ? nums[0] % nums[1] : 0);
    case "h2d": return String(parseInt(args[1] || "0", 16));
    case "d2h": return Math.round(nums[0]).toString(16).toUpperCase();
    default: return "";
  }
}

/** Evaluate $tc(mode, args...)$ — text utilities */
function evaluateTc(argsStr: string): string {
  const args = splitArgs(argsStr);
  const mode = args[0]?.toLowerCase() ?? "";
  const text = args[1] ?? "";

  switch (mode) {
    case "low": return text.toLowerCase();
    case "up": return text.toUpperCase();
    case "cap": return text.replace(/\b\w/g, c => c.toUpperCase());
    case "cut": {
      const start = parseInt(args[2] || "0");
      const len = parseInt(args[3] || String(text.length));
      return text.substring(start, start + len);
    }
    case "ell": {
      const max = parseInt(args[2] || "10");
      return text.length > max ? text.substring(0, max) + "\u2026" : text;
    }
    case "split": {
      const sep = args[2] ?? " ";
      const idx = parseInt(args[3] || "0");
      return text.split(sep)[idx] ?? "";
    }
    case "len": return String(text.length);
    case "count": {
      const needle = args[2] ?? "";
      if (!needle) return "0";
      let count = 0, pos = 0;
      while ((pos = text.indexOf(needle, pos)) !== -1) { count++; pos += needle.length; }
      return String(count);
    }
    case "lines": return String(text.split("\n").length);
    case "reg": {
      const pattern = args[2] ?? "";
      const replacement = args[3] ?? "";
      try { return text.replace(new RegExp(pattern, "g"), replacement); } catch { return text; }
    }
    case "n2w": {
      const n = parseInt(text);
      if (isNaN(n) || n < 0 || n > 999) return text;
      const ones = ["", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen"];
      const tens = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
      if (n === 0) return "zero";
      if (n < 20) return ones[n];
      if (n < 100) return tens[Math.floor(n / 10)] + (n % 10 ? " " + ones[n % 10] : "");
      return ones[Math.floor(n / 100)] + " hundred" + (n % 100 ? " " + evaluateTc("n2w," + (n % 100)) : "");
    }
    case "ord": {
      const n = parseInt(text);
      if (isNaN(n)) return text;
      const s = ["th", "st", "nd", "rd"];
      const v = n % 100;
      return n + (s[(v - 20) % 10] || s[v] || s[0]);
    }
    case "utf": {
      const code = parseInt(text);
      try { return String.fromCodePoint(code); } catch { return ""; }
    }
    case "roman": {
      let n = parseInt(text);
      if (isNaN(n) || n <= 0 || n > 3999) return text;
      const vals = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
      const syms = ["M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I"];
      let result = "";
      for (let i = 0; i < vals.length; i++) {
        while (n >= vals[i]) { result += syms[i]; n -= vals[i]; }
      }
      return result;
    }
    case "url": return encodeURIComponent(text);
    case "html": return text.replace(/<[^>]*>/g, "");
    case "json": {
      const path = args[2] ?? "";
      try {
        let data: any = JSON.parse(text);
        if (path) { for (const p of path.split(".")) { data = data?.[p]; } }
        if (data == null) return "";
        return typeof data === "object" ? JSON.stringify(data) : String(data);
      } catch { return ""; }
    }
    default: return "";
  }
}

/** Evaluate $ce(color, filter, amount)$ — color edit */
function evaluateCe(argsStr: string): string {
  const args = splitArgs(argsStr);
  const colorStr = args[0] ?? "";
  const filter = args[1] ?? "";
  const amount = parseFloat(args[2] ?? "0");
  const [r, g, b, a] = parseColor(colorStr);

  // If filter looks like a hex color, interpolate (gradient mix)
  if (/^#?[0-9a-fA-F]{3,8}$/.test(filter)) {
    const [r2, g2, b2, a2] = parseColor(filter);
    const t = isNaN(amount) ? 0.5 : amount;
    const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
    return toHexColor(lerp(r, r2, t), lerp(g, g2, t), lerp(b, b2, t), lerp(a, a2, t));
  }

  switch (filter.toLowerCase()) {
    case "invert":
      return toHexColor(255 - r, 255 - g, 255 - b, a);
    case "comp": {
      const [h, s, l] = rgbToHsl(r, g, b);
      const [nr, ng, nb] = hslToRgb((h + 180) % 360, s, l);
      return toHexColor(nr, ng, nb, a);
    }
    case "contrast": {
      const lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
      return lum > 0.5 ? toHexColor(0, 0, 0, a) : toHexColor(255, 255, 255, a);
    }
    case "alpha": {
      const newA = Math.max(0, Math.min(255, Math.round(amount * 255)));
      return toHexColor(r, g, b, newA);
    }
    case "sat": {
      const [h, s, l] = rgbToHsl(r, g, b);
      const [nr, ng, nb] = hslToRgb(h, Math.max(0, Math.min(100, s + amount)), l);
      return toHexColor(nr, ng, nb, a);
    }
    case "lum": {
      const [h, s, l] = rgbToHsl(r, g, b);
      const [nr, ng, nb] = hslToRgb(h, s, Math.max(0, Math.min(100, l + amount)));
      return toHexColor(nr, ng, nb, a);
    }
    default: return toHexColor(r, g, b, a);
  }
}

/** Evaluate $cm(h, s, l)$ — create color from HSL */
function evaluateCm(argsStr: string): string {
  const args = splitArgs(argsStr).map(Number);
  const h = args[0] ?? 0;
  const s = args[1] ?? 100;
  const l = args[2] ?? 50;
  const [r, g, b] = hslToRgb(h, s, l);
  return toHexColor(r, g, b, 255);
}

/** Evaluate $if(cond, then, else)$ — conditional */
function evaluateIf(argsStr: string, globals: Record<string, string>): string {
  const args = splitArgs(argsStr);
  const rawCond = args[0] ?? "";

  // Evaluate any nested formulas in the condition first
  const cond = resolveInnerFormulas(rawCond, globals);

  // Check for comparison operators: =, !=, <, >, <=, >=
  let isTruthy = false;
  const cmpMatch = cond.match(/^(.+?)\s*(!=|<=|>=|=|<|>)\s*(.+)$/);
  if (cmpMatch) {
    let [, left, op, right] = cmpMatch;
    left = left.trim();
    right = right.trim();
    const numL = parseFloat(left);
    const numR = parseFloat(right);
    const isNumeric = !isNaN(numL) && !isNaN(numR);

    switch (op) {
      case "=":
        isTruthy = isNumeric ? numL === numR : left === right;
        break;
      case "!=":
        isTruthy = isNumeric ? numL !== numR : left !== right;
        break;
      case "<":
        isTruthy = isNumeric ? numL < numR : left < right;
        break;
      case ">":
        isTruthy = isNumeric ? numL > numR : left > right;
        break;
      case "<=":
        isTruthy = isNumeric ? numL <= numR : left <= right;
        break;
      case ">=":
        isTruthy = isNumeric ? numL >= numR : left >= right;
        break;
    }
  } else {
    isTruthy = cond !== "" && cond !== "0" && cond !== "false";
  }

  // Also resolve inner formulas in the result values
  const trueVal = args[1] ?? "";
  const falseVal = args[2] ?? "";
  return isTruthy ? trueVal : falseVal;
}

/** Evaluate $fl(init, stop, incr, body, sep)$ — for loop */
function evaluateFl(argsStr: string, _globals: Record<string, string>): string {
  const args = splitArgs(argsStr);
  const init = parseInt(args[0] ?? "0");
  const stop = parseInt(args[1] ?? "0");
  const incr = parseInt(args[2] ?? "1") || 1;
  const body = args[3] ?? "";
  const sep = args[4] ?? "";
  const results: string[] = [];
  const maxIter = 1000; // safety limit
  let count = 0;

  if (incr > 0) {
    for (let i = init; i <= stop && count < maxIter; i += incr, count++) {
      results.push(body.replace(/\$?lv\(i\)\$?/g, String(i)));
    }
  } else if (incr < 0) {
    for (let i = init; i >= stop && count < maxIter; i += incr, count++) {
      results.push(body.replace(/\$?lv\(i\)\$?/g, String(i)));
    }
  }
  return results.join(sep);
}

/** Evaluate simple inline arithmetic: "5+3", "10-1", "4*2", "8/4" */
function evalArithmetic(expr: string): string {
  // Only evaluate if the expression contains arithmetic operators between numbers
  const trimmed = expr.trim();
  // Match: optional negative number, then operator, then number (supports decimals)
  const m = trimmed.match(/^(-?\d+(?:\.\d+)?)\s*([+\-*/])\s*(-?\d+(?:\.\d+)?)$/);
  if (!m) return expr;
  const [, left, op, right] = m;
  const a = parseFloat(left);
  const b = parseFloat(right);
  switch (op) {
    case "+": return String(a + b);
    case "-": return String(a - b);
    case "*": return String(a * b);
    case "/": return b !== 0 ? String(a / b) : "0";
    default: return expr;
  }
}

/** Resolve nested formula calls within an arguments string (innermost first) */
function resolveInnerFormulas(argsStr: string, globals: Record<string, string>): string {
  // Match innermost function calls (no nested parens in args)
  // Exclude lv() — it's resolved by fl() during loop iteration
  const pattern = /\b(df|gv|wg|tf|dp|tu|mu|tc|ce|cm|fl|if|bi|mi|ai|rm|ts|ni|si|wi|wf)\(([^()]*)\)/;
  let result = argsStr;
  let safety = 50;

  while (safety-- > 0) {
    const match = result.match(pattern);
    if (!match) break;

    const [fullMatch] = match;
    const evaluated = evaluateClientSide(`$${fullMatch}$`, globals);
    result = result.slice(0, match.index!) + evaluated + result.slice(match.index! + fullMatch.length);
  }

  // After resolving all function calls, evaluate inline arithmetic in each argument
  // Split by commas (respecting quotes), evaluate arithmetic in each part, rejoin
  const parts = splitArgs(result);
  const evaluated = parts.map(p => evalArithmetic(p));
  return evaluated.join(", ");
}

/** Client-side formula evaluation for non-Tauri context (wallpaper view) */
function evaluateClientSide(formula: string, globals: Record<string, string>): string {
  // Strip outer $...$ delimiters
  let inner = formula.replace(/^\$|\$$/g, "");

  // Resolve nested function calls in arguments before evaluating the outer function
  const outerMatch = inner.match(/^([a-z]{2,4})\((.+)\)$/s);
  if (outerMatch) {
    const [, funcName, argsStr] = outerMatch;
    const resolvedArgs = resolveInnerFormulas(argsStr, globals);
    if (resolvedArgs !== argsStr) {
      inner = `${funcName}(${resolvedArgs})`;
    }
  }

  // $df(pattern)$ - date/time formatting (Java SimpleDateFormat patterns)
  const dfMatch = inner.match(/^df\((.+)\)$/);
  if (dfMatch) {
    return formatDate(dfMatch[1], new Date());
  }

  // $gv(name)$ - global variable lookup
  const gvMatch = inner.match(/^gv\((.+)\)$/);
  if (gvMatch) {
    return globals[gvMatch[1]] ?? "";
  }

  // $wg(url, type, path, index)$ - web get
  const wgMatch = inner.match(/^wg\((.+)\)$/s);
  if (wgMatch) {
    return evaluateWg(wgMatch[1]);
  }

  // $tf(secs, format?)$ - format seconds as time
  const tfMatch = inner.match(/^tf\((.+)\)$/);
  if (tfMatch) {
    return evaluateTf(tfMatch[1]);
  }

  // $dp(part)$ - date part
  const dpMatch = inner.match(/^dp\((.+)\)$/);
  if (dpMatch) {
    return evaluateDp(dpMatch[1]);
  }

  // $tu(mode)$ - unix timestamp
  const tuMatch = inner.match(/^tu\((.+)\)$/);
  if (tuMatch) {
    return evaluateTu(tuMatch[1]);
  }

  // $if(cond, then, else)$ - conditional
  const ifMatch = inner.match(/^if\((.+)\)$/s);
  if (ifMatch) {
    return evaluateIf(ifMatch[1], globals);
  }

  // $mu(func, args...)$ - math utilities
  const muMatch = inner.match(/^mu\((.+)\)$/s);
  if (muMatch) {
    return evaluateMu(muMatch[1]);
  }

  // $tc(mode, args...)$ - text utilities
  const tcMatch = inner.match(/^tc\((.+)\)$/s);
  if (tcMatch) {
    return evaluateTc(tcMatch[1]);
  }

  // $ce(color, filter, amount)$ - color edit
  const ceMatch = inner.match(/^ce\((.+)\)$/s);
  if (ceMatch) {
    return evaluateCe(ceMatch[1]);
  }

  // $cm(h, s, l)$ - create color from HSL
  const cmMatch = inner.match(/^cm\((.+)\)$/);
  if (cmMatch) {
    return evaluateCm(cmMatch[1]);
  }

  // $lv(name)$ - local variable (no loop context client-side)
  const lvMatch = inner.match(/^lv\((.+)\)$/);
  if (lvMatch) {
    return "";
  }

  // $fl(init, stop, incr, body, sep)$ - for loop
  const flMatch = inner.match(/^fl\((.+)\)$/s);
  if (flMatch) {
    return evaluateFl(flMatch[1], globals);
  }

  // Provider formulas: mi(field), bi(field), wi(field), wf(day, field), etc.
  const providerMatch = inner.match(/^([a-z]{2})\((.+)\)$/);
  if (providerMatch) {
    const [, prefix, argsStr] = providerMatch;
    const provider = providerData[prefix];
    if (provider) {
      const parts = argsStr.split(",").map((s) => s.trim());
      // Multi-arg: join with "_" (e.g. wf(0, temp) -> "0_temp")
      const key = parts.length > 1 ? parts.join("_") : parts[0];
      return provider[key] ?? "";
    }
  }

  return "";
}

/** Format a date using Java SimpleDateFormat-style patterns */
function formatDate(pattern: string, date: Date): string {
  const days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
  const months = ["January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December"];
  const daysShort = days.map(d => d.slice(0, 3));
  const monthsShort = months.map(m => m.slice(0, 3));

  const h24 = date.getHours();
  const h12 = h24 % 12 || 12;
  const min = date.getMinutes();
  const sec = date.getSeconds();
  const day = date.getDate();
  const dow = date.getDay();
  const month = date.getMonth();
  const year = date.getFullYear();
  const ampm = h24 < 12 ? "AM" : "PM";

  // Single-pass replacement so output values (e.g. "Sunday", "March")
  // are never re-scanned by later token patterns
  const tokens: Record<string, string> = {
    "EEEE": days[dow],
    "EEE": daysShort[dow],
    "EE": daysShort[dow],
    "MMMM": months[month],
    "MMM": monthsShort[month],
    "MM": String(month + 1).padStart(2, "0"),
    "dd": String(day).padStart(2, "0"),
    "d": String(day),
    "yyyy": String(year),
    "yy": String(year).slice(-2),
    "HH": String(h24).padStart(2, "0"),
    "H": String(h24),
    "hh": String(h12).padStart(2, "0"),
    "h": String(h12),
    "mm": String(min).padStart(2, "0"),
    "ss": String(sec).padStart(2, "0"),
    "a": ampm,
  };
  const tokenRegex = new RegExp(
    Object.keys(tokens).sort((a, b) => b.length - a.length).join("|"),
    "g",
  );
  return pattern.replace(tokenRegex, (m) => tokens[m]);
}

/** Resolve globals — evaluate any global values that contain formulas */
function resolveGlobals(rawGlobals: Record<string, string>): Record<string, string> {
  const resolved: Record<string, string> = {};
  for (const [name, value] of Object.entries(rawGlobals)) {
    if (hasFormula(value)) {
      resolved[name] = resolveFormula(value);
    } else {
      resolved[name] = value;
    }
  }
  return resolved;
}

/** Evaluate all pending formulas in a batch */
async function flushPending(globals: Record<string, string>) {
  if (pending.size === 0 || evaluating) return;
  evaluating = true;

  const batch = [...pending];
  pending.clear();

  // Separate wg() formulas — always handled client-side (async HTTP)
  const wgFormulas: string[] = [];
  const otherFormulas: string[] = [];
  for (const f of batch) {
    if (f.includes("wg(")) wgFormulas.push(f);
    else otherFormulas.push(f);
  }

  // Evaluate wg() formulas client-side (both Tauri and non-Tauri)
  for (const formula of wgFormulas) {
    cache.set(formula, evaluateClientSide(formula, globals));
  }

  if (!isTauri) {
    // Non-Tauri: evaluate everything client-side
    for (const formula of otherFormulas) {
      cache.set(formula, evaluateClientSide(formula, globals));
    }
    evaluating = false;
    return;
  }

  // Tauri: send non-wg formulas to Rust backend
  if (otherFormulas.length > 0) {
    try {
      const results = await Promise.all(
        otherFormulas.map(async (formula) => {
          try {
            const result = await tauriInvoke<string>("evaluate_formula", {
              formula,
              globals,
            });
            return { formula, result };
          } catch {
            return { formula, result: `[err]` };
          }
        })
      );

      for (const { formula, result } of results) {
        cache.set(formula, result);
      }
    } finally {
      evaluating = false;
    }
  } else {
    evaluating = false;
  }
}

/** Start the formula evaluation loop */
let intervalId: ReturnType<typeof setInterval> | null = null;

export function startFormulaLoop(getGlobals: () => Record<string, string>) {
  if (intervalId) return;

  // Fetch provider data for non-Tauri wallpaper view
  if (!isTauri) fetchProviderData();

  // Evaluate pending formulas and refresh time-sensitive ones every second
  intervalId = setInterval(() => {
    // Refresh provider data for wallpaper view
    if (!isTauri) fetchProviderData();

    // Queue time-dependent and web-get formulas for re-evaluation
    for (const [key] of cache) {
      if (key.match(/\b(df|dp|tf|tu|ai|mi|bi|rm|ts|wg|gv|mu|wi|wf)\(/)) {
        pending.add(key);
      }
    }
    // Resolve globals (evaluates formula values in globals before use)
    const resolved = resolveGlobals(getGlobals());
    flushPending(resolved);
  }, 1000);

  // Initial flush
  const resolved = resolveGlobals(getGlobals());
  flushPending(resolved);
}

export function stopFormulaLoop() {
  if (intervalId) {
    clearInterval(intervalId);
    intervalId = null;
  }
}

/**
 * Invalidate all cached formulas that reference global variables (gv).
 * Marks them as pending for re-evaluation on the next tick without removing
 * the stale value from cache, so the UI won't flash.
 */
/**
 * Synchronously evaluate a formula string (for click actions, URL resolution, etc.)
 * Uses cached RSS data from wgRawCache when available.
 */
export function evaluateSync(formula: string, globals: Record<string, string>): string {
  if (!hasFormula(formula)) return formula;
  return formula.replace(formulaPattern, (match) => {
    // Try cache first
    if (cache.has(match)) return cache.get(match)!;
    // Evaluate client-side (works if wgRawCache has the data)
    return evaluateClientSide(match, globals);
  });
}

export function invalidateGlobalsFormulas() {
  for (const [key] of cache) {
    if (key.includes("gv(")) {
      pending.add(key);
    }
  }
}

/**
 * Immediately re-evaluate all pending formulas that depend on globals.
 * Call after changing a global variable to avoid 1-second stale visibility.
 */
export function flushGlobalsNow(globals: Record<string, string>) {
  // Re-evaluate all cached formulas that depend on globals (not just pending ones)
  for (const key of [...cache.keys()]) {
    if (key.includes("gv(") || key.includes("if(")) {
      cache.set(key, evaluateClientSide(key, globals));
      pending.delete(key);
    }
  }
  // Also flush any remaining pending global-dependent formulas
  for (const key of [...pending]) {
    if (key.includes("gv(") || key.includes("if(")) {
      cache.set(key, evaluateClientSide(key, globals));
      pending.delete(key);
    }
  }
}
