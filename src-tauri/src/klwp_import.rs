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

/// Import result with project and warnings
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub project: Project,
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

/// Resolve a property: check formulas first, then globals binding, then static value
fn resolve_string_prop(
    key: &str,
    static_val: Option<&str>,
    formulas: &HashMap<String, String>,
    globals: &HashMap<String, String>,
) -> Option<String> {
    if let Some(formula) = formulas.get(key) {
        return Some(format!("${}$", formula));
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
        return Some(format!("${}$", formula));
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
        return Some(NumberOrString::String(format!("${}$", formula)));
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
            let props = LayerProperties {
                x: NumberOrString::Number(item.f64_field("position_offset_x").unwrap_or(0.0) * scale),
                y: NumberOrString::Number(item.f64_field("position_offset_y").unwrap_or(0.0) * scale),
                width: NumberOrString::Number(item.f64_field("shape_width").unwrap_or(100.0) * scale),
                height: NumberOrString::Number(item.f64_field("shape_height").unwrap_or(100.0) * scale),
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
            let pad_left = item.f64_field("position_padding_left").unwrap_or(0.0) * scale;
            let pad_right = item.f64_field("position_padding_right").unwrap_or(0.0) * scale;
            let pad_top = item.f64_field("position_padding_top").unwrap_or(0.0) * scale;
            let pad_bottom = item.f64_field("position_padding_bottom").unwrap_or(0.0) * scale;
            let scale_pct = item.f64_field("config_scale_value").unwrap_or(100.0) / 100.0;
            let available_w = parent_width - pad_left - pad_right;
            let available_h = parent_height - pad_top - pad_bottom;
            let w = available_w * scale_pct;
            let h = available_h * scale_pct;

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

            let pad_left = item.f64_field("position_padding_left").unwrap_or(0.0) * scale;
            let pad_right = item.f64_field("position_padding_right").unwrap_or(0.0) * scale;
            let pad_top = item.f64_field("position_padding_top").unwrap_or(0.0) * scale;
            let pad_bottom = item.f64_field("position_padding_bottom").unwrap_or(0.0) * scale;
            let scale_pct = item.f64_field("config_scale_value").unwrap_or(100.0) / 100.0;
            let available_w = parent_width - pad_left - pad_right;
            let available_h = parent_height - pad_top - pad_bottom;
            let w = available_w * scale_pct;
            let h = available_h * scale_pct;

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

    let name = item.str_field("internal_title")
        .map(|s| s.to_string())
        .unwrap_or_else(|| item.internal_type().trim_end_matches("Module").to_string());

    // Map config_visible formula to properties.visible
    let mut properties = properties;
    if let Some(vis_formula) = formulas.get("config_visible") {
        properties.visible = Some(BoolOrString::String(format!("${}$", vis_formula)));
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

/// Main import function: reads a .klwp ZIP, extracts preset.json, converts to Project
pub fn import_klwp_file(
    klwp_path: &str,
    output_dir: &str,
    target_width: u32,
    target_height: u32,
) -> Result<ImportResult, String> {
    let path = Path::new(klwp_path);
    if !path.exists() {
        return Err(format!("File not found: {}", klwp_path));
    }

    let file = fs::File::open(path).map_err(|e| format!("Cannot open file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP/KLWP file: {}", e))?;

    // Read preset.json
    let preset_json = {
        let mut entry = archive.by_name("preset.json")
            .map_err(|_| "No preset.json found in .klwp file")?;
        let mut contents = String::new();
        entry.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read preset.json: {}", e))?;
        contents
    };

    // Parse as generic JSON first, then extract parts
    let raw: JsonValue = serde_json::from_str(&preset_json)
        .map_err(|e| format!("Failed to parse preset.json: {}", e))?;

    let preset_info: KlwpInfo = serde_json::from_value(
        raw.get("preset_info").cloned().unwrap_or(JsonValue::Null)
    ).map_err(|e| format!("Failed to parse preset_info: {}", e))?;

    let preset_root = KlwpItem::from_value(
        raw.get("preset_root").ok_or("No preset_root in .klwp file")?
    ).ok_or("Failed to parse preset_root")?;

    // Extract assets to output directory
    let assets_dir = Path::new(output_dir).join("assets");
    let mut asset_count = 0;
    let mut warnings = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let entry_name = entry.name().to_string();

        if entry_name == "preset.json" || entry_name.starts_with("preset_thumb") {
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

    // Calculate scale factors
    let source_width = preset_info.width.unwrap_or(540) as f64;
    let source_height = preset_info.height.unwrap_or(1170) as f64;
    // Scale elements uniformly by height ratio so they're proportional to screen.
    let scale = target_height as f64 / source_height;
    let scaled_content_width = source_width * scale;
    let x_center_offset = (target_width as f64 - scaled_content_width) / 2.0;

    reset_counter();

    let assets_dir_str = assets_dir.to_string_lossy().to_string();

    // Convert the root item's children to layers, using target dimensions as initial parent size
    let mut layers: Vec<Layer> = preset_root.children.iter()
        .filter_map(|item| convert_item(item, &mut warnings, scale, &assets_dir_str, target_width as f64, target_height as f64))
        .collect();

    // Center content horizontally by offsetting top-level layer positions
    for layer in &mut layers {
        if let NumberOrString::Number(x) = &layer.properties.x {
            layer.properties.x = NumberOrString::Number(x + x_center_offset);
        }
    }

    // Collect all globals
    let mut all_globals = Vec::new();
    collect_all_globals(&preset_root, &mut all_globals);

    // Build background
    let background = match preset_root.str_field("background_type") {
        Some("IMAGE") => {
            let bg_path = preset_root.str_field("background_bitmap")
                .map(|p| resolve_asset_path(p, &assets_dir_str))
                .unwrap_or_default();
            Background {
                bg_type: BackgroundType::Image,
                value: bg_path,
            }
        }
        _ => {
            let bg_color = preset_root.str_field("background_color")
                .map(|c| convert_color(c))
                .unwrap_or_else(|| "#1a1a2e".to_string());
            Background {
                bg_type: BackgroundType::Color,
                value: bg_color,
            }
        }
    };

    let project = Project {
        version: "0.1.0".to_string(),
        name: preset_info.title.unwrap_or_else(|| "Imported KLWP".to_string()),
        resolution: Resolution {
            width: target_width,
            height: target_height,
        },
        background,
        globals: all_globals,
        layers,
    };

    warnings.push(format!(
        "Scaled from {}x{} (mobile portrait) to {}x{} (desktop) using uniform scale {:.2}x, centered with x_offset={:.1}px. Layout may need manual adjustment.",
        source_width as u32, source_height as u32, target_width, target_height, scale, x_center_offset
    ));

    Ok(ImportResult {
        project,
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
    fn test_import_fade_black() {
        let klwp_path = "/home/andason/Downloads/Fade_black.klwp";
        if !Path::new(klwp_path).exists() {
            return;
        }

        let result = import_klwp_file(klwp_path, "/tmp/klwp_import_test", 1920, 1080);
        let r = result.expect("Import should succeed");

        assert_eq!(r.project.name, "Fade black");
        assert!(!r.project.layers.is_empty(), "Should have layers");
        assert!(!r.project.globals.is_empty(), "Should have globals");
        assert!(r.asset_count > 0, "Should extract assets");
        assert_eq!(r.project.resolution.width, 1920);
        assert_eq!(r.project.resolution.height, 1080);
    }
}
