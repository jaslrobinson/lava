use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::project::*;

static ITEM_COUNTER: AtomicU32 = AtomicU32::new(0);

fn next_id() -> String {
    let id = ITEM_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("klwp_{}", id)
}

fn reset_counter() {
    ITEM_COUNTER.store(0, Ordering::Relaxed);
}

/// KLWP preset info header
#[derive(Debug, Deserialize)]
struct KlwpInfo {
    #[allow(dead_code)]
    title: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

/// KLWP global variable definition
#[derive(Debug, Clone, Deserialize)]
struct KlwpGlobal {
    #[serde(rename = "type")]
    var_type: Option<String>,
    value: Option<JsonValue>,
    entries: Option<String>,
}

/// A KLWP item parsed from flexible JSON. All field access via helper methods.
#[derive(Debug)]
struct KlwpItem {
    data: serde_json::Map<String, JsonValue>,
    children: Vec<KlwpItem>,
    formulas: HashMap<String, String>,
    globals_bindings: HashMap<String, String>,
    globals_list: HashMap<String, KlwpGlobal>,
}

impl KlwpItem {
    fn from_value(val: &JsonValue) -> Option<Self> {
        let obj = val.as_object()?;
        if !obj.contains_key("internal_type") {
            return None;
        }

        let children: Vec<KlwpItem> = obj.get("viewgroup_items")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(KlwpItem::from_value).collect())
            .unwrap_or_default();

        let formulas: HashMap<String, String> = obj.get("internal_formulas")
            .and_then(|v| v.as_object())
            .map(|m| m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect())
            .unwrap_or_default();

        let globals_bindings: HashMap<String, String> = obj.get("internal_globals")
            .and_then(|v| v.as_object())
            .map(|m| m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect())
            .unwrap_or_default();

        let globals_list: HashMap<String, KlwpGlobal> = obj.get("globals_list")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        Some(KlwpItem {
            data: obj.clone(),
            children,
            formulas,
            globals_bindings,
            globals_list,
        })
    }

    fn str_field(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(|v| v.as_str())
    }

    fn f64_field(&self, key: &str) -> Option<f64> {
        self.data.get(key).and_then(|v| v.as_f64())
    }

    fn internal_type(&self) -> &str {
        self.str_field("internal_type").unwrap_or("")
    }

    fn has_field(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
}

/// Import result for a .komp komponent file
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KompImportResult {
    pub root: Layer,
    pub globals: Vec<GlobalVariable>,
    pub warnings: Vec<String>,
    pub asset_count: usize,
    pub asset_dir: String,
}

/// Convert Android #AARRGGBB to CSS #RRGGBBAA
fn convert_color(color: &str) -> String {
    let c = color.trim();
    if c.len() == 9 && c.starts_with('#') {
        let aa = &c[1..3];
        let rrggbb = &c[3..9];
        if aa == "FF" {
            format!("#{}", rrggbb)
        } else {
            format!("#{}{}", rrggbb, aa)
        }
    } else {
        c.to_string()
    }
}

/// Wrap a formula string in $...$ delimiters, avoiding double-wrapping.
/// KLWP/KOMP internal_formulas values already include $...$ delimiters.
fn wrap_formula(formula: &str) -> String {
    if formula.starts_with('$') && formula.ends_with('$') {
        formula.to_string()
    } else {
        format!("${}$", formula)
    }
}

/// Resolve a property: check formulas first, then globals binding, then static value
fn resolve_string_prop(
    key: &str,
    static_val: Option<&str>,
    formulas: &HashMap<String, String>,
    globals: &HashMap<String, String>,
) -> Option<String> {
    if let Some(formula) = formulas.get(key) {
        return Some(wrap_formula(formula));
    }
    if let Some(gvar) = globals.get(key) {
        return Some(format!("$gv({})$", gvar));
    }
    static_val.map(|s| s.to_string())
}

fn resolve_color_prop(
    key: &str,
    static_val: Option<&str>,
    formulas: &HashMap<String, String>,
    globals: &HashMap<String, String>,
) -> Option<String> {
    if let Some(formula) = formulas.get(key) {
        return Some(wrap_formula(formula));
    }
    if let Some(gvar) = globals.get(key) {
        return Some(format!("$gv({})$", gvar));
    }
    static_val.map(|s| convert_color(s))
}

fn resolve_number_prop(
    key: &str,
    static_val: Option<f64>,
    formulas: &HashMap<String, String>,
    globals: &HashMap<String, String>,
) -> Option<NumberOrString> {
    if let Some(formula) = formulas.get(key) {
        return Some(NumberOrString::String(wrap_formula(formula)));
    }
    if let Some(gvar) = globals.get(key) {
        return Some(NumberOrString::String(format!("$gv({})$", gvar)));
    }
    static_val.map(NumberOrString::Number)
}

fn map_anchor(anchor: Option<&str>) -> Option<AnchorPoint> {
    match anchor {
        Some("CENTER") => Some(AnchorPoint::Center),
        Some("TOP") => Some(AnchorPoint::TopCenter),
        Some("BOTTOM") => Some(AnchorPoint::BottomCenter),
        Some("LEFT") => Some(AnchorPoint::CenterLeft),
        Some("RIGHT") => Some(AnchorPoint::CenterRight),
        Some("TOPLEFT") => Some(AnchorPoint::TopLeft),
        Some("TOPCENTER") => Some(AnchorPoint::TopCenter),
        Some("TOPRIGHT") => Some(AnchorPoint::TopRight),
        Some("CENTERLEFT") => Some(AnchorPoint::CenterLeft),
        Some("CENTERRIGHT") => Some(AnchorPoint::CenterRight),
        Some("BOTTOMLEFT") => Some(AnchorPoint::BottomLeft),
        Some("BOTTOMCENTER") => Some(AnchorPoint::BottomCenter),
        Some("BOTTOMRIGHT") => Some(AnchorPoint::BottomRight),
        _ => None,
    }
}

fn map_shape_kind(shape: Option<&str>) -> Option<ShapeKind> {
    match shape {
        Some("RECT") => Some(ShapeKind::Rectangle),
        Some("CIRCLE") | Some("ELLIPSE") => Some(ShapeKind::Circle),
        Some("OVAL") => Some(ShapeKind::Oval),
        Some("TRIANGLE") => Some(ShapeKind::Triangle),
        Some("ARC") => Some(ShapeKind::Arc),
        _ => Some(ShapeKind::Rectangle),
    }
}

fn map_text_align(align: Option<&str>) -> Option<TextAlign> {
    match align {
        Some("LEFT") => Some(TextAlign::Left),
        Some("CENTER") => Some(TextAlign::Center),
        Some("RIGHT") => Some(TextAlign::Right),
        _ => None,
    }
}

/// Extract font name from kfile:// path, resolving to absolute asset path when possible
fn extract_font_name(path: &str, assets_dir: &str) -> String {
    if path.starts_with("kfile://") {
        let relative = path.replace("kfile://org.kustom.provider/", "");
        let abs_path = format!("{}/assets/{}", assets_dir, relative);
        if std::path::Path::new(&abs_path).exists() {
            return abs_path;
        }
        // Fall back to just the filename stem
        path.rsplit('/').next().unwrap_or(path)
            .trim_end_matches(".ttf")
            .trim_end_matches(".otf")
            .to_string()
    } else if path.contains('/') {
        path.rsplit('/').next().unwrap_or(path)
            .trim_end_matches(".ttf")
            .trim_end_matches(".otf")
            .to_string()
    } else {
        path.to_string()
    }
}

/// Resolve an asset path to an absolute path, trying with common extensions if the exact path doesn't exist
pub fn resolve_asset_path(path: &str, assets_dir: &str) -> String {
    let relative = if path.starts_with("kfile://") {
        // Strip kfile://org.kustom.provider/ prefix
        path.replace("kfile://org.kustom.provider/", "")
    } else if path.starts_with("assets/") {
        path.strip_prefix("assets/").unwrap_or(path).to_string()
    } else {
        return path.to_string(); // External URL or other path, leave as-is
    };

    let base = Path::new(assets_dir).join(&relative);

    // Try exact path first
    if base.exists() {
        return base.to_string_lossy().to_string();
    }

    // Try common image extensions
    for ext in &["jpg", "jpeg", "png", "webp", "gif", "bmp", "svg"] {
        let with_ext = base.with_extension(ext);
        if with_ext.exists() {
            return with_ext.to_string_lossy().to_string();
        }
    }

    // Try with extension appended (in case the filename already has a partial extension)
    let parent = base.parent().unwrap_or(Path::new(assets_dir));
    if let Some(filename) = base.file_name().and_then(|f| f.to_str()) {
        // Search directory for files starting with this name
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(filename) && name.len() > filename.len() {
                        return entry.path().to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    // Fall back to the constructed path even if it doesn't exist yet
    base.to_string_lossy().to_string()
}

/// Convert a KLWP item tree to our Layer tree
/// Uses a uniform scale factor to maintain proportions from mobile to desktop.
/// `assets_dir` is the absolute path to the extracted assets directory.
/// `parent_width` and `parent_height` are the dimensions (in scaled pixels) of the parent container.
fn convert_item(item: &KlwpItem, warnings: &mut Vec<String>, scale: f64, assets_dir: &str, parent_width: f64, parent_height: f64) -> Option<Layer> {
    let formulas = &item.formulas;
    let globals = &item.globals_bindings;

    let (layer_type, properties) = match item.internal_type() {
        "TextModule" | "CurvedTextModule" => {
            let text = resolve_string_prop(
                "text_expression",
                item.str_field("text_expression"),
                formulas, globals,
            ).unwrap_or_default();

            let font_family = item.str_field("text_family").map(|f| extract_font_name(f, assets_dir));

            let props = LayerProperties {
                x: NumberOrString::Number(item.f64_field("position_offset_x").unwrap_or(0.0) * scale),
                y: NumberOrString::Number(item.f64_field("position_offset_y").unwrap_or(0.0) * scale),
                width: NumberOrString::Number(item.f64_field("text_width").unwrap_or(400.0) * scale),
                height: NumberOrString::Number(item.f64_field("text_height").unwrap_or(50.0) * scale),
                rotation: item.f64_field("text_rotate_offset")
                    .or(item.f64_field("config_rotate_offset"))
                    .map(NumberOrString::Number),
                opacity: Some(NumberOrString::Number(255.0)),
                anchor: map_anchor(item.str_field("position_anchor")),
                text: Some(text),
                font_size: resolve_number_prop("text_size", item.f64_field("text_size"), formulas, globals),
                font_family,
                color: resolve_color_prop("paint_color", item.str_field("paint_color"), formulas, globals),
                text_align: map_text_align(item.str_field("text_align")),
                max_lines: item.f64_field("text_lines").map(|l| l as u32),
                line_spacing: item.f64_field("text_spacing"),
                shadow: convert_shadow(item, scale),
                ..Default::default()
            };
            (LayerType::Text, props)
        }

        "ShapeModule" => {
            let sw = item.f64_field("shape_width").unwrap_or(100.0);
            // For circles/ellipses without explicit height, use width (square bounding box)
            let sh = item.f64_field("shape_height").unwrap_or(sw);
            let props = LayerProperties {
                x: NumberOrString::Number(item.f64_field("position_offset_x").unwrap_or(0.0) * scale),
                y: NumberOrString::Number(item.f64_field("position_offset_y").unwrap_or(0.0) * scale),
                width: NumberOrString::Number(sw * scale),
                height: NumberOrString::Number(sh * scale),
                rotation: item.f64_field("shape_rotate_offset")
                    .or(item.f64_field("config_rotate_offset"))
                    .map(NumberOrString::Number),
                anchor: map_anchor(item.str_field("position_anchor")),
                shape_kind: map_shape_kind(item.str_field("shape_type")),
                fill: if item.str_field("paint_style") == Some("STROKE") {
                    None // Outline only, no fill
                } else {
                    resolve_color_prop("paint_color", item.str_field("paint_color"), formulas, globals)
                },
                stroke: if item.str_field("paint_style") == Some("STROKE") || item.has_field("paint_stroke") {
                    resolve_color_prop("paint_color", item.str_field("paint_color"), formulas, globals)
                } else {
                    None
                },
                stroke_width: item.f64_field("paint_stroke"),
                corner_radius: item.f64_field("shape_corners"),
                shadow: convert_shadow(item, scale),
                ..Default::default()
            };
            (LayerType::Shape, props)
        }

        "BitmapModule" | "MovieModule" => {
            let src = resolve_string_prop(
                "bitmap_bitmap",
                item.str_field("bitmap_bitmap"),
                formulas, globals,
            ).map(|s| resolve_asset_path(&s, assets_dir));

            let bw = item.f64_field("bitmap_width");
            let props = LayerProperties {
                x: NumberOrString::Number(item.f64_field("position_offset_x").unwrap_or(0.0) * scale),
                y: NumberOrString::Number(item.f64_field("position_offset_y").unwrap_or(0.0) * scale),
                width: resolve_number_prop("bitmap_width", bw.map(|w| w * scale), formulas, globals)
                    .unwrap_or(NumberOrString::Number(200.0)),
                height: resolve_number_prop("bitmap_height", item.f64_field("bitmap_height").map(|h| h * scale), formulas, globals)
                    .unwrap_or(NumberOrString::Number(200.0)),
                rotation: item.f64_field("bitmap_rotate_offset")
                    .or(item.f64_field("config_rotate_offset"))
                    .map(NumberOrString::Number),
                opacity: item.f64_field("bitmap_alpha").map(NumberOrString::Number),
                anchor: map_anchor(item.str_field("position_anchor")),
                src,
                scale_mode: Some(ScaleMode::Fit),
                ..Default::default()
            };
            (LayerType::Image, props)
        }

        "FontIconModule" => {
            let icon_set = item.str_field("icon_set").map(|f| extract_font_name(f, assets_dir));
            let icon_size = item.f64_field("icon_size").unwrap_or(48.0);

            let props = LayerProperties {
                x: NumberOrString::Number(item.f64_field("position_offset_x").unwrap_or(0.0) * scale),
                y: NumberOrString::Number(item.f64_field("position_offset_y").unwrap_or(0.0) * scale),
                width: NumberOrString::Number(icon_size * scale),
                height: NumberOrString::Number(icon_size * scale),
                rotation: item.f64_field("icon_rotate_offset")
                    .or(item.f64_field("config_rotate_offset"))
                    .map(NumberOrString::Number),
                anchor: map_anchor(item.str_field("position_anchor")),
                color: resolve_color_prop("paint_color", item.str_field("paint_color"), formulas, globals),
                icon_set,
                glyph_code: item.str_field("icon_icon").map(|s| s.to_string()),
                ..Default::default()
            };
            (LayerType::Fonticon, props)
        }

        "ProgressModule" => {
            let style = match item.str_field("progress_mode") {
                Some("SHAPES") | Some("LINE") => ProgressStyle::Bar,
                _ => ProgressStyle::Arc,
            };

            let sw = item.f64_field("style_width")
                .or(item.f64_field("style_size"))
                .unwrap_or(80.0);
            let sh = item.f64_field("style_height")
                .or(item.f64_field("style_size"))
                .unwrap_or(80.0);

            let props = LayerProperties {
                x: NumberOrString::Number(item.f64_field("position_offset_x").unwrap_or(0.0) * scale),
                y: NumberOrString::Number(item.f64_field("position_offset_y").unwrap_or(0.0) * scale),
                width: NumberOrString::Number(sw * scale),
                height: NumberOrString::Number(sh * scale),
                anchor: map_anchor(item.str_field("position_anchor")),
                style: Some(style),
                min: Some(0.0),
                max: Some(100.0),
                value: resolve_number_prop("progress_level", item.f64_field("progress_level"), formulas, globals)
                    .or(Some(NumberOrString::Number(50.0))),
                color: resolve_color_prop("color_fgcolor", None, formulas, globals)
                    .or_else(|| resolve_color_prop("paint_color", item.str_field("paint_color"), formulas, globals)),
                track_color: resolve_color_prop("color_bgcolor", None, formulas, globals),
                ..Default::default()
            };
            (LayerType::Progress, props)
        }

        "OverlapLayerModule" | "KomponentModule" | "RootLayerModule" => {
            // Containers fill their parent like in KLWP. Padding is not subtracted
            // from dimensions — it's internal spacing handled by the rendering.
            let scale_pct = item.f64_field("config_scale_value").unwrap_or(100.0) / 100.0;
            let w = parent_width * scale_pct;
            let h = parent_height * scale_pct;

            let props = LayerProperties {
                x: NumberOrString::Number(item.f64_field("position_offset_x").unwrap_or(0.0) * scale),
                y: NumberOrString::Number(item.f64_field("position_offset_y").unwrap_or(0.0) * scale),
                width: NumberOrString::Number(w),
                height: NumberOrString::Number(h),
                anchor: map_anchor(item.str_field("position_anchor")),
                ..Default::default()
            };
            (LayerType::Overlap, props)
        }

        "StackLayerModule" => {
            let stacking = item.str_field("config_stacking").unwrap_or("VERTICAL_CENTER");
            let orientation = if stacking.starts_with("HORIZONTAL") {
                Some(Orientation::Horizontal)
            } else {
                Some(Orientation::Vertical)
            };

            // Containers fill their parent like in KLWP. Padding is not subtracted.
            let scale_pct = item.f64_field("config_scale_value").unwrap_or(100.0) / 100.0;
            let w = parent_width * scale_pct;
            let h = parent_height * scale_pct;

            let props = LayerProperties {
                x: NumberOrString::Number(item.f64_field("position_offset_x").unwrap_or(0.0) * scale),
                y: NumberOrString::Number(item.f64_field("position_offset_y").unwrap_or(0.0) * scale),
                width: NumberOrString::Number(w),
                height: NumberOrString::Number(h),
                anchor: map_anchor(item.str_field("position_anchor")),
                orientation,
                spacing: item.f64_field("config_margin"),
                ..Default::default()
            };
            (LayerType::Stack, props)
        }

        other => {
            warnings.push(format!("Unsupported item type: {}", other));
            return None;
        }
    };

    // Convert children recursively, passing this item's dimensions as the parent size
    let child_parent_w = match &properties.width {
        NumberOrString::Number(w) => *w,
        _ => parent_width,
    };
    let child_parent_h = match &properties.height {
        NumberOrString::Number(h) => *h,
        _ => parent_height,
    };
    let children: Vec<Layer> = item.children.iter()
        .filter_map(|child| convert_item(child, warnings, scale, assets_dir, child_parent_w, child_parent_h))
        .collect();

    let mut properties = properties;

    // Auto-size containers based on content (like Android wrap_content).
    // This ensures anchors like CENTER resolve against actual content bounds.
    // Cap at parent dimensions to prevent overflow from oversized children
    // (e.g. shapes wider than screen for edge coverage).
    let is_container = matches!(layer_type, LayerType::Overlap | LayerType::Stack);
    if is_container && !children.is_empty() {
        let max_w = match &properties.width {
            NumberOrString::Number(v) => *v,
            _ => parent_width,
        };
        let max_h = match &properties.height {
            NumberOrString::Number(v) => *v,
            _ => parent_height,
        };

        let content_h = compute_content_height(&children, &layer_type, properties.orientation.as_ref());
        if content_h > 0.0 {
            properties.height = NumberOrString::Number(content_h.min(max_h));
        }
        let content_w = compute_content_width(&children, &layer_type, properties.orientation.as_ref());
        if content_w > 0.0 {
            properties.width = NumberOrString::Number(content_w.min(max_w));
        }
    }

    let name = item.str_field("internal_title")
        .map(|s| s.to_string())
        .unwrap_or_else(|| item.internal_type().trim_end_matches("Module").to_string());

    // Map config_visible formula to properties.visible
    if let Some(vis_formula) = formulas.get("config_visible") {
        properties.visible = Some(BoolOrString::String(wrap_formula(vis_formula)));
    } else if let Some(vis_global) = globals.get("config_visible") {
        properties.visible = Some(BoolOrString::String(format!("$gv({})$", vis_global)));
    } else {
        // Check static config_visible value
        match item.str_field("config_visible") {
            Some("NEVER") | Some("REMOVE") => {
                properties.visible = Some(BoolOrString::String("never".to_string()));
            }
            Some("ALWAYS") | None => {} // default visible
            Some(other) => {
                properties.visible = Some(BoolOrString::String(other.to_string()));
            }
        }
    }

    Some(Layer {
        id: next_id(),
        name,
        layer_type,
        properties,
        animations: None,
        children: if children.is_empty() { None } else { Some(children) },
        locked: None,
        visible: Some(true),
    })
}

/// Compute content-based height for containers (wrap_content behavior).
/// For Overlap: max of children's (y + height).
/// For Stack vertical: sum of children heights. Horizontal: max height.
fn compute_content_height(children: &[Layer], layer_type: &LayerType, orientation: Option<&Orientation>) -> f64 {
    match layer_type {
        LayerType::Overlap => {
            let mut max_h: f64 = 0.0;
            for child in children {
                let cy = match &child.properties.y {
                    NumberOrString::Number(v) => v.max(0.0),
                    _ => 0.0,
                };
                let ch = match &child.properties.height {
                    NumberOrString::Number(v) => *v,
                    _ => 0.0,
                };
                max_h = max_h.max(cy + ch);
            }
            max_h
        }
        LayerType::Stack => {
            let is_horizontal = matches!(orientation, Some(Orientation::Horizontal));
            if is_horizontal {
                children.iter().map(|c| match &c.properties.height {
                    NumberOrString::Number(v) => *v,
                    _ => 0.0,
                }).fold(0.0_f64, f64::max)
            } else {
                children.iter().map(|c| match &c.properties.height {
                    NumberOrString::Number(v) => *v,
                    _ => 0.0,
                }).sum()
            }
        }
        _ => 0.0,
    }
}

/// Compute content-based width for containers (wrap_content behavior).
/// For Overlap: max of children's (x + width).
/// For Stack horizontal: sum of children widths. Vertical: max width.
fn compute_content_width(children: &[Layer], layer_type: &LayerType, orientation: Option<&Orientation>) -> f64 {
    match layer_type {
        LayerType::Overlap => {
            let mut max_w: f64 = 0.0;
            for child in children {
                let cx = match &child.properties.x {
                    NumberOrString::Number(v) => v.max(0.0),
                    _ => 0.0,
                };
                let cw = match &child.properties.width {
                    NumberOrString::Number(v) => *v,
                    _ => 0.0,
                };
                max_w = max_w.max(cx + cw);
            }
            max_w
        }
        LayerType::Stack => {
            let is_horizontal = matches!(orientation, Some(Orientation::Horizontal));
            if is_horizontal {
                children.iter().map(|c| match &c.properties.width {
                    NumberOrString::Number(v) => *v,
                    _ => 0.0,
                }).sum()
            } else {
                children.iter().map(|c| match &c.properties.width {
                    NumberOrString::Number(v) => *v,
                    _ => 0.0,
                }).fold(0.0_f64, f64::max)
            }
        }
        _ => 0.0,
    }
}

fn convert_shadow(item: &KlwpItem, scale: f64) -> Option<Shadow> {
    if !item.has_field("fx_shadow") && !item.has_field("fx_shadow_blur") {
        return None;
    }
    let distance = item.f64_field("fx_shadow_distance").unwrap_or(2.0) * scale;
    let direction_deg = item.f64_field("fx_shadow_direction").unwrap_or(315.0);
    let rad = direction_deg.to_radians();
    Some(Shadow {
        color: item.str_field("fx_shadow_color")
            .map(convert_color)
            .unwrap_or_else(|| "#00000080".to_string()),
        dx: distance * rad.cos(),
        dy: distance * rad.sin(),
        radius: item.f64_field("fx_shadow_blur").unwrap_or(4.0) * scale,
    })
}

/// Convert KLWP globals to our GlobalVariable format
fn convert_globals(globals: &HashMap<String, KlwpGlobal>) -> Vec<GlobalVariable> {
    let mut result: Vec<GlobalVariable> = globals.iter().map(|(key, g)| {
        let var_type = match g.var_type.as_deref() {
            Some("COLOR") => GlobalVarType::Color,
            Some("NUMBER") => GlobalVarType::Number,
            Some("SWITCH") => GlobalVarType::Switch,
            Some("LIST") => GlobalVarType::List,
            Some("FONT") | Some("TEXT") | _ => GlobalVarType::Text,
        };

        let value = match &g.value {
            Some(JsonValue::String(s)) => {
                if matches!(var_type, GlobalVarType::Color) {
                    GlobalVarValue::String(convert_color(s))
                } else {
                    GlobalVarValue::String(s.clone())
                }
            }
            Some(JsonValue::Number(n)) => {
                if let Some(f) = n.as_f64() {
                    GlobalVarValue::Number(f)
                } else {
                    GlobalVarValue::String(n.to_string())
                }
            }
            Some(JsonValue::Bool(b)) => GlobalVarValue::Bool(*b),
            _ => GlobalVarValue::String(String::new()),
        };

        let options = g.entries.as_ref().map(|e| {
            e.split("##").flat_map(|group| {
                group.split(',').map(|s| s.trim().to_string())
            }).collect()
        });

        GlobalVariable {
            name: key.clone(),
            var_type,
            value,
            options,
        }
    }).collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

fn collect_all_globals(item: &KlwpItem, globals: &mut Vec<GlobalVariable>) {
    let mut seen = std::collections::HashSet::new();
    collect_all_globals_inner(item, globals, &mut seen);
}

fn collect_all_globals_inner(item: &KlwpItem, globals: &mut Vec<GlobalVariable>, seen: &mut std::collections::HashSet<String>) {
    if !item.globals_list.is_empty() {
        for g in convert_globals(&item.globals_list) {
            if seen.insert(g.name.clone()) {
                globals.push(g);
            }
        }
    }
    for child in &item.children {
        collect_all_globals_inner(child, globals, seen);
    }
}

/// Import a .komp komponent file: reads the ZIP, extracts komponent.json, converts to a Layer tree
pub fn import_komp_file(
    komp_path: &str,
    output_dir: &str,
    _target_width: u32,
    _target_height: u32,
) -> Result<KompImportResult, String> {
    let path = Path::new(komp_path);
    if !path.exists() {
        return Err(format!("File not found: {}", komp_path));
    }

    let file = fs::File::open(path).map_err(|e| format!("Cannot open file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP/KOMP file: {}", e))?;

    // Read komponent.json
    let komp_json = {
        let mut entry = archive.by_name("komponent.json")
            .map_err(|_| "No komponent.json found in .komp file")?;
        let mut contents = String::new();
        entry.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read komponent.json: {}", e))?;
        contents
    };

    let raw: JsonValue = serde_json::from_str(&komp_json)
        .map_err(|e| format!("Failed to parse komponent.json: {}", e))?;

    let preset_info: KlwpInfo = serde_json::from_value(
        raw.get("preset_info").cloned().unwrap_or(JsonValue::Null)
    ).map_err(|e| format!("Failed to parse preset_info: {}", e))?;

    let preset_root = KlwpItem::from_value(
        raw.get("preset_root").ok_or("No preset_root in .komp file")?
    ).ok_or("Failed to parse preset_root")?;

    // Extract assets to output directory
    let assets_dir = Path::new(output_dir).join("assets");
    let mut asset_count = 0;
    let mut warnings = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let entry_name = entry.name().to_string();

        if entry_name == "komponent.json" || entry_name.starts_with("komponent_thumb") {
            continue;
        }

        let out_path = assets_dir.join(&entry_name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        if !entry.is_dir() {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).ok();
            if fs::write(&out_path, &buf).is_ok() {
                asset_count += 1;
            }
        }
    }

    // Komponents are small widgets in dp units (e.g. 70x40).
    // Use a fixed dp-to-px multiplier instead of screen-height ratio.
    let source_width = preset_info.width.unwrap_or(70) as f64;
    let source_height = preset_info.height.unwrap_or(40) as f64;
    let dp_scale = 3.0; // 3x dp-to-px, reasonable for modern displays
    let komp_width = source_width * dp_scale;
    let komp_height = source_height * dp_scale;

    reset_counter();

    let assets_dir_str = assets_dir.to_string_lossy().to_string();

    // Convert the root KomponentModule using komponent's own dimensions, not screen size
    let root = convert_item(&preset_root, &mut warnings, dp_scale, &assets_dir_str, komp_width, komp_height)
        .ok_or("Failed to convert komponent root")?;

    // Collect all globals from the komponent
    let mut globals = Vec::new();
    collect_all_globals(&preset_root, &mut globals);

    Ok(KompImportResult {
        root,
        globals,
        warnings,
        asset_count,
        asset_dir: assets_dir_str,
    })
}

impl Default for LayerProperties {
    fn default() -> Self {
        LayerProperties {
            x: NumberOrString::Number(0.0),
            y: NumberOrString::Number(0.0),
            width: NumberOrString::Number(100.0),
            height: NumberOrString::Number(100.0),
            rotation: None,
            scale_x: None,
            scale_y: None,
            opacity: None,
            anchor: None,
            visible: None,
            text: None,
            font_size: None,
            font_family: None,
            color: None,
            text_align: None,
            max_lines: None,
            line_spacing: None,
            shadow: None,
            shape_kind: None,
            fill: None,
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            src: None,
            scale_mode: None,
            tint: None,
            style: None,
            min: None,
            max: None,
            value: None,
            track_color: None,
            icon_set: None,
            glyph_code: None,
            orientation: None,
            spacing: None,
            click_action: None,
            icon_src: None,
            viz_style: None,
            bar_count: None,
            bar_spacing: None,
            sensitivity: None,
            color_top: None,
            color_mid: None,
            color_bottom: None,
            peak_color: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_color_argb_to_rgba() {
        assert_eq!(convert_color("#FF000000"), "#000000");
        assert_eq!(convert_color("#FFDC7E30"), "#DC7E30");
        assert_eq!(convert_color("#8FE6E6E6"), "#E6E6E68F");
        assert_eq!(convert_color("#00FFFFFF"), "#FFFFFF00");
    }

    #[test]
    fn test_convert_color_short() {
        assert_eq!(convert_color("#FF0000"), "#FF0000");
    }

    #[test]
    fn test_map_anchor() {
        assert!(matches!(map_anchor(Some("CENTER")), Some(AnchorPoint::Center)));
        assert!(matches!(map_anchor(Some("TOPLEFT")), Some(AnchorPoint::TopLeft)));
        assert!(map_anchor(Some("UNKNOWN")).is_none());
        assert!(map_anchor(None).is_none());
    }

    #[test]
    fn test_map_shape_kind() {
        assert!(matches!(map_shape_kind(Some("RECT")), Some(ShapeKind::Rectangle)));
        assert!(matches!(map_shape_kind(Some("CIRCLE")), Some(ShapeKind::Circle)));
        assert!(matches!(map_shape_kind(Some("TRIANGLE")), Some(ShapeKind::Triangle)));
    }

    #[test]
    fn test_resolve_with_formula() {
        let mut formulas = HashMap::new();
        formulas.insert("paint_color".to_string(), "gv(accent)".to_string());
        let globals = HashMap::new();

        let result = resolve_string_prop("paint_color", Some("#FF0000"), &formulas, &globals);
        assert_eq!(result, Some("$gv(accent)$".to_string()));
    }

    #[test]
    fn test_resolve_with_already_wrapped_formula() {
        let mut formulas = HashMap::new();
        // KLWP/KOMP internal_formulas already include $...$ delimiters
        formulas.insert("paint_color".to_string(), "$if(gv(x)=1, #FF0000, #00FF00)$".to_string());
        let globals = HashMap::new();

        let result = resolve_string_prop("paint_color", Some("#FF0000"), &formulas, &globals);
        // Should NOT double-wrap
        assert_eq!(result, Some("$if(gv(x)=1, #FF0000, #00FF00)$".to_string()));
    }

    #[test]
    fn test_resolve_with_global() {
        let formulas = HashMap::new();
        let mut globals = HashMap::new();
        globals.insert("paint_color".to_string(), "mycolor".to_string());

        let result = resolve_string_prop("paint_color", Some("#FF0000"), &formulas, &globals);
        assert_eq!(result, Some("$gv(mycolor)$".to_string()));
    }

    #[test]
    fn test_resolve_static() {
        let formulas = HashMap::new();
        let globals = HashMap::new();

        let result = resolve_string_prop("paint_color", Some("#FF0000"), &formulas, &globals);
        assert_eq!(result, Some("#FF0000".to_string()));
    }

    #[test]
    fn test_extract_font_name() {
        // With non-existent assets dir, falls back to filename stem
        assert_eq!(
            extract_font_name("kfile://org.kustom.provider/fonts/Medium.ttf", "/nonexistent"),
            "Medium"
        );
        assert_eq!(extract_font_name("Roboto", "/nonexistent"), "Roboto");
    }

    #[test]
    fn test_import_komp_toggle() {
        let komp_path = "/home/andason/Downloads/Dynamic Komponents for KLWP_2.1_APKPure/assets/komponents/Dynamic_Toggle.komp";
        if !Path::new(komp_path).exists() {
            return;
        }

        let result = import_komp_file(komp_path, "/tmp/komp_import_test", 1920, 1080);
        let r = result.expect("Import should succeed");

        // Verify root dimensions match komp source (70x40dp * 3x scale = 210x120px)
        assert!(matches!(r.root.properties.width, NumberOrString::Number(w) if (w - 210.0).abs() < 0.1));
        assert!(matches!(r.root.properties.height, NumberOrString::Number(h) if (h - 120.0).abs() < 0.1));
        assert!(!r.globals.is_empty(), "Should have globals");
        assert!(r.root.children.is_some(), "Should have children");
    }

}
