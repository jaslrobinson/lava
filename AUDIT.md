# Mismatch Audit: Data Model / Renderer / Property Editor

Audited 2026-03-08. All file paths are absolute.

---

## 1. Layer Type Coverage

All 8 `LayerType` values are handled by all three surfaces. No type is missing.

| Type | Renderer function | PropertyPanel section | Toolbar button |
|---|---|---|---|
| text | `renderText()` L313 | L90-113 | yes |
| shape | `renderShape()` L346 | L115-137 | yes |
| image | `renderImage()` L446 | L139-164 | yes |
| progress | `renderProgress()` L514 | L166-188 | yes |
| fonticon | `renderFontIcon()` L557 | L223-237 | yes |
| group | `renderOverlap()` L583 | L210-221 | yes |
| stack | `renderStack()` L599 | L190-208 | yes |
| overlap | `renderOverlap()` L583 | L210-221 | yes |

All line numbers refer to `/home/andason/Projects/klwp/src/lib/canvas/renderer.ts` (renderer) and `/home/andason/Projects/klwp/src/lib/editor/PropertyPanel.svelte` (panel).

---

## 2. Property Coverage Gaps

Properties present in the TypeScript model with no PropertyPanel UI.

### All layer types (Transform section)

| Property | Defined at | Renderer uses it | Panel shows it |
|---|---|---|---|
| `anchor` | `project.ts:49` | YES — `anchorPosition()` at `renderer.ts:264` | NO |
| `scaleX` / `scaleY` | `project.ts:46-47` | NO (see section 3) | NO |
| `properties.visible` (formula) | `project.ts:50` | YES — `isLayerVisible()` at `renderer.ts:186-195` | NO — only `layer.visible` bool is toggled via LayerTree eye icon |

### Text layers

| Property | Defined at | Renderer uses it | Panel shows it |
|---|---|---|---|
| `maxLines` | `project.ts:58` | NO | NO |
| `lineSpacing` | `project.ts:59` | NO | NO |
| `shadow` | `project.ts:60` | YES — `renderer.ts:326-332` | NO |

### Shape layers

| Property | Defined at | Renderer uses it | Panel shows it |
|---|---|---|---|
| `shadow` | `project.ts:60` | NO (see section 3) | NO |

### Progress layers

| Property | Defined at | Renderer uses it | Panel shows it |
|---|---|---|---|
| `strokeWidth` | `project.ts:67` | YES — `renderer.ts:521` | NO — `createLayer()` sets it to `6` at `project.ts:170` with no way to change it |

---

## 3. Formula Integration Gaps

Properties that hold `number | string` (formula-capable) but are not resolved by the renderer, or properties that are stored but silently ignored.

### `props.scaleX` / `props.scaleY` — completely ignored by renderer

**File:** `/home/andason/Projects/klwp/src/lib/canvas/renderer.ts:278-282`

```ts
if (rotation || deltas.scaleX !== 1 || deltas.scaleY !== 1) {
  ctx.translate(cx, cy);
  if (rotation) ctx.rotate((rotation * Math.PI) / 180);
  if (deltas.scaleX !== 1 || deltas.scaleY !== 1) ctx.scale(deltas.scaleX, deltas.scaleY);
  ctx.translate(-cx, -cy);
}
```

Only animation-engine deltas (`deltas.scaleX`, `deltas.scaleY`) are applied. `props.scaleX` and `props.scaleY` are never read. There is no `resolveNumber(props.scaleX)` call anywhere in the codebase.

### `props.tint` on image layers — stored and shown in UI, never applied

**File:** `/home/andason/Projects/klwp/src/lib/canvas/renderer.ts:446-512`

`renderImage()` reads `scaleMode` and draws the image but contains no reference to `props.tint`. The PropertyPanel shows a color picker for tint (`PropertyPanel.svelte:161`), so the user can set it, but it has no effect on rendering.

### Shadow on shape layers — imported, never rendered

**File:** `/home/andason/Projects/klwp/src/lib/canvas/renderer.ts:346-407`

`renderShape()` sets `ctx.fillStyle` and `ctx.strokeStyle` but never sets `ctx.shadowColor`. Only `renderText()` applies shadow (`renderer.ts:326-332`). The importer calls `convert_shadow()` for shape layers at `klwp_import.rs:347`, so shape shadows from KLWP presets are carried in the data but silently dropped at render time.

### `props.lineSpacing` on text — stored and imported, never applied

**File:** `/home/andason/Projects/klwp/src/lib/canvas/renderer.ts:313-343`

`renderText()` calls `ctx.fillText(text, textX, y)` once on the whole string with no line splitting. Multi-line layout is not implemented. `lineSpacing` has no effect. The importer sets it at `klwp_import.rs:324`.

### `props.maxLines` on text — same situation as lineSpacing

The renderer does not split text or limit line count. The property is imported (`klwp_import.rs:323`) and ignored.

### `progress` style "circle" is functionally identical to "arc"

**File:** `/home/andason/Projects/klwp/src/lib/canvas/renderer.ts:525`

```ts
if (style === "bar") { ... } else { /* arc / circle */ }
```

The TypeScript type defines three variants (`"arc" | "bar" | "circle"`), the PropertyPanel offers all three, and the Rust `ProgressStyle` enum has all three. But the renderer treats `"circle"` the same as `"arc"`.

---

## 4. Importer → Editor Gaps

Fields set by `klwp_import.rs` that the editor cannot display or author.

| Importer field | Set at | Renderer handles | Panel shows |
|---|---|---|---|
| `shadow` on text | `klwp_import.rs:325` | YES | NO |
| `shadow` on shape | `klwp_import.rs:347` | NO (bug above) | NO |
| `lineSpacing` | `klwp_import.rs:324` | NO (bug above) | NO |
| `maxLines` | `klwp_import.rs:323` | NO (bug above) | NO |
| `properties.visible` as formula | `klwp_import.rs:477-491` | YES | NO |
| `anchor` | `klwp_import.rs:317` | YES | NO — imported anchor is permanent; user cannot change it |
| `rotation` on FontIcon (`icon_rotate_offset`) | `klwp_import.rs:386` | YES | YES (Transform applies to all types) |

The importer also hardcodes `opacity: Some(NumberOrString::Number(255.0))` at `klwp_import.rs:316` for text layers and does not read any opacity formula from the KLWP source. KLWP items with formula-driven opacity will be fully opaque after import.

---

## 5. Store Actions Without Any UI Trigger

Both functions are exported from `/home/andason/Projects/klwp/src/lib/stores/project.svelte.ts` but are never imported by any `.svelte` component.

### `addLayerToParent(parentId, type)` — L102-113

Explicitly adds a new child layer to a named parent container by ID. The Toolbar's `addLayer()` (L71-86) achieves the same result by auto-targeting the currently selected container. `addLayerToParent` has no caller and is dead code from a UI perspective.

### `reorderLayers(fromIndex, toIndex)` — L160-166

Operates only on the flat top-level `project.layers` array. The LayerTree uses `moveLayer()` for all drag-and-drop, which handles the full nested tree. `reorderLayers` has no caller and does not support nested layers, making it both unused and incomplete.

---

## Priority Summary

### Renderer bugs (data exists, silently dropped)
1. `props.scaleX` / `props.scaleY` never read — `renderer.ts:278-282`
2. `props.tint` on image never applied — `renderer.ts:446-512`
3. Shadow on shape layers never applied — `renderShape()` missing shadow code
4. `lineSpacing` / `maxLines` on text never applied — `renderText()` missing multi-line support

### Missing property UI (data exists, no way to author it)
5. `anchor` — no picker despite renderer and importer both using it
6. `strokeWidth` on progress — set by `createLayer()`, used by renderer, absent from panel
7. `shadow` on text and shape — no shadow editor (color, dx, dy, radius)
8. `properties.visible` as formula — only `layer.visible` bool is toggleable
9. `scaleX` / `scaleY` — in the type, not in panel, not in renderer
10. `maxLines`, `lineSpacing` on text — set by importer, not editable

### Dead code
11. `addLayerToParent()` — `project.svelte.ts:102`
12. `reorderLayers()` — `project.svelte.ts:160`
