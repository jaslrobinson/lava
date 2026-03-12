<script lang="ts">
  import { getProject, flattenLayers } from "../stores/project.svelte";
  import type { Animation, AnimationType, AnimationTrigger, EasingType } from "../types/project";

  interface Props {
    animation: Animation;
    onApply: (anim: Animation) => void;
    onClose: () => void;
  }
  let { animation, onApply, onClose }: Props = $props();

  let draft = $state<Animation>({
    type: animation.type,
    trigger: animation.trigger,
    rule: animation.rule ?? "",
    amount: animation.amount,
    speed: animation.speed ?? 300,
    easing: animation.easing ?? "ease-out",
    delay: animation.delay ?? 0,
    loop: animation.loop ?? "none",
    colorTarget: animation.colorTarget ?? "#ffffff",
  });

  let hoverInsertSpeed = $state(300);

  const allLayers = $derived(flattenLayers(getProject().layers));

  const TRIGGERS: { value: AnimationTrigger; label: string; desc: string }[] = [
    { value: "hover", label: "Hover", desc: "Animates when this layer is hovered. Reverses when hover ends." },
    { value: "tap", label: "Tap", desc: "Plays once when this layer is clicked." },
    { value: "show", label: "Show", desc: "Plays once each time this layer becomes visible." },
    { value: "time", label: "Time", desc: "Progress driven by elapsed time. Use with Loop for continuous effects." },
    { value: "reactive", label: "Reactive", desc: "Progress driven by a formula. Use $hover() and $hoverProgress() to react to another layer's hover state." },
    { value: "scroll", label: "Scroll", desc: "Progress follows scroll position (0 to 1)." },
  ];

  const TYPES: { value: AnimationType; label: string; icon: string; desc: string; amountLabel: string }[] = [
    { value: "fade", label: "Fade", icon: "◎", desc: "Animate opacity. Amount is the target alpha (0–255).", amountLabel: "Target Opacity (0–255)" },
    { value: "scale", label: "Scale", icon: "⤢", desc: "Scale the layer. Amount is the scale factor (1.0 = normal, 1.2 = 20% larger).", amountLabel: "Scale Factor (e.g. 1.2)" },
    { value: "rotate", label: "Rotate", icon: "↻", desc: "Rotate the layer. Amount is the target angle in degrees.", amountLabel: "Degrees" },
    { value: "translate", label: "Move", icon: "↕", desc: "Shift position in pixels. Set direction with the rule field.", amountLabel: "Pixels" },
    { value: "blur", label: "Blur", icon: "◌", desc: "Blur the layer. Amount is the radius in pixels.", amountLabel: "Blur Radius (px)" },
    { value: "color", label: "Color", icon: "◉", desc: "Tint the layer with a color. Set the target color below. Use Reactive trigger to tint based on another layer's hover.", amountLabel: "Tint Strength (0–1)" },
    { value: "jiggle", label: "Jiggle", icon: "〜", desc: "Oscillating shake effect. Amount is max rotation in degrees.", amountLabel: "Max Rotation (°)" },
    { value: "flash", label: "Flash", icon: "⚡", desc: "Brief brightness flash that decays to normal. Great for tap feedback.", amountLabel: "Flash Intensity (0–255)" },
  ];

  const EASINGS: { value: EasingType; label: string }[] = [
    { value: "linear", label: "Linear" },
    { value: "ease-out", label: "Ease Out" },
    { value: "ease-in", label: "Ease In" },
    { value: "ease-in-out", label: "Ease In-Out" },
    { value: "bounce", label: "Bounce" },
    { value: "elastic", label: "Elastic" },
  ];

  interface Preset {
    label: string;
    desc: string;
    anim: Partial<Animation>;
  }

  const PRESETS: Preset[] = [
    { label: "Hover Fade", desc: "Fade in on hover", anim: { type: "fade", trigger: "hover", amount: 255, speed: 200, easing: "ease-out", loop: "none", rule: "" } },
    { label: "Hover Scale", desc: "Grow slightly on hover", anim: { type: "scale", trigger: "hover", amount: 1.1, speed: 200, easing: "ease-out", loop: "none", rule: "" } },
    { label: "Hover Glow", desc: "Color tint on hover", anim: { type: "color", trigger: "hover", amount: 0.4, speed: 200, easing: "ease-out", loop: "none", rule: "#ffffff" } },
    { label: "Click Flash", desc: "Quick flash on tap", anim: { type: "flash", trigger: "tap", amount: 200, speed: 300, easing: "ease-out", loop: "none", rule: "" } },
    { label: "Bounce In", desc: "Bouncy scale on appear", anim: { type: "scale", trigger: "show", amount: 1.15, speed: 500, easing: "bounce", loop: "none", rule: "" } },
    { label: "Fade In", desc: "Smooth fade on appear", anim: { type: "fade", trigger: "show", amount: 255, speed: 400, easing: "ease-in-out", delay: 0, loop: "none", rule: "" } },
    { label: "Spin Loop", desc: "Continuous rotation", anim: { type: "rotate", trigger: "time", amount: 360, speed: 2000, easing: "linear", loop: "restart", rule: "" } },
    { label: "Scroll Parallax", desc: "Shift vertically with scroll", anim: { type: "translate", trigger: "scroll", rule: "y", amount: 80 } },
    { label: "Cross Hover Scale", desc: "Scale when another layer is hovered", anim: { type: "scale", trigger: "reactive", rule: "", amount: 1.1, easing: "ease-out" } },
    { label: "Cross Hover Fade", desc: "Fade when another layer is hovered", anim: { type: "fade", trigger: "reactive", rule: "", amount: 255, easing: "ease-out" } },
    { label: "Cross Hover Color", desc: "Tint when another layer is hovered", anim: { type: "color", trigger: "reactive", rule: "", colorTarget: "#ffffff", amount: 0.5, easing: "ease-out" } },
  ];

  function applyPreset(p: Preset) {
    draft = { ...draft, ...p.anim } as Animation;
  }

  function insertHover(layerId: string) {
    draft.rule = `$hover('${layerId}')`;
  }

  function insertHoverProgress(layerId: string) {
    draft.rule = `$hoverProgress('${layerId}', ${hoverInsertSpeed})`;
  }

  const currentTrigger = $derived(TRIGGERS.find(t => t.value === draft.trigger));
  const currentType = $derived(TYPES.find(t => t.value === draft.type));
  const showSpeedDelay = $derived(draft.trigger !== "scroll" && draft.trigger !== "reactive");
  const showRule = $derived(draft.type === "translate" || draft.type === "color" || draft.type === "jiggle" || draft.trigger === "reactive");
  const isReactive = $derived(draft.trigger === "reactive");
  const isColorType = $derived(draft.type === "color");

  // Color palette — which field gets updated depends on reactive mode
  function setColor(hex: string) {
    if (isColorType && isReactive) draft.colorTarget = hex;
    else if (isColorType) draft.rule = hex;
  }

  const PALETTE: string[] = [
    // Neutrals
    "#ffffff","#f5f5f5","#e0e0e0","#bdbdbd","#9e9e9e","#757575","#424242","#000000",
    // Warm
    "#fff9c4","#ffe082","#ffb300","#ff6f00","#ff5722","#e94560","#c62828","#ff80ab",
    // Cool
    "#e3f2fd","#81d4fa","#29b6f6","#0288d1","#1565c0","#3949ab","#6c5ce7","#a29bfe",
    // Nature / accent
    "#e8f5e9","#a5d6a7","#4ecca3","#00acc1","#26a69a","#558b2f","#f9ca24","#fd79a8",
  ];
</script>

<div class="overlay" role="dialog" aria-modal="true">
  <div class="builder">
    <div class="bld-header">
      <span class="bld-title">FX Builder</span>
      <button class="x-btn" onclick={onClose}>×</button>
    </div>

    <div class="bld-body">
      <!-- LEFT COL: config -->
      <div class="config-col">

        <div class="sec-label">TRIGGER</div>
        <div class="chip-row">
          {#each TRIGGERS as t}
            <button
              class="chip {draft.trigger === t.value ? 'chip-on' : ''}"
              onclick={() => { draft.trigger = t.value; }}
            >{t.label}</button>
          {/each}
        </div>

        <div class="sec-label" style="margin-top:10px">EFFECT</div>
        <div class="chip-row">
          {#each TYPES as t}
            <button
              class="chip {draft.type === t.value ? 'chip-on' : ''}"
              onclick={() => { draft.type = t.value; }}
            >{t.icon} {t.label}</button>
          {/each}
        </div>

        <div class="sec-label" style="margin-top:10px">PARAMETERS</div>
        <div class="param-grid">

          <span class="plabel">{currentType?.amountLabel ?? "Amount"}</span>
          <input type="number" bind:value={draft.amount} step="any" />

          {#if showRule}
            <span class="plabel">
              {#if draft.type === "translate"}Direction{:else if draft.type === "color" && !isReactive}Target Color{:else if draft.type === "jiggle"}Cycle Period (ms){:else}Formula{/if}
            </span>
            {#if draft.type === "translate"}
              <div class="dir-btns">
                {#each [["x","X"], ["y","Y"], ["x,y","Both"]] as [val, lbl]}
                  <button class="chip {draft.rule === val ? 'chip-on' : ''}" onclick={() => { draft.rule = val; }}>{lbl}</button>
                {/each}
              </div>
            {:else if draft.type === "color" && !isReactive}
              <div class="color-pick-row">
                <input type="color" value={draft.rule || "#ffffff"} oninput={(e) => { draft.rule = (e.target as HTMLInputElement).value; }} />
                <input type="text" bind:value={draft.rule} placeholder="#rrggbb" class="hex-input" />
              </div>
            {:else if isReactive}
              <textarea class="formula-ta" bind:value={draft.rule} rows="2" placeholder="e.g. $hoverProgress('layerId', 300)" spellcheck="false"></textarea>
            {:else}
              <input type="text" bind:value={draft.rule} />
            {/if}
          {/if}

          {#if draft.type === "color" && isReactive}
            <span class="plabel">Target Color</span>
            <div class="color-pick-row">
              <input type="color" value={draft.colorTarget || "#ffffff"} oninput={(e) => { draft.colorTarget = (e.target as HTMLInputElement).value; }} />
              <input type="text" bind:value={draft.colorTarget} placeholder="#rrggbb" class="hex-input" />
            </div>
          {/if}

          {#if showSpeedDelay}
            <span class="plabel">Speed (ms)</span>
            <input type="number" bind:value={draft.speed} min="50" step="50" />

            <span class="plabel">Delay (ms)</span>
            <input type="number" bind:value={draft.delay} min="0" step="50" />
          {/if}

          <span class="plabel">Easing</span>
          <select bind:value={draft.easing}>
            {#each EASINGS as e}
              <option value={e.value}>{e.label}</option>
            {/each}
          </select>

          <span class="plabel">Loop</span>
          <select bind:value={draft.loop}>
            <option value="none">None</option>
            <option value="restart">Restart</option>
            <option value="reverse">Reverse</option>
          </select>

        </div>

        {#if isColorType}
          <div class="sec-label" style="margin-top:10px">COLOR PALETTE</div>
          <div class="palette-grid">
            {#each PALETTE as hex}
              <button
                class="swatch"
                style="background:{hex}; outline: {(isReactive ? draft.colorTarget : draft.rule) === hex ? '2px solid var(--accent)' : '2px solid transparent'};"
                title={hex}
                onclick={() => setColor(hex)}
              ></button>
            {/each}
          </div>
        {/if}

        {#if isReactive}
          <div class="sec-label" style="margin-top:10px">LAYER BROWSER</div>
          <div class="layer-hint">Click to insert a hover formula for that layer</div>
          <div class="speed-row">
            <span>$hoverProgress speed:</span>
            <input type="number" bind:value={hoverInsertSpeed} min="50" step="50" />
            <span>ms</span>
          </div>
          <div class="layer-list">
            {#each allLayers as l}
              <div class="lr">
                <div class="lr-info">
                  <span class="lr-name">{l.name}</span>
                  <span class="lr-type">{l.type}</span>
                </div>
                <div class="lr-btns">
                  <button class="fn-btn" onclick={() => insertHover(l.id)} title="Binary 0/1 hover">$hover</button>
                  <button class="fn-btn fn-prog" onclick={() => insertHoverProgress(l.id)} title="Smooth 0→1 transition">$hoverProgress</button>
                </div>
              </div>
            {/each}
          </div>

          <div class="sec-label" style="margin-top:10px">FORMULA REFERENCE</div>
          <div class="fn-ref">
            <div class="fn-entry">
              <code>$hover('id')</code>
              <span>1 when hovered, 0 otherwise</span>
            </div>
            <div class="fn-entry">
              <code>$hoverProgress('id', ms)</code>
              <span>Smooth 0→1 over ms milliseconds</span>
            </div>
            <div class="fn-entry">
              <code>$isHovered('id')</code>
              <span>Alias for $hover()</span>
            </div>
          </div>
        {/if}

      </div>

      <!-- RIGHT COL: description + presets -->
      <div class="help-col">
        <div class="desc-box">
          {#if currentTrigger}
            <div class="desc-row">
              <span class="desc-key">Trigger:</span>
              <span class="desc-val">{currentTrigger.desc}</span>
            </div>
          {/if}
          {#if currentType}
            <div class="desc-row" style="margin-top:6px">
              <span class="desc-key">Effect:</span>
              <span class="desc-val">{currentType.desc}</span>
            </div>
          {/if}
        </div>

        <div class="sec-label" style="margin-top:12px">PRESETS</div>
        <div class="preset-list">
          {#each PRESETS as p}
            <button class="preset-card" onclick={() => applyPreset(p)}>
              <span class="preset-name">{p.label}</span>
              <span class="preset-desc">{p.desc}</span>
            </button>
          {/each}
        </div>
      </div>
    </div>

    <div class="bld-footer">
      <button class="btn-cancel" onclick={onClose}>Cancel</button>
      <button class="btn-apply" onclick={() => onApply({ ...draft })}>Apply</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .builder {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 720px;
    max-width: 95vw;
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .bld-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }
  .bld-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: 0.3px;
  }
  .x-btn {
    width: 22px;
    height: 22px;
    border-radius: 4px;
    font-size: 16px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .x-btn:hover {
    background: var(--error);
    color: #fff;
  }
  .bld-body {
    display: flex;
    overflow: hidden;
    flex: 1;
    min-height: 0;
  }
  .config-col {
    flex: 1;
    padding: 12px 14px;
    overflow-y: auto;
    border-right: 1px solid var(--border);
  }
  .help-col {
    width: 220px;
    padding: 12px 14px;
    overflow-y: auto;
    flex-shrink: 0;
  }
  .sec-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.8px;
    color: var(--text-muted);
    text-transform: uppercase;
    margin-bottom: 6px;
  }
  .chip-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    padding: 3px 9px;
    font-size: 11px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.1s;
  }
  .chip:hover {
    border-color: var(--accent);
    color: var(--text-primary);
  }
  .chip-on {
    background: var(--accent-dim);
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }
  .param-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 8px;
    align-items: center;
  }
  .plabel {
    font-size: 10px;
    color: var(--text-secondary);
    white-space: nowrap;
  }
  .param-grid input,
  .param-grid select {
    width: 100%;
    font-size: 11px;
    padding: 3px 6px;
  }
  .dir-btns {
    display: flex;
    gap: 4px;
  }
  .color-pick-row {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .color-pick-row input[type="color"] {
    width: 30px;
    height: 26px;
    padding: 1px 2px;
    flex-shrink: 0;
    border-radius: 3px;
    cursor: pointer;
  }
  .hex-input {
    flex: 1;
    font-size: 11px;
    font-family: var(--font-mono);
  }
  .palette-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 3px;
    margin-bottom: 4px;
  }
  .swatch {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 3px;
    border: 1px solid rgba(255,255,255,0.1);
    cursor: pointer;
    padding: 0;
    transition: transform 0.08s, outline-offset 0.08s;
    outline-offset: 1px;
  }
  .swatch:hover {
    transform: scale(1.15);
    border-color: rgba(255,255,255,0.4);
  }
  .formula-ta {
    width: 100%;
    font-family: var(--font-mono);
    font-size: 11px;
    resize: vertical;
    padding: 4px 6px;
  }
  .layer-hint {
    font-size: 10px;
    color: var(--text-muted);
    margin-bottom: 6px;
  }
  .speed-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }
  .speed-row input {
    width: 64px;
    font-size: 11px;
    padding: 2px 4px;
  }
  .layer-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 200px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px;
    background: var(--bg-input);
  }
  .lr {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 3px 4px;
    border-radius: 3px;
    gap: 6px;
  }
  .lr:hover {
    background: var(--bg-secondary);
  }
  .lr-info {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
  }
  .lr-name {
    font-size: 11px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 120px;
  }
  .lr-type {
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .lr-btns {
    display: flex;
    gap: 3px;
    flex-shrink: 0;
  }
  .fn-btn {
    font-size: 10px;
    padding: 2px 5px;
    border-radius: 3px;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-secondary);
    cursor: pointer;
    font-family: var(--font-mono);
    white-space: nowrap;
  }
  .fn-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .fn-prog {
    color: var(--success);
    border-color: #4ecca340;
  }
  .fn-prog:hover {
    border-color: var(--success);
    background: #4ecca310;
  }
  .fn-ref {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .fn-entry {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 4px 6px;
    background: var(--bg-input);
    border-radius: 3px;
    border: 1px solid var(--border);
  }
  .fn-entry code {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--accent);
  }
  .fn-entry span {
    font-size: 10px;
    color: var(--text-muted);
  }
  .desc-box {
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px 10px;
  }
  .desc-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .desc-key {
    font-size: 10px;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .desc-val {
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .preset-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .preset-card {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 6px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.1s;
  }
  .preset-card:hover {
    border-color: var(--accent);
    background: var(--accent-dim);
  }
  .preset-name {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .preset-desc {
    font-size: 10px;
    color: var(--text-muted);
  }
  .bld-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }
  .btn-cancel {
    padding: 5px 14px;
    font-size: 12px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .btn-cancel:hover {
    border-color: var(--text-muted);
    color: var(--text-primary);
  }
  .btn-apply {
    padding: 5px 18px;
    font-size: 12px;
    font-weight: 600;
    border-radius: 4px;
    border: none;
    background: var(--accent);
    color: #fff;
    cursor: pointer;
  }
  .btn-apply:hover {
    background: var(--accent-hover);
  }
</style>
