<script lang="ts">
  import { resolveFormula } from "../formula/service";
  import { getProject } from "../stores/project.svelte";
  import { formulaCategories, type FormulaCategory } from "../data/formulaCategories";

  let { value, onchange }: {
    value: string;
    onchange: (v: string) => void;
  } = $props();

  let textareaRef = $state<HTMLTextAreaElement | null>(null);
  let expandedCategory = $state<string | null>(null);
  let showGlobalsPicker = $state(false);
  let showFavoritesPicker = $state(false);

  // Favorites stored in localStorage
  function loadFavorites(): { name: string; formula: string }[] {
    try { return JSON.parse(localStorage.getItem("lava-fav-formulas") ?? "[]"); }
    catch { return []; }
  }
  function saveFavoritesToStorage(favs: { name: string; formula: string }[]) {
    localStorage.setItem("lava-fav-formulas", JSON.stringify(favs));
  }
  let favorites = $state<{ name: string; formula: string }[]>(loadFavorites());

  function addFavorite(name: string, formula: string) {
    if (!favorites.some(f => f.formula === formula)) {
      favorites = [...favorites, { name, formula }];
      saveFavoritesToStorage(favorites);
    }
  }
  function removeFavorite(formula: string) {
    favorites = favorites.filter(f => f.formula !== formula);
    saveFavoritesToStorage(favorites);
  }
  function isFavorite(formula: string) {
    return favorites.some(f => f.formula === formula);
  }

  // Live preview
  let preview = $derived(value ? resolveFormula(value) : "");
  let hasFormula = $derived(value.includes("$"));

  function insertAtCursor(text: string) {
    if (!textareaRef) { onchange(value + text); return; }
    const start = textareaRef.selectionStart ?? value.length;
    const end = textareaRef.selectionEnd ?? value.length;
    const newVal = value.slice(0, start) + text + value.slice(end);
    onchange(newVal);
    setTimeout(() => {
      if (!textareaRef) return;
      const pos = start + text.length;
      textareaRef.selectionStart = textareaRef.selectionEnd = pos;
      textareaRef.focus();
    }, 0);
  }

  function wrapSelection(before: string, after: string) {
    if (!textareaRef) { onchange(value + before + after); return; }
    const start = textareaRef.selectionStart ?? value.length;
    const end = textareaRef.selectionEnd ?? value.length;
    const selected = value.slice(start, end);
    const newVal = value.slice(0, start) + before + selected + after + value.slice(end);
    onchange(newVal);
    setTimeout(() => {
      if (!textareaRef) return;
      textareaRef.selectionStart = start + before.length;
      textareaRef.selectionEnd = start + before.length + selected.length;
      textareaRef.focus();
    }, 0);
  }

  function toggleCategory(id: string) {
    expandedCategory = expandedCategory === id ? null : id;
    showGlobalsPicker = false;
    showFavoritesPicker = false;
  }

  function toggleGlobals() {
    showGlobalsPicker = !showGlobalsPicker;
    showFavoritesPicker = false;
    expandedCategory = null;
  }

  function toggleFavorites() {
    showFavoritesPicker = !showFavoritesPicker;
    showGlobalsPicker = false;
    expandedCategory = null;
  }

  function insertGlobal(name: string) {
    insertAtCursor(`$gv(${name})$`);
    showGlobalsPicker = false;
  }

  function insertFormula(formula: string) {
    insertAtCursor(formula);
  }

  // Globals from project
  let globals = $derived(getProject().globals ?? []);

  // Currently expanded category data
  let expandedCat = $derived(
    expandedCategory ? formulaCategories.find(c => c.id === expandedCategory) ?? null : null
  );
</script>

<div style="display:flex; flex-direction:column; gap:0; margin-bottom:4px;">

  <!-- Text Preview -->
  <div style="font-size:10px; font-weight:600; color:var(--text-secondary); text-transform:uppercase; letter-spacing:0.5px; margin-bottom:3px;">
    Text Preview
  </div>
  <div style="background:var(--bg-secondary); border:1px solid var(--border); border-radius:4px; padding:8px 10px; min-height:32px; margin-bottom:8px; display:flex; align-items:center; gap:8px;">
    {#if hasFormula}
      <span style="width:8px; height:8px; border-radius:50%; background:var(--accent); flex-shrink:0;"></span>
    {/if}
    <span style="font-size:13px; color:var(--text-primary); font-family:var(--font-mono); white-space:pre-wrap; word-break:break-all; flex:1;">
      {preview || "\u00a0"}
    </span>
  </div>

  <!-- Formula Editor label -->
  <div style="font-size:10px; font-weight:600; color:var(--text-secondary); text-transform:uppercase; letter-spacing:0.5px; margin-bottom:3px;">
    Formula Editor
  </div>

  <!-- Textarea -->
  <textarea
    bind:this={textareaRef}
    value={value}
    placeholder="Enter text or $formula$..."
    rows={3}
    oninput={(e) => onchange((e.target as HTMLTextAreaElement).value)}
    style="
      width:100%; box-sizing:border-box; resize:vertical; min-height:60px;
      font-family:var(--font-mono); font-size:12px; padding:6px 8px;
      background:var(--bg-input); border:1px solid var(--border);
      color:var(--text-primary); border-radius:4px; line-height:1.5;
      margin-bottom:4px;
    "
  ></textarea>

  <!-- Toolbar -->
  <div style="display:flex; align-items:center; gap:3px; padding:4px 0; border-top:1px solid var(--border); border-bottom:1px solid var(--border); margin-bottom:6px; flex-wrap:wrap;">
    <!-- Globals button -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      title="Insert Global Variable"
      onclick={toggleGlobals}
      style="
        display:inline-flex; align-items:center; justify-content:center;
        width:28px; height:28px; border-radius:4px; cursor:pointer; font-size:14px;
        background:{showGlobalsPicker ? 'var(--accent-dim)' : 'transparent'};
        color:{showGlobalsPicker ? 'var(--accent)' : 'var(--text-secondary)'};
        user-select:none;
      "
    >◉</span>

    <!-- Color formula button -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      title="Insert Color Formula"
      onclick={() => { toggleCategory("color"); }}
      style="
        display:inline-flex; align-items:center; justify-content:center;
        width:28px; height:28px; border-radius:4px; cursor:pointer; font-size:14px;
        background:{expandedCategory === 'color' ? 'var(--accent-dim)' : 'transparent'};
        color:{expandedCategory === 'color' ? 'var(--accent)' : 'var(--text-secondary)'};
        user-select:none;
      "
    >🎨</span>

    <!-- Favorites button -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      title="Fave Formulas"
      onclick={toggleFavorites}
      style="
        display:inline-flex; align-items:center; justify-content:center;
        width:28px; height:28px; border-radius:4px; cursor:pointer; font-size:14px;
        background:{showFavoritesPicker ? 'var(--accent-dim)' : 'transparent'};
        color:{showFavoritesPicker ? 'var(--accent)' : 'var(--text-secondary)'};
        user-select:none;
      "
    >★</span>

    <div style="width:1px; height:18px; background:var(--border); margin:0 3px; flex-shrink:0;"></div>

    <!-- BB Code: Bold -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      title="Bold [b]...[/b]"
      onclick={() => wrapSelection("[b]", "[/b]")}
      style="
        display:inline-flex; align-items:center; justify-content:center;
        width:28px; height:28px; border-radius:4px; cursor:pointer; font-size:13px; font-weight:700;
        background:transparent; color:var(--text-secondary); user-select:none;
      "
    >B</span>

    <!-- BB Code: Font size -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      title="Font Size [size=20]...[/size]"
      onclick={() => wrapSelection("[size=20]", "[/size]")}
      style="
        display:inline-flex; align-items:center; justify-content:center;
        width:28px; height:28px; border-radius:4px; cursor:pointer; font-size:11px;
        background:transparent; color:var(--text-secondary); user-select:none; letter-spacing:-1px;
      "
    >TT</span>

    <!-- BB Code: Underline -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      title="Underline [u]...[/u]"
      onclick={() => wrapSelection("[u]", "[/u]")}
      style="
        display:inline-flex; align-items:center; justify-content:center;
        width:28px; height:28px; border-radius:4px; cursor:pointer; font-size:13px; text-decoration:underline;
        background:transparent; color:var(--text-secondary); user-select:none;
      "
    >U</span>

    <!-- BB Code: Italic -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      title="Italic [i]...[/i]"
      onclick={() => wrapSelection("[i]", "[/i]")}
      style="
        display:inline-flex; align-items:center; justify-content:center;
        width:28px; height:28px; border-radius:4px; cursor:pointer; font-size:13px; font-style:italic;
        background:transparent; color:var(--text-secondary); user-select:none;
      "
    >I</span>

    <!-- BB Code: Color -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      title="Text Color [color=#FF0000]...[/color]"
      onclick={() => wrapSelection("[color=#FF0000]", "[/color]")}
      style="
        display:inline-flex; align-items:center; justify-content:center;
        width:28px; height:28px; border-radius:4px; cursor:pointer; font-size:13px;
        background:transparent; color:var(--text-secondary); user-select:none;
      "
    >A</span>
  </div>

  <!-- Globals picker dropdown -->
  {#if showGlobalsPicker}
    <div style="background:var(--bg-panel); border:1px solid var(--border); border-radius:4px; margin-bottom:6px; max-height:160px; overflow-y:auto;">
      <div style="padding:6px 10px; font-size:10px; font-weight:600; color:var(--text-muted); text-transform:uppercase; letter-spacing:0.5px; border-bottom:1px solid var(--border);">
        Global Variables
      </div>
      {#if globals.length === 0}
        <div style="padding:10px 12px; font-size:12px; color:var(--text-muted); font-style:italic;">No globals defined</div>
      {:else}
        {#each globals as g}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            onclick={() => insertGlobal(g.name)}
            style="display:flex; align-items:center; gap:8px; padding:6px 12px; cursor:pointer; user-select:none;"
            onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = "var(--bg-secondary)"}
            onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = "transparent"}
          >
            <span style="font-size:11px; color:var(--text-muted); font-family:var(--font-mono); min-width:16px; text-align:center;">
              {g.type === "color" ? "●" : g.type === "number" ? "#" : g.type === "switch" ? "⇄" : "Aa"}
            </span>
            <span style="font-size:12px; font-family:var(--font-mono); color:var(--accent); flex:1;">{g.name}</span>
            <span style="font-size:11px; color:var(--text-muted); font-family:var(--font-mono);">$gv({g.name})$</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  <!-- Favorites picker dropdown -->
  {#if showFavoritesPicker}
    <div style="background:var(--bg-panel); border:1px solid var(--border); border-radius:4px; margin-bottom:6px; max-height:160px; overflow-y:auto;">
      <div style="padding:6px 10px; font-size:10px; font-weight:600; color:var(--text-muted); text-transform:uppercase; letter-spacing:0.5px; border-bottom:1px solid var(--border);">
        Fave Formulas
      </div>
      {#if favorites.length === 0}
        <div style="padding:10px 12px; font-size:12px; color:var(--text-muted); font-style:italic;">No favorites yet. Star formulas below to save them.</div>
      {:else}
        {#each favorites as fav}
          <div style="display:flex; align-items:center; gap:6px; padding:5px 12px; user-select:none;">
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              onclick={() => insertFormula(fav.formula)}
              style="flex:1; cursor:pointer; overflow:hidden;"
              onmouseenter={(e) => (e.currentTarget as HTMLElement).style.opacity = "0.7"}
              onmouseleave={(e) => (e.currentTarget as HTMLElement).style.opacity = "1"}
            >
              <div style="font-size:12px; color:var(--text-primary);">{fav.name}</div>
              <div style="font-size:10px; color:var(--text-muted); font-family:var(--font-mono); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">{fav.formula}</div>
            </div>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <span
              onclick={() => removeFavorite(fav.formula)}
              style="font-size:14px; color:var(--text-muted); cursor:pointer; flex-shrink:0; user-select:none;"
              title="Remove from favorites"
            >×</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  <!-- Examples label -->
  <div style="font-size:10px; font-weight:600; color:var(--text-secondary); text-transform:uppercase; letter-spacing:0.5px; margin-bottom:5px;">
    Examples <span style="font-weight:400; text-transform:none; letter-spacing:0; color:var(--text-muted); font-size:9px;">(click appends)</span>
  </div>

  <!-- Expanded category formula list -->
  {#if expandedCat}
    <div style="background:var(--bg-panel); border:1px solid var(--border); border-radius:4px; margin-bottom:6px; overflow:hidden;">
      <div style="display:flex; align-items:center; justify-content:space-between; padding:6px 10px; border-bottom:1px solid var(--border); background:var(--bg-secondary);">
        <span style="font-size:12px; font-weight:600; color:var(--text-primary);">
          {expandedCat.icon} {expandedCat.name}
        </span>
        <div style="display:flex; align-items:center; gap:4px;">
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span
            onclick={() => expandedCategory = null}
            style="font-size:14px; color:var(--text-muted); cursor:pointer; padding:2px 6px; user-select:none;"
            title="Close"
          >×</span>
        </div>
      </div>
      <div style="max-height:200px; overflow-y:auto;">
        {#each expandedCat.options as opt}
          <div style="display:flex; align-items:center; gap:6px; padding:6px 10px; border-bottom:1px solid var(--border); user-select:none;">
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              onclick={() => insertFormula(opt.formula)}
              style="flex:1; cursor:pointer; overflow:hidden;"
              onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = "var(--bg-secondary)"}
              onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = "transparent"}
            >
              <div style="font-size:12px; color:var(--text-primary);">{opt.name}</div>
              <div style="font-size:10px; color:var(--text-muted); font-family:var(--font-mono); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">{opt.formula}</div>
            </div>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <span
              onclick={() => { isFavorite(opt.formula) ? removeFavorite(opt.formula) : addFavorite(opt.name, opt.formula); }}
              style="font-size:14px; cursor:pointer; flex-shrink:0; user-select:none; color:{isFavorite(opt.formula) ? 'var(--accent)' : 'var(--text-muted)'};"
              title={isFavorite(opt.formula) ? "Remove from favorites" : "Add to favorites"}
            >★</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Category cards grid -->
  <div style="display:grid; grid-template-columns:1fr 1fr; gap:5px;">
    {#each formulaCategories as cat}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        onclick={() => toggleCategory(cat.id)}
        style="
          background:{expandedCategory === cat.id ? 'var(--accent-dim)' : 'var(--bg-secondary)'};
          border:1px solid {expandedCategory === cat.id ? 'var(--accent)' : 'var(--border)'};
          border-radius:4px; padding:8px 10px; cursor:pointer; user-select:none;
          transition:background 0.1s, border-color 0.1s;
        "
        onmouseenter={(e) => { if (expandedCategory !== cat.id) (e.currentTarget as HTMLElement).style.background = "var(--bg-panel)"; }}
        onmouseleave={(e) => { if (expandedCategory !== cat.id) (e.currentTarget as HTMLElement).style.background = "var(--bg-secondary)"; }}
      >
        <div style="display:flex; align-items:center; gap:6px; margin-bottom:2px;">
          <span style="font-size:13px; flex-shrink:0;">{cat.icon}</span>
          <span style="font-size:12px; font-weight:600; color:var(--text-primary); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">{cat.name}</span>
        </div>
        <div style="font-size:10px; color:var(--text-muted); line-height:1.3; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">{cat.desc}</div>
      </div>
    {/each}
  </div>

</div>
