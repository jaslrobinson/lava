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

/** Client-side formula evaluation for non-Tauri context (wallpaper view) */
function evaluateClientSide(formula: string): string {
  // Strip outer $...$ delimiters
  const inner = formula.replace(/^\$|\$$/g, "");

  // $df(pattern)$ - date/time formatting (Java SimpleDateFormat patterns)
  const dfMatch = inner.match(/^df\((.+)\)$/);
  if (dfMatch) {
    return formatDate(dfMatch[1], new Date());
  }

  // $tf(seconds)$ - format seconds as time
  const tfMatch = inner.match(/^tf\((\d+)\)$/);
  if (tfMatch) {
    const secs = parseInt(tfMatch[1]);
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    return h > 0
      ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
      : `${m}:${String(s).padStart(2, "0")}`;
  }

  // Unsupported formulas — return empty
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

  // Replace tokens longest-first to avoid partial matches
  return pattern
    .replace(/EEEE/g, days[dow])
    .replace(/EEE/g, daysShort[dow])
    .replace(/EE/g, daysShort[dow])
    .replace(/MMMM/g, months[month])
    .replace(/MMM/g, monthsShort[month])
    .replace(/MM/g, String(month + 1).padStart(2, "0"))
    .replace(/dd/g, String(day).padStart(2, "0"))
    .replace(/d/g, String(day))
    .replace(/yyyy/g, String(year))
    .replace(/yy/g, String(year).slice(-2))
    .replace(/HH/g, String(h24).padStart(2, "0"))
    .replace(/H/g, String(h24))
    .replace(/hh/g, String(h12).padStart(2, "0"))
    .replace(/h/g, String(h12))
    .replace(/mm/g, String(min).padStart(2, "0"))
    .replace(/ss/g, String(sec).padStart(2, "0"))
    .replace(/a/g, ampm);
}

/** Evaluate all pending formulas in a batch */
async function flushPending(globals: Record<string, string>) {
  if (pending.size === 0 || evaluating) return;
  evaluating = true;

  const batch = [...pending];
  pending.clear();

  // In non-Tauri context, evaluate formulas client-side (no Rust backend)
  if (!isTauri) {
    for (const formula of batch) {
      cache.set(formula, evaluateClientSide(formula));
    }
    evaluating = false;
    return;
  }

  try {
    const results = await Promise.all(
      batch.map(async (formula) => {
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
}

/** Start the formula evaluation loop */
let intervalId: ReturnType<typeof setInterval> | null = null;

export function startFormulaLoop(getGlobals: () => Record<string, string>) {
  if (intervalId) return;

  // Evaluate pending formulas and refresh time-sensitive ones every second
  intervalId = setInterval(() => {
    // Queue time-dependent formulas for re-evaluation WITHOUT removing from cache.
    // The stale cached value continues to display until the new result arrives,
    // eliminating the flash/flicker that happened when cache was cleared.
    for (const [key] of cache) {
      if (key.match(/\$(df|tf|tu|ai|mi|bi|rm|ts)\(/)) {
        pending.add(key);
      }
    }
    flushPending(getGlobals());
  }, 1000);

  // Initial flush
  flushPending(getGlobals());
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
export function invalidateGlobalsFormulas() {
  for (const [key] of cache) {
    if (key.includes("gv(")) {
      pending.add(key);
    }
  }
}
