<script lang="ts">
  interface Props {
    value: string | undefined;
    defaultColor?: string;
    onChange: (val: string) => void;
  }
  let { value, defaultColor = "#ffffff", onChange }: Props = $props();

  // Auto-detect formula mode: if value contains $ or doesn't look like a plain hex
  function looksLikeFormula(v: string | undefined): boolean {
    if (!v) return false;
    return v.includes("$") || v.startsWith("if(") || v.startsWith("rgb(");
  }

  let formulaMode = $state(looksLikeFormula(value));

  $effect(() => {
    if (looksLikeFormula(value)) formulaMode = true;
  });

  // Clamp to valid 6-char hex for the color picker input
  function safeHex(v: string | undefined): string {
    if (!v) return defaultColor;
    if (/^#[0-9a-fA-F]{6}$/.test(v)) return v;
    return defaultColor;
  }
</script>

{#if formulaMode}
  <div class="cf-row">
    <input
      class="cf-text"
      type="text"
      value={value ?? ""}
      placeholder="$if(gv(x)=y, #hex1, #hex2)$"
      spellcheck="false"
      oninput={(e) => onChange((e.target as HTMLInputElement).value)}
    />
    <button class="cf-btn" title="Switch to color picker" onclick={() => { formulaMode = false; }}>◎</button>
  </div>
{:else}
  <div class="cf-row">
    <input
      class="cf-color"
      type="color"
      value={safeHex(value)}
      oninput={(e) => onChange((e.target as HTMLInputElement).value)}
    />
    <input
      class="cf-hex"
      type="text"
      value={looksLikeFormula(value) ? "" : (value ?? defaultColor)}
      placeholder={defaultColor}
      spellcheck="false"
      oninput={(e) => onChange((e.target as HTMLInputElement).value)}
    />
    <button class="cf-btn cf-fx" title="Enter formula instead" onclick={() => { formulaMode = true; }}>fx</button>
  </div>
{/if}

<style>
  .cf-row {
    display: flex;
    align-items: center;
    gap: 3px;
    width: 100%;
  }
  .cf-color {
    width: 28px;
    height: 24px;
    padding: 1px 2px;
    flex-shrink: 0;
    border-radius: 3px;
    cursor: pointer;
  }
  .cf-hex {
    flex: 1;
    min-width: 0;
    font-size: 11px;
    font-family: var(--font-mono);
    padding: 2px 4px;
  }
  .cf-text {
    flex: 1;
    min-width: 0;
    font-size: 11px;
    font-family: var(--font-mono);
    padding: 2px 4px;
  }
  .cf-btn {
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    border-radius: 3px;
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text-muted);
    font-size: 10px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
  }
  .cf-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .cf-fx {
    color: var(--accent);
    border-color: var(--accent-dim);
    background: var(--accent-dim);
  }
  .cf-fx:hover {
    background: var(--accent);
    color: #fff;
  }
</style>
