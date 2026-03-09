<script lang="ts">
  import { getSelectedLayer, addAnimation, updateAnimation, removeAnimation } from "../stores/project.svelte";
  import type { Animation, AnimationType, AnimationTrigger, EasingType } from "../types/project";

  const animTypes: { value: AnimationType; label: string }[] = [
    { value: "fade", label: "Fade" },
    { value: "rotate", label: "Rotate" },
    { value: "scale", label: "Scale" },
    { value: "translate", label: "Translate" },
    { value: "blur", label: "Blur" },
    { value: "color", label: "Color" },
  ];

  const triggerTypes: { value: AnimationTrigger; label: string }[] = [
    { value: "scroll", label: "Scroll" },
    { value: "tap", label: "Tap" },
    { value: "show", label: "Show" },
    { value: "time", label: "Time" },
    { value: "reactive", label: "Reactive" },
  ];

  const easingTypes: { value: EasingType; label: string }[] = [
    { value: "linear", label: "Linear" },
    { value: "ease-in", label: "Ease In" },
    { value: "ease-out", label: "Ease Out" },
    { value: "ease-in-out", label: "Ease In-Out" },
    { value: "bounce", label: "Bounce" },
    { value: "elastic", label: "Elastic" },
  ];

  const loopModes: { value: string; label: string }[] = [
    { value: "none", label: "None" },
    { value: "restart", label: "Restart" },
    { value: "reverse", label: "Reverse" },
  ];

  function createDefaultAnimation(): Animation {
    return {
      type: "fade",
      trigger: "show",
      rule: "",
      amount: 255,
      speed: 1000,
      easing: "ease-out",
      delay: 0,
      loop: "none",
    };
  }

  function handleAdd() {
    const layer = getSelectedLayer();
    if (!layer) return;
    addAnimation(layer.id, createDefaultAnimation());
  }

  function handleUpdate(index: number, field: keyof Animation, value: any) {
    const layer = getSelectedLayer();
    if (!layer || !layer.animations) return;
    const anim = { ...layer.animations[index], [field]: value };
    updateAnimation(layer.id, index, anim);
  }

  function handleNumericUpdate(index: number, field: keyof Animation, e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    if (!isNaN(val)) handleUpdate(index, field, val);
  }

  function handleRemove(index: number) {
    const layer = getSelectedLayer();
    if (!layer) return;
    removeAnimation(layer.id, index);
  }

  function getAmountLabel(type: AnimationType): string {
    switch (type) {
      case "fade": return "Target Alpha (0-255)";
      case "rotate": return "Degrees";
      case "scale": return "Scale Factor";
      case "translate": return "Pixels";
      case "blur": return "Blur Radius (px)";
      case "color": return "Strength";
    }
  }

  function getRuleLabel(trigger: AnimationTrigger, type: AnimationType): string | null {
    if (type === "translate") return "Direction (x, y, or x,y)";
    if (type === "color") return "Target Color (#hex)";
    if (trigger === "reactive") return "Formula ($...$)";
    return null;
  }
</script>

{#if getSelectedLayer()}
  {@const layer = getSelectedLayer()!}
  {@const anims = layer.animations || []}

  <section class="prop-section">
    <div class="section-title">
      Animations
      <button class="add-btn" title="Add animation" onclick={handleAdd}>+</button>
    </div>

    {#if anims.length === 0}
      <div class="empty-hint">No animations. Click + to add one.</div>
    {/if}

    {#each anims as anim, i}
      <div class="anim-card">
        <div class="anim-header">
          <span class="anim-label">{anim.type} / {anim.trigger}</span>
          <button class="remove-btn" title="Remove animation" onclick={() => handleRemove(i)}>x</button>
        </div>

        <div class="anim-fields">
          <label>Type</label>
          <select value={anim.type} onchange={(e) => handleUpdate(i, "type", (e.target as HTMLSelectElement).value)}>
            {#each animTypes as t}
              <option value={t.value}>{t.label}</option>
            {/each}
          </select>

          <label>Trigger</label>
          <select value={anim.trigger} onchange={(e) => handleUpdate(i, "trigger", (e.target as HTMLSelectElement).value)}>
            {#each triggerTypes as t}
              <option value={t.value}>{t.label}</option>
            {/each}
          </select>

          <label>{getAmountLabel(anim.type)}</label>
          <input type="number" value={anim.amount} step="any" oninput={(e) => handleNumericUpdate(i, "amount", e)} />

          {#if getRuleLabel(anim.trigger, anim.type)}
            <label>{getRuleLabel(anim.trigger, anim.type)}</label>
            <input type="text" value={anim.rule} oninput={(e) => handleUpdate(i, "rule", (e.target as HTMLInputElement).value)} />
          {/if}

          {#if anim.trigger !== "scroll" && anim.trigger !== "reactive"}
            <label>Speed (ms)</label>
            <input type="number" value={anim.speed ?? 1000} min="50" step="50" oninput={(e) => handleNumericUpdate(i, "speed", e)} />

            <label>Delay (ms)</label>
            <input type="number" value={anim.delay ?? 0} min="0" step="50" oninput={(e) => handleNumericUpdate(i, "delay", e)} />
          {/if}

          <label>Easing</label>
          <select value={anim.easing ?? "linear"} onchange={(e) => handleUpdate(i, "easing", (e.target as HTMLSelectElement).value)}>
            {#each easingTypes as e}
              <option value={e.value}>{e.label}</option>
            {/each}
          </select>

          <label>Loop</label>
          <select value={anim.loop ?? "none"} onchange={(e) => handleUpdate(i, "loop", (e.target as HTMLSelectElement).value)}>
            {#each loopModes as m}
              <option value={m.value}>{m.label}</option>
            {/each}
          </select>
        </div>
      </div>
    {/each}
  </section>
{/if}

<style>
  .prop-section {
    padding: 0 12px;
    margin-bottom: 12px;
  }
  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: 6px;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .add-btn {
    width: 20px;
    height: 20px;
    border-radius: 3px;
    background: var(--accent-dim);
    color: var(--accent);
    font-size: 14px;
    font-weight: 700;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    border: none;
  }
  .add-btn:hover {
    background: var(--accent);
    color: #fff;
  }
  .empty-hint {
    font-size: 11px;
    color: var(--text-muted);
    padding: 8px 0;
  }
  .anim-card {
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 6px;
    overflow: hidden;
  }
  .anim-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }
  .anim-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
    text-transform: capitalize;
  }
  .remove-btn {
    width: 18px;
    height: 18px;
    border-radius: 3px;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    border: none;
  }
  .remove-btn:hover {
    background: #c0392b;
    color: #fff;
  }
  .anim-fields {
    padding: 6px 8px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .anim-fields label {
    font-size: 10px;
    color: var(--text-secondary);
    margin-top: 2px;
  }
  .anim-fields label:first-child {
    margin-top: 0;
  }
  .anim-fields input,
  .anim-fields select {
    width: 100%;
    font-size: 11px;
    padding: 2px 4px;
  }
</style>
