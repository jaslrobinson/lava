# KustomLinux Implementation Plan

## Current Architecture Summary

**Frontend (Svelte 5):**
- `App.svelte` composes: `Toolbar`, `LayerTree` (left panel with tabs), `CanvasRenderer`, `PropertyPanel`, `FormulaBar`
- `LeftPanel.svelte` exists but `App.svelte` currently uses the older `LayerTree.svelte` directly (which has inline tabs for Layers/Globals/Shortcuts/BG)
- `GlobalsPanel.svelte` exists as a standalone component with full editing UI (expand/collapse, type-specific inputs) but is only used in `LeftPanel.svelte`, not the active layout
- `project.svelte.ts` manages state with `$state` runes; exposes `reorderLayers()` (top-level only)
- `renderer.ts` renders layers on a 2D canvas with formula resolution via `resolve()` calls
- `formula/service.ts` batches `$...$` formula evaluation through the Rust backend every 1 second

**Backend (Rust/Tauri v2):**
- `kustom-formula` crate: full parser + evaluator for KLWP formulas (`$gv(...)$`, `$df(...)$`, `$if(...)$`, `$tc(...)$`, `$mu(...)$`, `$bi(...)$`, etc.)
- Provider system: `DateTimeProvider`, `BatteryProvider`, `SysInfoProvider`, `ResourceMonitorProvider`, `MusicProvider`, `NetworkProvider`, `TrafficProvider` -- all poll data and feed into formula context
- `klwp_import.rs`: imports `.klwp` ZIP files, extracts `preset.json`, converts to `Project` with formula strings preserved as `$...$`, resolves asset paths, extracts assets to disk
- `commands/formula.rs`: `evaluate_formula` Tauri command passes globals + provider data to `kustom_formula::evaluate()`

**Key observation:** The formula engine is already functional end-to-end. The frontend `service.ts` calls the backend `evaluate_formula` command, which uses the `kustom-formula` crate. Globals are passed from the project state. The main gap is that globals values are passed as `String(g.value)` but `g.value` can be a number/boolean from imported KLWP projects -- the serialization may not match what the formula engine expects.

---

## Feature 1: KLWP Formula Evaluation

### Understanding

Formula evaluation already works end-to-end: `service.ts` detects `$...$` patterns, calls the Rust `evaluate_formula` command which uses the `kustom-formula` crate with globals and provider data. However, there are gaps:

1. **Globals passing is incomplete**: `CanvasRenderer.svelte` line 31 does `globals[g.name] = String(g.value)` but `g.value` is typed as `string | number | boolean` in the TS types. For imported KLWP projects, the Rust `GlobalVarValue` enum serializes as `{"String": "..."}` or `{"Number": 42.0}` -- the frontend `String()` coercion may produce `"[object Object]"` if deserialization is wrong, or may work if serde `untagged` produces raw values. Need to verify.

2. **Formula resolution in renderer is synchronous**: `renderer.ts` calls `resolveFormula()` which returns cached values or `"..."` for new formulas. The 1-second polling loop picks up pending formulas. This means first render shows `"..."` until the batch completes. This is acceptable but could be improved.

3. **Non-text properties with formulas**: Properties like `x`, `y`, `width`, `height`, `opacity` can contain formula strings (imported as `NumberOrString::String("$...$")`). The renderer does `Number(props.x) || 0` which returns `NaN` then `0` for formula strings. These numeric formula properties are never resolved through the formula service.

4. **Color formulas**: `resolve()` in `renderer.ts` is only called for `color` and `text` properties. Other formula-bearing properties (position, size, opacity, fontSize, visible) are not passed through `resolve()`.

### Tasks

1. **Fix globals serialization in CanvasRenderer** - `src/lib/canvas/CanvasRenderer.svelte` - risk: low
   - Ensure `String(g.value)` correctly converts all `GlobalVarValue` variants
   - The Rust `GlobalVarValue` uses `#[serde(untagged)]` so it deserializes as raw JS values (string/number/boolean). `String()` should work, but verify with imported projects.

2. **Resolve formula strings in numeric properties** - `src/lib/canvas/renderer.ts` - risk: medium
   - Create a `resolveNumber(value, fallback)` helper that calls `resolve()` then parses to number
   - Apply to: `x`, `y`, `width`, `height`, `rotation`, `opacity`, `fontSize`, `strokeWidth`, `cornerRadius`, `min`, `max`, `value` (progress)
   - Apply in `renderLayer()`, `renderText()`, `renderShape()`, `renderProgress()`, `renderImage()`

3. **Resolve formula strings in color properties** - `src/lib/canvas/renderer.ts` - risk: low
   - Ensure `fill`, `stroke`, `trackColor`, `tint` all go through `resolve()`
   - Some already do (e.g., `color` in `renderText`), but others use raw `props.fill`

4. **Resolve formula strings in visibility** - `src/lib/canvas/renderer.ts` - risk: low
   - `isLayerVisible()` already calls `resolve()` for string visibility values. Verify it handles `BoolOrString` correctly from imported projects.

5. **Invalidate formula cache on globals change** - `src/lib/formula/service.ts` - risk: low
   - When a global variable value changes, formulas referencing `$gv(...)$` should be re-evaluated
   - Add a `invalidateGlobalsFormulas()` function that queues all cached `gv(` formulas for re-evaluation

### Dependencies
- None (this is the foundation for features 2 and 5)

### Risks
- **Performance**: Resolving many formula properties per frame could be slow if cache misses cause async batches. Mitigation: the cache ensures only first render is slow; subsequent frames use cached values.
- **Type coercion**: `Number(resolvedFormulaString)` could fail if formula returns non-numeric text. Mitigation: use fallback values.

---

## Feature 2: Globals Editing

### Understanding

`GlobalsPanel.svelte` already exists with a full editing UI: expandable rows, type-specific inputs (text, number, color picker, switch toggle, list with options). It is used inside `LeftPanel.svelte`. However, `App.svelte` uses the older `LayerTree.svelte` which has its own simpler inline globals tab. The architecture has two globals UIs in conflict.

Key issues:
1. `App.svelte` uses `LayerTree.svelte` directly, which has a basic globals tab (read-only display + add/remove)
2. `GlobalsPanel.svelte` has the richer editing UI but is only wired through `LeftPanel.svelte`, which is not used in `App.svelte`
3. `updateGlobal()` in `GlobalsPanel.svelte` mutates globals directly via `(g as any)[field] = value` which may not trigger Svelte reactivity properly since `project.globals` is a `$state` array
4. Renaming a global (changing `g.name`) breaks references in formulas like `$gv(oldname)$`
5. Formula cache needs invalidation when globals change (ties into Feature 1 task 5)

### Tasks

1. **Switch App.svelte to use LeftPanel instead of LayerTree** - `src/App.svelte` - risk: low
   - Replace `<LayerTree />` with `<LeftPanel />` in the main layout
   - This brings in the proper tabbed layout with `GlobalsPanel`, `ShortcutsPanel`, `BackgroundPanel`
   - Verify the LayerTree tab inside LeftPanel still works correctly

2. **Fix globals reactivity in GlobalsPanel** - `src/lib/editor/GlobalsPanel.svelte`, `src/lib/stores/project.svelte.ts` - risk: medium
   - `updateGlobal()` currently does `(g as any)[field] = value` which mutates in-place
   - Need to create a proper `updateGlobal(name, field, value)` function in `project.svelte.ts` that creates a new globals array (immutable update pattern for `$state`)
   - Export it and use it in `GlobalsPanel.svelte`

3. **Add globals store functions** - `src/lib/stores/project.svelte.ts` - risk: low
   - Add `addGlobal(type: GlobalVarType)`: creates a new global with default value
   - Add `removeGlobal(name: string)`: removes by name
   - Add `updateGlobal(name: string, field: string, value: any)`: immutable update
   - Move duplicate logic out of `GlobalsPanel.svelte` and `LayerTree.svelte`

4. **Wire globals changes to formula cache invalidation** - `src/lib/formula/service.ts` - risk: low
   - When `updateGlobal` is called, call `invalidateGlobalsFormulas()` from Feature 1
   - This ensures `$gv(varname)$` formulas re-evaluate with new values

5. **Handle global rename with formula reference update** - `src/lib/stores/project.svelte.ts` - risk: medium
   - When a global is renamed from `oldName` to `newName`, walk all layer properties and replace `$gv(oldName)$` with `$gv(newName)$`
   - Recursive traversal of layer tree needed
   - Also invalidate formula cache for affected formulas

### Dependencies
- Feature 1 (task 5: formula cache invalidation)

### Risks
- **Reactivity**: Svelte 5 `$state` requires immutable updates for arrays/objects. Direct mutation won't trigger re-renders. Mitigation: use spread/map patterns for all updates.
- **Rename cascading**: Renaming a global used in many formulas is error-prone. Mitigation: only replace exact `$gv(name)$` patterns, not substring matches.

---

## Feature 3: Layer Drag Reordering

### Understanding

`project.svelte.ts` has `reorderLayers(fromIndex, toIndex)` but it only works for top-level layers. There is no drag-and-drop UI in `LayerTree.svelte`. The layer tree renders nested layers recursively via a Svelte snippet `layerRow()`.

Key challenges:
1. Need drag-and-drop within a recursive tree (nested layers inside groups/stacks/overlaps)
2. Must support reordering within the same parent AND reparenting (moving a layer into/out of a group)
3. The visual list is rendered in reverse order (`.toReversed()`) to match KLWP's bottom-to-top z-order

### Tasks

1. **Add drag handles and HTML5 drag events to layerRow** - `src/lib/editor/LayerTree.svelte` - risk: medium
   - Add `draggable="true"` to layer items
   - Implement `ondragstart`, `ondragover`, `ondrop`, `ondragend` handlers
   - Track drag source (layer ID) and drop target (layer ID + position: before/after/inside)
   - Show visual drop indicator (line above/below, or highlight for "drop into group")

2. **Implement tree-aware reorder in project store** - `src/lib/stores/project.svelte.ts` - risk: high
   - Replace `reorderLayers()` with `moveLayer(layerId: string, targetParentId: string | null, targetIndex: number)`
   - Remove layer from its current parent, insert at target position in target parent
   - Requires recursive find-and-remove + find-parent-and-insert
   - Handle edge cases: can't drop a group into itself (circular reference)

3. **Reverse-order drag index mapping** - `src/lib/editor/LayerTree.svelte` - risk: medium
   - Since the tree is rendered with `.toReversed()`, drag indices need to be mapped back to the actual array indices
   - The drop indicator position (before/after in visual order) maps to the opposite in data order

4. **Visual feedback during drag** - `src/lib/editor/LayerTree.svelte` - risk: low
   - CSS for drag-over states: highlight line between items, highlight group containers
   - Ghost/preview of the dragged item
   - Disable pointer events on non-droppable targets

### Dependencies
- None (independent of other features, but should be done after Feature 2 so LeftPanel is in use)

### Risks
- **Circular drops**: Dropping a parent group into its own child. Mitigation: validate drop target is not a descendant of the drag source.
- **Reversed rendering**: The `.toReversed()` rendering makes index calculations tricky. Mitigation: use layer IDs rather than indices for all drag operations.
- **Nested depth**: Deep nesting makes drop target detection complex. Mitigation: use indentation-based zones (left side = reparent up, center = reorder, right side = drop into).

---

## Feature 4: Property Panel Improvements

### Understanding

`PropertyPanel.svelte` shows type-specific property editors for the selected layer. It handles text, shape, image, and progress types. Key gaps:

1. **No fonticon property editing**: FontIcon layers have no property section
2. **No group/stack/overlap property editing**: No orientation, spacing editors
3. **No animation editing**: Animation data exists in types but no UI
4. **Formula-aware inputs**: Numeric inputs use `type="number"` which can't display formula strings like `$gv(size)$`. When a property contains a formula, the input should show the formula text and allow editing it as text.
5. **No shadow editing UI**: Shadow properties exist but no editor
6. **Missing properties**: `anchor`, `scaleX`, `scaleY`, `visible`, `lineSpacing`, `maxLines` have no editors

### Tasks

1. **Add formula-aware input component** - `src/lib/editor/FormulaInput.svelte` (new file) - risk: medium
   - A smart input that detects whether the value is a number or formula string
   - Shows `type="number"` for pure numbers, switches to `type="text"` when value contains `$`
   - Has an "fx" toggle button to switch between number mode and formula mode
   - Displays the resolved formula value as a tooltip or secondary label

2. **Add FontIcon property section** - `src/lib/editor/PropertyPanel.svelte` - risk: low
   - Add section for `layer.type === "fonticon"` with: iconSet, glyphCode, color, fontSize

3. **Add Group/Stack/Overlap property section** - `src/lib/editor/PropertyPanel.svelte` - risk: low
   - Add section for container types with: orientation (stack only), spacing
   - Show child count info

4. **Add Shadow editing sub-section** - `src/lib/editor/PropertyPanel.svelte` - risk: low
   - Add collapsible shadow editor to text and shape sections
   - Fields: shadow color, dx, dy, blur radius
   - Toggle to enable/disable shadow

5. **Add missing transform properties** - `src/lib/editor/PropertyPanel.svelte` - risk: low
   - Add `anchor` dropdown to Transform section (all 9 anchor points)
   - Add `scaleX`, `scaleY` numeric inputs
   - Add `visible` toggle/formula input

6. **Replace numeric inputs with FormulaInput** - `src/lib/editor/PropertyPanel.svelte` - risk: medium
   - Replace `<input type="number">` with `<FormulaInput>` for all properties that can contain formulas
   - This allows viewing/editing formulas in imported KLWP presets

### Dependencies
- Feature 1 (formula resolution is needed for FormulaInput to show resolved values)

### Risks
- **UX complexity**: Formula-aware inputs add cognitive load. Mitigation: default to simple number mode, only show formula mode when value contains `$` or user clicks "fx".
- **Type coercion**: Switching between number and formula modes needs careful handling. Mitigation: store as string when formula, number when pure numeric.

---

## Feature 5: Asset/Image Rendering

### Understanding

Image rendering has a path resolution issue. When a KLWP preset is imported:
1. `klwp_import.rs` extracts assets to `<klwp_dir>/<stem>_assets/assets/...`
2. Image paths are resolved via `resolve_asset_path()` which produces absolute paths like `/home/user/Downloads/Fade_black_assets/assets/bitmaps/IMG123.jpg`
3. The frontend `renderer.ts` uses `convertFileSrc(src)` to convert local paths to `asset://` protocol URLs
4. The CSP in `tauri.conf.json` allows `asset:` protocol with scope `["**"]`

Potential issues:
1. **Path format**: The Rust importer produces Unix absolute paths. `convertFileSrc()` should handle these correctly for the asset protocol.
2. **Missing extension**: `resolve_asset_path()` tries common extensions but may miss some. KLWP bitmap references often omit file extensions.
3. **kfile:// paths not resolved**: If a formula evaluates to a `kfile://` path at runtime, the renderer won't know how to resolve it.
4. **Image cache never clears**: `imageFailedSet` never gets cleared, so if an image fails once (e.g., during import before extraction completes), it stays failed forever.

### Tasks

1. **Add asset path resolution Tauri command** - `src-tauri/src/commands/project.rs` - risk: low
   - Add `resolve_asset_path` command that takes a relative or kfile:// path and the project's asset directory, returns the absolute resolved path
   - Reuse the existing `resolve_asset_path()` function from `klwp_import.rs` (make it `pub`)

2. **Store asset base directory in project state** - `src/lib/stores/project.svelte.ts`, `src/lib/types/project.ts` - risk: low
   - After KLWP import, store the asset directory path in the project or as a separate state
   - This is needed so the renderer knows where to look up relative asset paths
   - Add `assetDir?: string` to the `Project` type (frontend only, not serialized to Rust)

3. **Fix image path resolution in renderer** - `src/lib/canvas/renderer.ts` - risk: medium
   - In `getCachedImage()`, handle relative paths by prepending the asset base directory
   - Handle `kfile://` paths by stripping the prefix and resolving against asset dir
   - Clear `imageFailedSet` entries periodically or when project changes

4. **Fix image path resolution in PropertyPanel** - `src/lib/editor/PropertyPanel.svelte` - risk: low
   - `getImageSrc()` should also handle the asset directory resolution
   - Show the resolved path in the preview, not the raw kfile:// path

5. **Handle formula-resolved image paths** - `src/lib/canvas/renderer.ts` - risk: medium
   - In `renderImage()`, pass `src` through `resolve()` before looking up the image
   - If the resolved path is a kfile:// or relative path, resolve it against asset dir
   - Currently, `getCachedImage()` skips unresolved formulas (lines 22-23), which is correct

### Dependencies
- Feature 1 (formula resolution needed for dynamic image paths)

### Risks
- **Asset protocol security**: Allowing `**` scope is permissive. For production, scope should be limited to specific directories. Mitigation: acceptable for now, tighten later.
- **Large images**: No size limits on loaded images. Mitigation: could add thumbnail generation later.

---

## Feature 6: Apply as Wallpaper

### Understanding

This is the most complex feature. The goal is to take the rendered canvas and display it as a live desktop wallpaper on Arch Linux. This requires different approaches for X11 and Wayland.

**Approach: Dual-backend wallpaper mode**

The app needs a "wallpaper mode" that transforms the Tauri window into a desktop-level window that sits below all other windows and spans the entire screen (or a selected monitor).

### X11 Approach

Set the window's `_NET_WM_WINDOW_TYPE` to `_NET_WM_WINDOW_TYPE_DESKTOP` using the X11 protocol. This tells the window manager to treat the window as a desktop surface. Most X11 window managers (KWin, Openbox, i3, etc.) respect this hint.

Implementation:
- Access the GTK window via Tauri's `window.gtk_window()` method
- Get the X11 window ID from the GDK X11 display
- Use `x11rb` or raw `xcb` to set the `_NET_WM_WINDOW_TYPE` atom on the window
- Set the window to fullscreen/undecorated and position it at (0,0)
- Set `_NET_WM_STATE_BELOW` to keep it below other windows
- Disable window decorations and taskbar entry

### Wayland Approach

Use the `wlr-layer-shell` protocol via `gtk-layer-shell` (or `gtk4-layer-shell`). This protocol is supported by wlroots-based compositors (Hyprland, Sway, river, etc.) which are common on Arch Linux.

Implementation:
- Use the `gtk-layer-shell` crate (GTK3 bindings, since Tauri v2 uses GTK3 via webkit2gtk)
- In wallpaper mode, call `gtk_layer_shell::init_for_window()` on the GTK window
- Set layer to `Layer::Background`
- Set anchors to all edges (stretch to fill screen)
- Set exclusive zone to -1 (behind panels)
- Set namespace to "kustomlinux-wallpaper"

### Fallback: Static Wallpaper Export

For unsupported compositors (GNOME Wayland doesn't support layer-shell), provide a "screenshot and set" fallback:
- Render the canvas to a PNG image
- Set it as the desktop wallpaper using `gsettings` (GNOME), `feh` (X11), or `swaybg` (Wayland)
- Optionally re-render periodically to update time-based formulas

### Tasks

1. **Add wallpaper mode Tauri command and state** - `src-tauri/src/commands/wallpaper.rs` (new), `src-tauri/src/lib.rs` - risk: high
   - Add `start_wallpaper_mode` and `stop_wallpaper_mode` Tauri commands
   - Track wallpaper mode state (active/inactive, which monitor)
   - Detect display server: check `WAYLAND_DISPLAY` env var (Wayland) vs `DISPLAY` (X11)

2. **Implement X11 desktop window type** - `src-tauri/src/wallpaper/x11.rs` (new) - risk: high
   - Add `x11rb` crate dependency
   - Get the X11 window ID from the GTK window via GDK
   - Set `_NET_WM_WINDOW_TYPE` to `_NET_WM_WINDOW_TYPE_DESKTOP`
   - Set `_NET_WM_STATE_BELOW`, `_NET_WM_STATE_SKIP_TASKBAR`, `_NET_WM_STATE_SKIP_PAGER`
   - Remove window decorations
   - Set window geometry to cover the full screen

3. **Implement Wayland layer-shell wallpaper** - `src-tauri/src/wallpaper/wayland.rs` (new) - risk: high
   - Add `gtk-layer-shell` crate dependency
   - Access GTK window via `window.gtk_window()`
   - Initialize layer shell on the window with `Layer::Background`
   - Anchor to all edges, set exclusive zone to -1
   - Handle multi-monitor: enumerate outputs, let user pick target monitor

4. **Implement static wallpaper export fallback** - `src-tauri/src/commands/wallpaper.rs` - risk: medium
   - Add `export_wallpaper_image` command that renders current canvas to PNG
   - Detect desktop environment and use appropriate tool:
     - GNOME: `gsettings set org.gnome.desktop.background picture-uri file:///path/to/img.png`
     - KDE: `qdbus org.kde.plasmashell ...` or `plasma-apply-wallpaperimage`
     - Sway/wlroots: write a swaybg config
     - Generic X11: `feh --bg-fill /path/to/img.png`
   - Optionally set up a timer to re-export periodically for time-based wallpapers

5. **Add wallpaper UI controls** - `src/lib/editor/Toolbar.svelte` - risk: low
   - Add "Apply as Wallpaper" button to toolbar
   - Show dropdown with options: "Live Wallpaper", "Export Static Image"
   - Show wallpaper status indicator (active/inactive)
   - Add "Stop Wallpaper" button when active

6. **Handle editor/wallpaper mode transition** - `src/App.svelte`, `src-tauri/src/lib.rs` - risk: medium
   - In wallpaper mode: hide editor UI (toolbar, panels), show only the canvas fullscreen
   - The canvas should render at native monitor resolution
   - Keep the formula evaluation loop running
   - Provide a way to return to editor mode (keyboard shortcut, system tray, or second window)

7. **Add system tray for wallpaper mode** - `src-tauri/src/lib.rs` - risk: medium
   - When in wallpaper mode, show a system tray icon
   - Tray menu: "Open Editor", "Stop Wallpaper", "Quit"
   - This allows the user to control the wallpaper without a visible window

### Dependencies
- Features 1, 5 (formulas and images must render correctly before applying as wallpaper)
- Feature 2 (globals editing is nice-to-have but not blocking)

### Risks
- **Compositor compatibility**: Layer-shell only works on wlroots compositors. GNOME, KDE Wayland won't support live wallpaper mode. Mitigation: detect compositor and fall back to static export; document supported compositors.
- **GTK version mismatch**: Tauri v2 uses GTK3 (via webkit2gtk-4.1). The `gtk-layer-shell` crate (not gtk4) is correct for this. Verify API compatibility.
- **Window manager override**: Some WMs may ignore `_NET_WM_WINDOW_TYPE_DESKTOP`. Mitigation: test on common Arch WMs (i3, Openbox, KWin/X11, Hyprland, Sway).
- **Performance**: Running a full Tauri app + WebKitGTK as a wallpaper uses more resources than a lightweight renderer. Mitigation: reduce render frequency when no time-based formulas are active; consider rendering to a shared memory buffer.
- **Multi-monitor**: Different monitors may have different resolutions. Mitigation: start with single-monitor, expand later.

---

## Dependency Graph

```
Feature 1: Formula Evaluation  (foundation - no dependencies)
    |
    +---> Feature 2: Globals Editing (depends on F1 for cache invalidation)
    |
    +---> Feature 5: Asset/Image Rendering (depends on F1 for formula-resolved paths)
    |
    +---> Feature 4: Property Panel (depends on F1 for FormulaInput resolved values)

Feature 3: Layer Drag Reorder (independent, but do after F2 so LeftPanel is active)

Feature 6: Apply as Wallpaper (depends on F1, F5 for correct rendering)
```

**Recommended execution order:**
1. Feature 1 (Formula Evaluation) -- unlocks everything else
2. Feature 2 (Globals Editing) -- quick win, uses F1
3. Feature 5 (Asset/Image Rendering) -- needed before wallpaper mode
4. Feature 4 (Property Panel) -- improves editing experience
5. Feature 3 (Layer Drag Reorder) -- independent UX improvement
6. Feature 6 (Apply as Wallpaper) -- capstone feature, most complex

---

## File Change Summary

### New Files
| File | Feature | Description |
|------|---------|-------------|
| `src/lib/editor/FormulaInput.svelte` | F4 | Smart formula/number input component |
| `src-tauri/src/commands/wallpaper.rs` | F6 | Wallpaper mode Tauri commands |
| `src-tauri/src/wallpaper/mod.rs` | F6 | Wallpaper module with X11/Wayland detection |
| `src-tauri/src/wallpaper/x11.rs` | F6 | X11 desktop window type implementation |
| `src-tauri/src/wallpaper/wayland.rs` | F6 | Wayland layer-shell wallpaper |

### Modified Files
| File | Features | Changes |
|------|----------|---------|
| `src/lib/canvas/renderer.ts` | F1, F5 | Formula resolution for all properties, image path fixes |
| `src/lib/formula/service.ts` | F1, F2 | Cache invalidation, globals-aware re-evaluation |
| `src/lib/canvas/CanvasRenderer.svelte` | F1 | Globals passing fix |
| `src/lib/stores/project.svelte.ts` | F2, F3 | Globals CRUD, tree-aware moveLayer, asset dir state |
| `src/lib/types/project.ts` | F5 | Add `assetDir` field |
| `src/lib/editor/GlobalsPanel.svelte` | F2 | Use store functions, fix reactivity |
| `src/lib/editor/LayerTree.svelte` | F3 | Drag-and-drop events and visual feedback |
| `src/lib/editor/PropertyPanel.svelte` | F4 | FontIcon/Group sections, FormulaInput, shadow editor |
| `src/lib/editor/Toolbar.svelte` | F6 | Wallpaper mode button |
| `src/App.svelte` | F2, F6 | Switch to LeftPanel, wallpaper mode UI |
| `src-tauri/src/lib.rs` | F6 | Register wallpaper commands, system tray |
| `src-tauri/src/commands/mod.rs` | F5, F6 | Register new command modules |
| `src-tauri/src/commands/project.rs` | F5 | Asset path resolution command |
| `src-tauri/src/klwp_import.rs` | F5 | Make `resolve_asset_path` pub |
| `src-tauri/Cargo.toml` | F6 | Add x11rb, gtk-layer-shell dependencies |
| `src-tauri/tauri.conf.json` | F6 | System tray config, window options |

### New Cargo Dependencies (Feature 6)
| Crate | Purpose |
|-------|---------|
| `x11rb` | X11 protocol for setting window type |
| `gtk-layer-shell` | Wayland layer-shell for GTK3 windows |

---

## Questions / Decisions Needed

1. **Multi-monitor support for wallpaper mode**: Should we support different wallpapers per monitor from the start, or single-monitor first?
   - Recommendation: Single-monitor first, expand later.

2. **Wallpaper auto-start**: Should the app support launching directly into wallpaper mode (e.g., `kustomlinux --wallpaper project.klwp`)? This would be needed for system startup.
   - Recommendation: Yes, add CLI args for wallpaper mode.

3. **LeftPanel migration**: `App.svelte` currently uses the old `LayerTree.svelte` directly. Should we migrate to `LeftPanel.svelte` (which includes the newer `GlobalsPanel.svelte`) or update the old `LayerTree.svelte`?
   - Recommendation: Migrate to `LeftPanel.svelte`. The `GlobalsPanel.svelte` already has the richer editing UI we need.

4. **Layer drag reorder scope**: Should drag-and-drop support reparenting (moving into/out of groups) or just sibling reorder?
   - Recommendation: Start with sibling reorder within the same parent, add reparenting as a follow-up.
