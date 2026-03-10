use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::klwp_import::KompImportResult;
use crate::project::{
    BoolOrString, GlobalVarType, GlobalVarValue, GlobalVariable, Layer, LayerProperties,
    LayerType, NumberOrString, Orientation, ProgressStyle, ScaleMode, Shadow, ShapeKind, TextAlign,
};

// ---------------------------------------------------------------------------
// ID generation
// ---------------------------------------------------------------------------

static RM_COUNTER: AtomicU32 = AtomicU32::new(0);

fn next_id() -> String {
    let id = RM_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("rm_{}", id)
}

fn reset_counter() {
    RM_COUNTER.store(0, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// INI parser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct IniSection {
    name: String,
    options: HashMap<String, String>,
}

/// Decode bytes to a String, detecting UTF-16LE BOM.
fn decode_ini_bytes(data: &[u8]) -> String {
    if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
        // UTF-16LE with BOM
        let body = &data[2..];
        let len = body.len() / 2;
        let mut u16s = Vec::with_capacity(len);
        for i in 0..len {
            let lo = body[i * 2] as u16;
            let hi = body[i * 2 + 1] as u16;
            u16s.push(lo | (hi << 8));
        }
        String::from_utf16_lossy(&u16s)
    } else if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        // UTF-8 with BOM
        String::from_utf8_lossy(&data[3..]).into_owned()
    } else {
        String::from_utf8_lossy(data).into_owned()
    }
}

/// Parse INI text into ordered sections.
fn parse_ini(text: &str) -> Vec<IniSection> {
    let mut sections: Vec<IniSection> = Vec::new();
    let mut current: Option<IniSection> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            if let Some(end) = line.find(']') {
                if let Some(sec) = current.take() {
                    sections.push(sec);
                }
                current = Some(IniSection {
                    name: line[1..end].to_string(),
                    options: HashMap::new(),
                });
                continue;
            }
        }
        if let Some(ref mut sec) = current {
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let val = line[eq_pos + 1..].trim().to_string();
                sec.options.insert(key, val);
            }
        }
    }
    if let Some(sec) = current {
        sections.push(sec);
    }
    sections
}

// ---------------------------------------------------------------------------
// Simple arithmetic expression evaluator
// ---------------------------------------------------------------------------

/// Evaluate a simple arithmetic expression containing numbers, +, -, *, /, and parentheses.
/// Returns None if the expression cannot be evaluated.
fn eval_expr(expr: &str) -> Option<f64> {
    let tokens = tokenize_expr(expr)?;
    let (val, rest) = parse_add_sub(&tokens)?;
    if rest.is_empty() {
        Some(val)
    } else {
        None
    }
}

#[derive(Debug, Clone)]
enum Token {
    Num(f64),
    Op(char),
    LParen,
    RParen,
}

fn tokenize_expr(s: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' => {
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '+' | '*' | '/' => {
                tokens.push(Token::Op(chars[i]));
                i += 1;
            }
            '-' => {
                // Unary minus: at start, after '(' or after an operator
                let is_unary = tokens.is_empty()
                    || matches!(
                        tokens.last(),
                        Some(Token::Op(_)) | Some(Token::LParen)
                    );
                if is_unary {
                    // Collect number with leading minus
                    let start = i;
                    i += 1;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    let num_str: String = chars[start..i].iter().collect();
                    tokens.push(Token::Num(num_str.parse().ok()?));
                } else {
                    tokens.push(Token::Op('-'));
                    i += 1;
                }
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                tokens.push(Token::Num(num_str.parse().ok()?));
            }
            _ => return None, // Unknown character — cannot evaluate
        }
    }
    Some(tokens)
}

fn parse_add_sub<'a>(tokens: &'a [Token]) -> Option<(f64, &'a [Token])> {
    let (mut left, mut rest) = parse_mul_div(tokens)?;
    while let Some(Token::Op(op @ ('+' | '-'))) = rest.first() {
        let op = *op;
        let (right, r) = parse_mul_div(&rest[1..])?;
        rest = r;
        left = if op == '+' { left + right } else { left - right };
    }
    Some((left, rest))
}

fn parse_mul_div<'a>(tokens: &'a [Token]) -> Option<(f64, &'a [Token])> {
    let (mut left, mut rest) = parse_atom(tokens)?;
    while let Some(Token::Op(op @ ('*' | '/'))) = rest.first() {
        let op = *op;
        let (right, r) = parse_atom(&rest[1..])?;
        rest = r;
        left = if op == '*' {
            left * right
        } else {
            if right == 0.0 {
                return None;
            }
            left / right
        };
    }
    Some((left, rest))
}

fn parse_atom<'a>(tokens: &'a [Token]) -> Option<(f64, &'a [Token])> {
    match tokens.first()? {
        Token::Num(n) => Some((*n, &tokens[1..])),
        Token::LParen => {
            let (val, rest) = parse_add_sub(&tokens[1..])?;
            match rest.first() {
                Some(Token::RParen) => Some((val, &rest[1..])),
                _ => None,
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Color conversion
// ---------------------------------------------------------------------------

/// Convert Rainmeter `R,G,B` or `R,G,B,A` (0-255) to `#RRGGBB` or `#RRGGBBAA`.
fn convert_color(rm_color: &str) -> String {
    let parts: Vec<&str> = rm_color.split(',').map(|s| s.trim()).collect();
    if parts.len() >= 3 {
        let r: u8 = parts[0].parse().unwrap_or(0);
        let g: u8 = parts[1].parse().unwrap_or(0);
        let b: u8 = parts[2].parse().unwrap_or(0);
        if parts.len() >= 4 {
            let a: u8 = parts[3].parse().unwrap_or(255);
            if a == 255 {
                format!("#{:02X}{:02X}{:02X}", r, g, b)
            } else {
                format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
            }
        } else {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        }
    } else {
        // Maybe it's already hex or something else — return as-is
        rm_color.to_string()
    }
}

/// Check whether a string looks like a Rainmeter color value (R,G,B or R,G,B,A).
fn is_rm_color(val: &str) -> bool {
    let parts: Vec<&str> = val.split(',').map(|s| s.trim()).collect();
    (parts.len() == 3 || parts.len() == 4) && parts.iter().all(|p| p.parse::<u8>().is_ok())
}

// ---------------------------------------------------------------------------
// Variable substitution
// ---------------------------------------------------------------------------

/// Replace all `#VarName#` patterns with variable values.
fn substitute_variables(s: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' {
            // Look for closing #
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '#') {
                let var_name: String = chars[i + 1..i + 1 + end].iter().collect();
                // #@# is the @Resources path marker, not a variable
                if var_name == "@" {
                    result.push_str("#@#");
                    i += 3;
                    continue;
                }
                if let Some(val) = vars.get(&var_name) {
                    result.push_str(val);
                } else {
                    // Unknown variable — leave as-is
                    result.push('#');
                    result.push_str(&var_name);
                    result.push('#');
                }
                i += end + 2; // skip past closing #
            } else {
                result.push(chars[i]);
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Numeric value parsing with expression support
// ---------------------------------------------------------------------------

/// Parse a numeric value that may be an expression like `(#size#*256)`.
/// Variables should already be substituted. Strips outer parens for evaluation.
fn parse_numeric(val: &str, warnings: &mut Vec<String>) -> f64 {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    // Strip relative positioning suffix (r/R)
    let cleaned = trimmed.trim_end_matches(|c: char| c == 'r' || c == 'R');
    if cleaned.len() != trimmed.len() {
        warnings.push(format!(
            "Relative positioning '{}' converted to absolute",
            trimmed
        ));
    }
    // Try direct parse first
    if let Ok(n) = cleaned.parse::<f64>() {
        return n;
    }
    // Try expression evaluation — strip outer parens if present
    let expr = if cleaned.starts_with('(') && cleaned.ends_with(')') {
        &cleaned[1..cleaned.len() - 1]
    } else {
        cleaned
    };
    if let Some(n) = eval_expr(expr) {
        return n;
    }
    warnings.push(format!("Could not evaluate expression '{}', using 0", val));
    0.0
}

// ---------------------------------------------------------------------------
// Measure mapping
// ---------------------------------------------------------------------------

/// Build a mapping from measure names to our formula equivalents.
fn map_measures(
    sections: &[IniSection],
    vars: &HashMap<String, String>,
    warnings: &mut Vec<String>,
) -> HashMap<String, String> {
    let mut measures: HashMap<String, String> = HashMap::new();
    for sec in sections {
        let measure_type = match sec.options.get("Measure") {
            Some(m) => m.to_ascii_lowercase(),
            None => continue,
        };
        let name = &sec.name;
        let mapped = match measure_type.as_str() {
            "time" => {
                if let Some(fmt) = sec.options.get("Format") {
                    let kustom_fmt = convert_time_format(fmt);
                    format!("$df({})$", kustom_fmt)
                } else {
                    "$df(HH:mm)$".to_string()
                }
            }
            "cpu" => "$si(cpuutil)$".to_string(),
            "physicalmemory" => {
                let info_type = sec
                    .options
                    .get("InfoType")
                    .map(|s| s.as_str())
                    .unwrap_or("Free");
                match info_type {
                    "Free" => "$rm(freeram)$".to_string(),
                    "Used" => "$rm(usedram)$".to_string(),
                    "Total" => "$rm(totalram)$".to_string(),
                    _ => "$rm(freeram)$".to_string(),
                }
            }
            "string" => {
                if let Some(s) = sec.options.get("String") {
                    substitute_variables(s, vars)
                } else {
                    format!("{{{}}}", name)
                }
            }
            "calc" => {
                if let Some(formula) = sec.options.get("Formula") {
                    let sub = substitute_variables(formula, vars);
                    if let Some(n) = eval_expr(sub.trim()) {
                        format!("{}", n)
                    } else {
                        warnings.push(format!(
                            "Could not evaluate Calc measure '{}': {}",
                            name, formula
                        ));
                        format!("{{{}}}", name)
                    }
                } else {
                    format!("{{{}}}", name)
                }
            }
            "freediskspace" => "$rm(freedisk)$".to_string(),
            "uptime" => "$si(uptime)$".to_string(),
            _ => {
                warnings.push(format!(
                    "Unsupported measure type '{}' for [{}], using placeholder",
                    measure_type, name
                ));
                format!("{{{}}}", name)
            }
        };
        measures.insert(name.clone(), mapped);
    }
    measures
}

/// Convert Rainmeter time format to Kustom df() format (best-effort).
fn convert_time_format(rm_fmt: &str) -> String {
    rm_fmt
        .replace("%H", "HH")
        .replace("%I", "hh")
        .replace("%M", "mm")
        .replace("%S", "ss")
        .replace("%p", "a")
        .replace("%A", "EEEE")
        .replace("%a", "EEE")
        .replace("%B", "MMMM")
        .replace("%b", "MMM")
        .replace("%d", "dd")
        .replace("%m", "MM")
        .replace("%Y", "yyyy")
        .replace("%y", "yy")
        .replace("%j", "DDD")
}

// ---------------------------------------------------------------------------
// Shape string parsing
// ---------------------------------------------------------------------------

struct ParsedShape {
    kind: ShapeKind,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corner_radius: Option<f64>,
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: Option<f64>,
}

/// Parse a Rainmeter Shape= value.
/// Examples:
///   Rectangle 0,0,200,100,10 | Fill Color 255,255,255,200 | StrokeWidth 2 | Stroke Color 0,0,0,255
///   Ellipse 100,50,80,40 | Fill Color 200,100,50
fn parse_shape_string(shape_str: &str, warnings: &mut Vec<String>) -> Option<ParsedShape> {
    let parts: Vec<&str> = shape_str.split('|').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let geom = parts[0];
    let (kind, x, y, width, height, corner_radius) = if geom
        .to_ascii_lowercase()
        .starts_with("rectangle")
    {
        let nums_str = geom["rectangle".len()..].trim();
        let nums: Vec<f64> = nums_str
            .split(',')
            .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
            .collect();
        let x = *nums.first().unwrap_or(&0.0);
        let y = *nums.get(1).unwrap_or(&0.0);
        let w = *nums.get(2).unwrap_or(&100.0);
        let h = *nums.get(3).unwrap_or(&100.0);
        let cr = nums.get(4).copied().filter(|&v| v > 0.0);
        (ShapeKind::Rectangle, x, y, w, h, cr)
    } else if geom.to_ascii_lowercase().starts_with("ellipse") {
        let nums_str = geom["ellipse".len()..].trim();
        let nums: Vec<f64> = nums_str
            .split(',')
            .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
            .collect();
        let cx = *nums.first().unwrap_or(&50.0);
        let cy = *nums.get(1).unwrap_or(&50.0);
        let rx = *nums.get(2).unwrap_or(&50.0);
        let ry = nums.get(3).copied().unwrap_or(rx);
        let kind = if (rx - ry).abs() < 0.01 {
            ShapeKind::Circle
        } else {
            ShapeKind::Oval
        };
        (kind, cx - rx, cy - ry, rx * 2.0, ry * 2.0, None)
    } else {
        warnings.push(format!("Unsupported shape geometry: {}", geom));
        return None;
    };

    let mut fill: Option<String> = None;
    let mut stroke: Option<String> = None;
    let mut stroke_width: Option<f64> = None;

    for modifier in &parts[1..] {
        let lower = modifier.to_ascii_lowercase();
        if lower.starts_with("fill color") {
            let color_str = modifier["fill color".len()..].trim();
            fill = Some(convert_color(color_str));
        } else if lower.starts_with("stroke color") {
            let color_str = modifier["stroke color".len()..].trim();
            stroke = Some(convert_color(color_str));
        } else if lower.starts_with("strokewidth") {
            let w_str = modifier["strokewidth".len()..].trim();
            stroke_width = w_str.parse().ok();
        }
    }

    Some(ParsedShape {
        kind,
        x,
        y,
        width,
        height,
        corner_radius,
        fill,
        stroke,
        stroke_width,
    })
}

// ---------------------------------------------------------------------------
// Meter → Layer conversion
// ---------------------------------------------------------------------------

/// Resolve text content, replacing `[MeasureName]` with mapped values and `%1`, `%2`, etc.
/// with the corresponding MeasureName, MeasureName2, etc. values.
fn resolve_text_content(
    text: &str,
    measure_names: &[Option<String>],
    measures: &HashMap<String, String>,
) -> String {
    let mut result = text.to_string();
    // Replace [MeasureName] references
    let mut i = 0;
    let chars: Vec<char> = result.chars().collect();
    let mut output = String::with_capacity(result.len());
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == ']') {
                let ref_name: String = chars[i + 1..i + 1 + end].iter().collect();
                if let Some(mapped) = measures.get(&ref_name) {
                    output.push_str(mapped);
                } else {
                    output.push_str(&format!("{{{}}}", ref_name));
                }
                i += end + 2;
                continue;
            }
        }
        output.push(chars[i]);
        i += 1;
    }
    result = output;

    // Replace %1, %2, %3... with corresponding MeasureName values
    for (idx, mn_opt) in measure_names.iter().enumerate() {
        let placeholder = format!("%{}", idx + 1);
        if result.contains(&placeholder) {
            if let Some(mn) = mn_opt {
                if let Some(mapped) = measures.get(mn.as_str()) {
                    result = result.replace(&placeholder, mapped);
                } else {
                    result = result.replace(&placeholder, &format!("{{{}}}", mn));
                }
            }
        }
    }

    result
}

/// Resolve an image filename to an absolute path using Rainmeter's resolution logic:
/// 1. Prepend meter-level ImagePath (or global ImagePath)
/// 2. If still relative, resolve against skin INI directory (skin_files_dir/skin_subdir)
/// 3. Auto-append .png if no extension
fn resolve_image_path(
    image_name: &str,
    meter_image_path: Option<&str>,
    global_image_path: &str,
    asset_dir: &str,
    skin_files_dir: &str,
    skin_subdir: &str,
) -> String {
    if image_name.is_empty() {
        return String::new();
    }

    // Step 1: Determine the path prefix (meter-level ImagePath overrides global)
    let ip = meter_image_path.unwrap_or(global_image_path);
    let ip_resolved = if ip.is_empty() {
        String::new()
    } else {
        let mut p = ip.replace("#@#", &format!("{}/", asset_dir));
        p = p.replace('\\', "/");
        if !p.ends_with('/') {
            p.push('/');
        }
        p
    };

    // Step 2: Compose filename
    let name_resolved = image_name
        .replace("#@#", &format!("{}/", asset_dir))
        .replace('\\', "/");

    let mut filename = if name_resolved.starts_with('/') || name_resolved.contains(":/") {
        // Already absolute
        name_resolved
    } else if name_resolved.contains('/') && (name_resolved.starts_with(&format!("{}/", asset_dir)) || name_resolved.starts_with(asset_dir)) {
        // Already resolved to asset_dir
        name_resolved
    } else {
        format!("{}{}", ip_resolved, name_resolved)
    };

    // Step 3: If still a relative path (no asset_dir or skin_files_dir prefix), resolve against skin INI directory
    if !filename.starts_with('/') && !filename.contains(":/") {
        // Try skin_files_dir/skin_subdir/ first (where skin-directory images are extracted)
        let skin_path = if skin_subdir.is_empty() {
            format!("{}/{}", skin_files_dir, filename)
        } else {
            format!("{}/{}/{}", skin_files_dir, skin_subdir, filename)
        };
        filename = skin_path;
    }

    // Step 4: Auto-append .png if no extension
    if let Some(last_slash) = filename.rfind('/') {
        let after_slash = &filename[last_slash + 1..];
        if !after_slash.contains('.') {
            filename.push_str(".png");
        }
    } else if !filename.contains('.') {
        filename.push_str(".png");
    }

    filename
}

/// Track previous meter position for relative positioning.
#[derive(Default, Clone)]
struct MeterPos {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Parse a position value that may have r/R suffix for relative positioning.
/// Returns the absolute position.
fn parse_position(
    raw: &str,
    prev: f64,
    prev_size: f64,
    warnings: &mut Vec<String>,
) -> f64 {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return 0.0;
    }

    let last_char = trimmed.chars().last().unwrap_or(' ');
    if last_char == 'r' {
        // Relative to previous meter's top/left
        let num_part = &trimmed[..trimmed.len() - 1];
        let offset = parse_numeric(num_part, warnings);
        prev + offset
    } else if last_char == 'R' {
        // Relative to previous meter's bottom/right
        let num_part = &trimmed[..trimmed.len() - 1];
        let offset = parse_numeric(num_part, warnings);
        prev + prev_size + offset
    } else {
        parse_numeric(trimmed, warnings)
    }
}

/// Convert a single meter section to a Layer.
fn convert_meter(
    sec: &IniSection,
    styles: &HashMap<String, HashMap<String, String>>,
    measures: &HashMap<String, String>,
    global_image_path: &str,
    asset_dir: &str,
    skin_files_dir: &str,
    skin_subdir: &str,
    prev_pos: &MeterPos,
    warnings: &mut Vec<String>,
) -> Option<Layer> {
    // Merge MeterStyle if present
    let mut opts = HashMap::new();
    if let Some(style_name) = sec.options.get("MeterStyle") {
        if let Some(style_opts) = styles.get(style_name) {
            for (k, v) in style_opts {
                opts.insert(k.clone(), v.clone());
            }
        }
    }
    // Meter's own options override style
    for (k, v) in &sec.options {
        opts.insert(k.clone(), v.clone());
    }

    let meter_type = opts.get("Meter")?.to_ascii_lowercase();

    // Parse positions with relative positioning support
    let x = parse_position(
        opts.get("X").map(|s| s.as_str()).unwrap_or("0"),
        prev_pos.x, prev_pos.w, warnings,
    );
    let y = parse_position(
        opts.get("Y").map(|s| s.as_str()).unwrap_or("0"),
        prev_pos.y, prev_pos.h, warnings,
    );
    let w_str = opts.get("W").map(|s| s.as_str()).unwrap_or("");
    let h_str = opts.get("H").map(|s| s.as_str()).unwrap_or("");

    let hidden = opts.get("Hidden").map(|v| v == "1").unwrap_or(false);

    // Collect all MeasureName, MeasureName2, MeasureName3, etc.
    let mut measure_names: Vec<Option<String>> = Vec::new();
    measure_names.push(opts.get("MeasureName").cloned());
    for i in 2..=10 {
        let key = format!("MeasureName{}", i);
        measure_names.push(opts.get(&key).cloned());
    }
    let measure_name = opts.get("MeasureName").map(|s| s.as_str());

    let (layer_type, mut props) = match meter_type.as_str() {
        "string" => {
            let raw_text = opts.get("Text").map(|s| s.as_str()).unwrap_or("");
            let text = resolve_text_content(raw_text, &measure_names, measures);
            let font_face = opts
                .get("FontFace")
                .cloned()
                .unwrap_or_else(|| "Arial".to_string());
            let font_size = parse_numeric(
                opts.get("FontSize").map(|s| s.as_str()).unwrap_or("10"),
                warnings,
            );
            let font_color = opts
                .get("FontColor")
                .map(|s| convert_color(s))
                .unwrap_or_else(|| "#000000".to_string());
            let text_align = opts.get("StringAlign").map(|sa| {
                match sa.to_ascii_lowercase().as_str() {
                    "center" | "centertop" | "centercenter" | "centerbottom" => TextAlign::Center,
                    "right" | "righttop" | "rightcenter" | "rightbottom" => TextAlign::Right,
                    _ => TextAlign::Left,
                }
            });

            // Shadow
            let shadow = if opts
                .get("StringEffect")
                .map(|s| s.to_ascii_lowercase() == "shadow")
                .unwrap_or(false)
            {
                let shadow_color = opts
                    .get("FontEffectColor")
                    .map(|s| convert_color(s))
                    .unwrap_or_else(|| "#00000080".to_string());
                Some(Shadow {
                    color: shadow_color,
                    dx: 1.0,
                    dy: 1.0,
                    radius: 2.0,
                })
            } else {
                None
            };

            // Rotation: Rainmeter Angle is in radians
            let rotation = opts.get("Angle").map(|a| {
                let rad = parse_numeric(a, warnings);
                NumberOrString::Number(rad * 180.0 / std::f64::consts::PI)
            });

            // Default width/height for text
            let w = if w_str.is_empty() {
                400.0
            } else {
                parse_numeric(w_str, warnings)
            };
            let h = if h_str.is_empty() {
                font_size * 1.5
            } else {
                parse_numeric(h_str, warnings)
            };

            let props = LayerProperties {
                x: NumberOrString::Number(x),
                y: NumberOrString::Number(y),
                width: NumberOrString::Number(w),
                height: NumberOrString::Number(h),
                rotation,
                text: Some(text),
                font_size: Some(NumberOrString::Number(font_size)),
                font_family: Some(font_face),
                color: Some(font_color),
                text_align,
                shadow,
                ..LayerProperties::default()
            };
            (LayerType::Text, props)
        }

        "image" => {
            // Determine image name: ImageName key, or measure value if MeasureName used without ImageName
            let has_image_name = opts.contains_key("ImageName");
            let src_raw = if has_image_name {
                let img_name = opts.get("ImageName").cloned().unwrap_or_default();
                // If ImageName contains %1, substitute with measure value
                if img_name.contains("%1") {
                    if let Some(mn) = measure_name {
                        if let Some(mapped) = measures.get(mn) {
                            img_name.replace("%1", mapped)
                        } else {
                            img_name.replace("%1", mn)
                        }
                    } else {
                        img_name
                    }
                } else {
                    img_name
                }
            } else if let Some(mn) = measure_name {
                // No ImageName: use measure's string value as filename
                if let Some(mapped) = measures.get(mn) {
                    // If mapped value contains { } placeholder, it's unresolved — skip
                    if mapped.starts_with('{') && mapped.ends_with('}') {
                        String::new()
                    } else {
                        mapped.clone()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // Meter-level ImagePath overrides global
            let meter_image_path = opts.get("ImagePath").map(|s| s.as_str());

            let src = resolve_image_path(
                &src_raw,
                meter_image_path,
                global_image_path,
                asset_dir,
                skin_files_dir,
                skin_subdir,
            );

            let scale_mode = opts.get("PreserveAspectRatio").map(|v| match v.as_str() {
                "0" => ScaleMode::Stretch,
                "2" => ScaleMode::Fill,
                _ => ScaleMode::Fit,
            });

            let tint = opts.get("ImageTint").map(|s| convert_color(s));

            let w = if w_str.is_empty() {
                100.0
            } else {
                parse_numeric(w_str, warnings)
            };
            let h = if h_str.is_empty() {
                100.0
            } else {
                parse_numeric(h_str, warnings)
            };

            let props = LayerProperties {
                x: NumberOrString::Number(x),
                y: NumberOrString::Number(y),
                width: NumberOrString::Number(w),
                height: NumberOrString::Number(h),
                src: if src.is_empty() { None } else { Some(src) },
                scale_mode,
                tint,
                ..LayerProperties::default()
            };
            (LayerType::Image, props)
        }

        "shape" => {
            let shape_str = opts.get("Shape")?;
            let parsed = parse_shape_string(shape_str, warnings)?;

            let w = if w_str.is_empty() {
                parsed.width
            } else {
                parse_numeric(w_str, warnings)
            };
            let h = if h_str.is_empty() {
                parsed.height
            } else {
                parse_numeric(h_str, warnings)
            };

            let props = LayerProperties {
                x: NumberOrString::Number(x + parsed.x),
                y: NumberOrString::Number(y + parsed.y),
                width: NumberOrString::Number(w),
                height: NumberOrString::Number(h),
                shape_kind: Some(parsed.kind),
                fill: parsed.fill,
                stroke: parsed.stroke,
                stroke_width: parsed.stroke_width,
                corner_radius: parsed.corner_radius,
                ..LayerProperties::default()
            };
            (LayerType::Shape, props)
        }

        "bar" => {
            let bar_color = opts
                .get("BarColor")
                .map(|s| convert_color(s))
                .unwrap_or_else(|| "#FFFFFF".to_string());
            let track_color = opts
                .get("SolidColor")
                .map(|s| convert_color(s));
            let orientation = opts.get("BarOrientation").map(|o| {
                if o.to_ascii_lowercase() == "vertical" {
                    Orientation::Vertical
                } else {
                    Orientation::Horizontal
                }
            });

            let value = if let Some(mn) = measure_name {
                if let Some(mapped) = measures.get(mn) {
                    Some(NumberOrString::String(mapped.clone()))
                } else {
                    Some(NumberOrString::String(format!("{{{}}}", mn)))
                }
            } else {
                None
            };

            let w = if w_str.is_empty() {
                100.0
            } else {
                parse_numeric(w_str, warnings)
            };
            let h = if h_str.is_empty() {
                20.0
            } else {
                parse_numeric(h_str, warnings)
            };

            let props = LayerProperties {
                x: NumberOrString::Number(x),
                y: NumberOrString::Number(y),
                width: NumberOrString::Number(w),
                height: NumberOrString::Number(h),
                color: Some(bar_color),
                style: Some(ProgressStyle::Bar),
                orientation,
                min: Some(0.0),
                max: Some(100.0),
                value,
                track_color,
                ..LayerProperties::default()
            };
            (LayerType::Progress, props)
        }

        _ => {
            warnings.push(format!(
                "Unsupported meter type '{}' for [{}]",
                meter_type, sec.name
            ));
            return None;
        }
    };

    if hidden {
        props.visible = Some(BoolOrString::Bool(false));
    }

    Some(Layer {
        id: next_id(),
        name: sec.name.clone(),
        layer_type,
        properties: props,
        animations: None,
        children: None,
        locked: None,
        visible: if hidden { Some(false) } else { None },
    })
}

// ---------------------------------------------------------------------------
// @Include resolution
// ---------------------------------------------------------------------------

/// Resolve @Include directives in a parsed INI.
/// Reads included files from the ZIP archive relative to the skin directory.
fn resolve_includes(
    sections: &mut Vec<IniSection>,
    archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
    skin_dir: &str,
    resources_dir: &str,
    warnings: &mut Vec<String>,
) {
    // Collect include paths from all sections
    let mut includes: Vec<(usize, String, String)> = Vec::new(); // (section_index, key, path)
    for (idx, sec) in sections.iter().enumerate() {
        for (key, val) in &sec.options {
            if key.starts_with("@Include") || key.starts_with("@include") {
                let path = val.replace("#@#", resources_dir);
                let path = path.replace('\\', "/");
                includes.push((idx, key.clone(), path));
            }
        }
    }

    for (_, _, path) in &includes {
        let full_path = if path.starts_with(skin_dir) {
            path.clone()
        } else {
            format!("{}/{}", skin_dir, path.trim_start_matches('/'))
        };

        match archive.by_name(&full_path) {
            Ok(mut file) => {
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    let text = decode_ini_bytes(&buf);
                    let included = parse_ini(&text);
                    // Merge included sections: add new sections, merge into existing ones
                    for inc_sec in included {
                        if let Some(existing) = sections.iter_mut().find(|s| s.name == inc_sec.name)
                        {
                            for (k, v) in inc_sec.options {
                                existing.options.entry(k).or_insert(v);
                            }
                        } else {
                            sections.push(inc_sec);
                        }
                    }
                }
            }
            Err(_) => {
                warnings.push(format!("Could not read included file: {}", full_path));
            }
        }
    }

    // Remove @Include keys from sections
    for sec in sections.iter_mut() {
        sec.options
            .retain(|k, _| !k.starts_with("@Include") && !k.starts_with("@include"));
    }
}

// ---------------------------------------------------------------------------
// Globals from Variables
// ---------------------------------------------------------------------------

fn extract_globals(vars: &HashMap<String, String>) -> Vec<GlobalVariable> {
    let skip_patterns = ["URL", "RegExp", "APIKey", "Key", "@Include", "@include"];
    let mut globals = Vec::new();

    for (name, value) in vars {
        // Skip internal-looking variables
        if skip_patterns.iter().any(|pat| name.contains(pat)) {
            continue;
        }
        if value.len() > 200 {
            continue;
        }
        if name.starts_with('@') {
            continue;
        }

        let (var_type, var_value) = if is_rm_color(value) {
            (
                GlobalVarType::Color,
                GlobalVarValue::String(convert_color(value)),
            )
        } else if let Ok(n) = value.parse::<f64>() {
            (GlobalVarType::Number, GlobalVarValue::Number(n))
        } else {
            (GlobalVarType::Text, GlobalVarValue::String(value.clone()))
        };

        globals.push(GlobalVariable {
            name: name.clone(),
            var_type,
            value: var_value,
            options: None,
        });
    }

    // Sort for deterministic output
    globals.sort_by(|a, b| a.name.cmp(&b.name));
    globals
}

// ---------------------------------------------------------------------------
// Skin processing
// ---------------------------------------------------------------------------

/// Process a single skin INI file: parse, resolve includes, substitute variables,
/// map measures, and convert meters to layers.
fn process_skin_ini(
    ini_data: &[u8],
    _skin_name: &str,
    skin_dir: &str,
    skin_root: &str,
    skin_subdir: &str,
    archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
    asset_dir: &str,
    skin_files_dir: &str,
    all_warnings: &mut Vec<String>,
) -> (Vec<Layer>, HashMap<String, String>) {
    let text = decode_ini_bytes(ini_data);
    let mut sections = parse_ini(&text);

    // #@# always refers to the root skin's @Resources/ directory
    let resources_dir = format!("{}/@Resources/", skin_root.trim_end_matches('/'));

    // Resolve @Include directives
    resolve_includes(
        &mut sections,
        archive,
        skin_dir,
        &resources_dir,
        all_warnings,
    );

    // Collect variables from [Variables] section
    let mut vars: HashMap<String, String> = HashMap::new();
    for sec in &sections {
        if sec.name.to_ascii_lowercase() == "variables" {
            for (k, v) in &sec.options {
                vars.insert(k.clone(), v.clone());
            }
        }
    }

    // Substitute variables in all section values
    for sec in sections.iter_mut() {
        let new_opts: HashMap<String, String> = sec
            .options
            .iter()
            .map(|(k, v)| (k.clone(), substitute_variables(v, &vars)))
            .collect();
        sec.options = new_opts;
    }

    // Re-read variables after substitution (variables can reference other variables)
    for sec in &sections {
        if sec.name.to_ascii_lowercase() == "variables" {
            for (k, v) in &sec.options {
                vars.insert(k.clone(), v.clone());
            }
        }
    }

    // Get ImagePath from [Rainmeter] section
    let image_path = sections
        .iter()
        .find(|s| s.name.to_ascii_lowercase() == "rainmeter")
        .and_then(|s| s.options.get("ImagePath"))
        .cloned()
        .unwrap_or_default();

    // Build styles map
    let mut styles: HashMap<String, HashMap<String, String>> = HashMap::new();
    for sec in &sections {
        // A style section has no Meter= key but is referenced by MeterStyle
        if !sec.options.contains_key("Meter") && !sec.options.contains_key("Measure") {
            styles.insert(sec.name.clone(), sec.options.clone());
        }
    }

    // Map measures
    let measures = map_measures(&sections, &vars, all_warnings);

    // Convert meters to layers (preserving order, tracking position for relative layout)
    let mut layers: Vec<Layer> = Vec::new();
    let mut prev_pos = MeterPos::default();
    for sec in &sections {
        if sec.options.contains_key("Meter") {
            if let Some(layer) = convert_meter(
                sec,
                &styles,
                &measures,
                &image_path,
                asset_dir,
                skin_files_dir,
                skin_subdir,
                &prev_pos,
                all_warnings,
            ) {
                // Update previous meter position for relative positioning
                if let NumberOrString::Number(lx) = &layer.properties.x {
                    prev_pos.x = *lx;
                }
                if let NumberOrString::Number(ly) = &layer.properties.y {
                    prev_pos.y = *ly;
                }
                if let NumberOrString::Number(lw) = &layer.properties.width {
                    prev_pos.w = *lw;
                }
                if let NumberOrString::Number(lh) = &layer.properties.height {
                    prev_pos.h = *lh;
                }
                layers.push(layer);
            }
        }
    }

    (layers, vars)
}

// ---------------------------------------------------------------------------
// Asset extraction
// ---------------------------------------------------------------------------

/// Extract assets from the .rmskin ZIP to output_dir/.
/// - `@Resources/` files → `output_dir/assets/` (strip @Resources/ prefix)
/// - Other skin files (images in subdirs) → `output_dir/skin/` (preserve subdir structure)
fn extract_assets(
    archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
    skin_prefix: &str,
    output_dir: &str,
) -> Result<usize, String> {
    let asset_dir = Path::new(output_dir).join("assets");
    let skin_files_dir = Path::new(output_dir).join("skin");
    std::fs::create_dir_all(&asset_dir)
        .map_err(|e| format!("Failed to create asset dir: {}", e))?;
    std::fs::create_dir_all(&skin_files_dir)
        .map_err(|e| format!("Failed to create skin files dir: {}", e))?;

    let skin_prefix_slash = format!("{}/", skin_prefix.trim_end_matches('/'));
    let resources_prefix = format!("{}@Resources/", &skin_prefix_slash);
    let mut count = 0;

    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    // Skip these extensions — they're not assets
    let skip_ext = [".ini", ".inc", ".txt", ".dll"];

    for name in &names {
        if name.ends_with('/') {
            continue;
        }

        let dest = if name.starts_with(&resources_prefix) {
            // @Resources/ files → assets/ (strip @Resources/)
            let rel = &name[resources_prefix.len()..];
            asset_dir.join(rel.replace('\\', "/"))
        } else if name.starts_with(&skin_prefix_slash) {
            // Other skin files → skin/ (preserve subdirectory structure)
            let rel = &name[skin_prefix_slash.len()..];
            let lower = rel.to_ascii_lowercase();
            // Skip non-asset files
            if skip_ext.iter().any(|ext| lower.ends_with(ext)) {
                continue;
            }
            // Skip @Resources files (already handled above)
            if lower.starts_with("@resources/") {
                continue;
            }
            skin_files_dir.join(rel.replace('\\', "/"))
        } else {
            continue;
        };

        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = archive.by_name(name) {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                if std::fs::write(&dest, &buf).is_ok() {
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// RMSKIN.ini parsing
// ---------------------------------------------------------------------------

/// Read the skin name from RMSKIN.ini inside the archive.
fn read_rmskin_ini(
    archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
) -> Option<String> {
    let mut buf = Vec::new();
    archive
        .by_name("RMSKIN.ini")
        .ok()?
        .read_to_end(&mut buf)
        .ok()?;
    let text = decode_ini_bytes(&buf);
    let sections = parse_ini(&text);
    for sec in &sections {
        if let Some(name) = sec.options.get("Name") {
            return Some(name.clone());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Import a Rainmeter `.rmskin` file and convert it to our Layer tree.
///
/// The `.rmskin` format is a ZIP archive containing skin INI files and assets.
/// This function parses the INI files, converts meters to layers, extracts assets,
/// and returns a `KompImportResult` compatible with the rest of the application.
pub fn import_rmskin_file(rmskin_path: &str, output_dir: &str) -> Result<KompImportResult, String> {
    reset_counter();

    let file_data =
        std::fs::read(rmskin_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let cursor = std::io::Cursor::new(file_data);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open ZIP: {}", e))?;

    let mut warnings: Vec<String> = Vec::new();
    let asset_dir_path = Path::new(output_dir).join("assets");
    let asset_dir = asset_dir_path.to_string_lossy().to_string();

    // Read skin name from RMSKIN.ini
    let skin_name = read_rmskin_ini(&mut archive).unwrap_or_else(|| "Rainmeter Skin".to_string());

    // Find all skin INI files under Skins/
    let all_names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    let skin_inis: Vec<String> = all_names
        .iter()
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            lower.starts_with("skins/")
                && lower.ends_with(".ini")
                && !lower.contains("/@resources/")
                && !lower.contains("/@resources\\")
        })
        .cloned()
        .collect();

    if skin_inis.is_empty() {
        // Fall back: look for any .ini file not named RMSKIN.ini
        let fallback_inis: Vec<String> = all_names
            .iter()
            .filter(|name| {
                let lower = name.to_ascii_lowercase();
                lower.ends_with(".ini") && lower != "rmskin.ini"
            })
            .cloned()
            .collect();

        if fallback_inis.is_empty() {
            return Err("No skin INI files found in archive".to_string());
        }

        // Process fallback INIs
        return process_ini_list(&fallback_inis, &skin_name, &mut archive, output_dir, &asset_dir, &mut warnings);
    }

    process_ini_list(&skin_inis, &skin_name, &mut archive, output_dir, &asset_dir, &mut warnings)
}

/// Process a list of INI files from the archive and produce the import result.
fn process_ini_list(
    ini_paths: &[String],
    skin_name: &str,
    archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
    output_dir: &str,
    asset_dir: &str,
    warnings: &mut Vec<String>,
) -> Result<KompImportResult, String> {
    let mut all_layers: Vec<Layer> = Vec::new();
    let mut all_vars: HashMap<String, String> = HashMap::new();
    let mut skin_prefix = String::new();
    let skin_files_dir = Path::new(output_dir).join("skin").to_string_lossy().to_string();

    for ini_path in ini_paths {
        // Read INI data
        let ini_data = {
            let mut buf = Vec::new();
            match archive.by_name(ini_path) {
                Ok(mut file) => {
                    file.read_to_end(&mut buf)
                        .map_err(|e| format!("Failed to read {}: {}", ini_path, e))?;
                }
                Err(e) => {
                    warnings.push(format!("Could not read {}: {}", ini_path, e));
                    continue;
                }
            }
            buf
        };

        // Determine skin directory (parent of the INI file within the ZIP)
        let skin_dir = Path::new(ini_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // Determine skin prefix for asset extraction (e.g., "Skins/MySkin")
        if skin_prefix.is_empty() {
            // Take the first path component under Skins/ as the skin root
            let parts: Vec<&str> = ini_path.split('/').collect();
            if parts.len() >= 2 {
                skin_prefix = format!("{}/{}", parts[0], parts[1]);
            } else {
                skin_prefix = skin_dir.clone();
            }
        }

        // Compute skin_subdir: the subdirectory of the INI relative to skin root
        // e.g., for "Skins/EOS/LAUNCHERS/LAUNCHER.ini" with skin_prefix="Skins/EOS",
        // skin_subdir = "LAUNCHERS"
        let skin_subdir = if !skin_prefix.is_empty() && skin_dir.starts_with(&skin_prefix) {
            let rel = skin_dir[skin_prefix.len()..].trim_start_matches('/');
            rel.to_string()
        } else {
            String::new()
        };

        let ini_name = Path::new(ini_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Skin".to_string());

        // Use parent folder name for the container if it differs from the INI name
        let display_name = if !skin_subdir.is_empty() {
            // Use the last component of the subdir path as the display name
            skin_subdir.rsplit('/').next().unwrap_or(&ini_name).to_string()
        } else {
            ini_name.clone()
        };

        let (layers, vars) = process_skin_ini(
            &ini_data,
            &ini_name,
            &skin_dir,
            &skin_prefix,
            &skin_subdir,
            archive,
            asset_dir,
            &skin_files_dir,
            warnings,
        );

        all_vars.extend(vars);

        if layers.is_empty() {
            continue;
        }

        if ini_paths.len() == 1 {
            all_layers = layers;
        } else {
            // Wrap in an Overlap container per skin INI
            let container = Layer {
                id: next_id(),
                name: display_name,
                layer_type: LayerType::Overlap,
                properties: LayerProperties::default(),
                animations: None,
                children: Some(layers),
                locked: None,
                visible: None,
            };
            all_layers.push(container);
        }
    }

    // Extract assets
    let asset_count = extract_assets(archive, &skin_prefix, output_dir).unwrap_or_else(|e| {
        warnings.push(format!("Asset extraction error: {}", e));
        0
    });

    // Calculate bounding box for root container
    let (max_w, max_h) = compute_bounding_box(&all_layers);

    // Build root container
    let root = Layer {
        id: next_id(),
        name: skin_name.to_string(),
        layer_type: LayerType::Overlap,
        properties: LayerProperties {
            x: NumberOrString::Number(0.0),
            y: NumberOrString::Number(0.0),
            width: NumberOrString::Number(max_w),
            height: NumberOrString::Number(max_h),
            ..LayerProperties::default()
        },
        animations: None,
        children: Some(all_layers),
        locked: None,
        visible: None,
    };

    let globals = extract_globals(&all_vars);

    Ok(KompImportResult {
        root,
        globals,
        warnings: warnings.clone(),
        asset_count,
        asset_dir: asset_dir.to_string(),
    })
}

/// Compute the bounding box of a set of layers.
fn compute_bounding_box(layers: &[Layer]) -> (f64, f64) {
    let mut max_w: f64 = 0.0;
    let mut max_h: f64 = 0.0;

    for layer in layers {
        let x = match &layer.properties.x {
            NumberOrString::Number(n) => *n,
            _ => 0.0,
        };
        let y = match &layer.properties.y {
            NumberOrString::Number(n) => *n,
            _ => 0.0,
        };
        let w = match &layer.properties.width {
            NumberOrString::Number(n) => *n,
            _ => 0.0,
        };
        let h = match &layer.properties.height {
            NumberOrString::Number(n) => *n,
            _ => 0.0,
        };
        max_w = max_w.max(x + w);
        max_h = max_h.max(y + h);

        // Recurse into children
        if let Some(children) = &layer.children {
            let (cw, ch) = compute_bounding_box(children);
            max_w = max_w.max(cw);
            max_h = max_h.max(ch);
        }
    }

    // Ensure minimum size
    if max_w < 1.0 {
        max_w = 800.0;
    }
    if max_h < 1.0 {
        max_h = 600.0;
    }

    (max_w, max_h)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ini_basic() {
        let text = r#"
[Rainmeter]
Update=1000
Background=#@#bg.png

[Variables]
Size=14
Color=255,255,255,200

; This is a comment
[MeterClock]
Meter=String
X=100
Y=50
FontSize=#Size#
Text=%1
"#;
        let sections = parse_ini(text);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].name, "Rainmeter");
        assert_eq!(sections[0].options.get("Update"), Some(&"1000".to_string()));
        assert_eq!(sections[1].name, "Variables");
        assert_eq!(sections[1].options.get("Size"), Some(&"14".to_string()));
        assert_eq!(sections[2].name, "MeterClock");
        assert_eq!(sections[2].options.get("Meter"), Some(&"String".to_string()));
        assert_eq!(sections[2].options.get("X"), Some(&"100".to_string()));
    }

    #[test]
    fn test_parse_ini_comment_skipped() {
        let text = "[Sec]\n; comment\nKey=Val\n";
        let sections = parse_ini(text);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].options.len(), 1);
        assert_eq!(sections[0].options.get("Key"), Some(&"Val".to_string()));
    }

    #[test]
    fn test_convert_color_rgb() {
        assert_eq!(convert_color("255,0,128"), "#FF0080");
    }

    #[test]
    fn test_convert_color_rgba_opaque() {
        assert_eq!(convert_color("0,128,255,255"), "#0080FF");
    }

    #[test]
    fn test_convert_color_rgba_transparent() {
        assert_eq!(convert_color("0,128,255,128"), "#0080FF80");
    }

    #[test]
    fn test_convert_color_spaces() {
        assert_eq!(convert_color("255, 0, 128, 200"), "#FF0080C8");
    }

    #[test]
    fn test_is_rm_color() {
        assert!(is_rm_color("255,0,128"));
        assert!(is_rm_color("255, 0, 128, 200"));
        assert!(!is_rm_color("hello"));
        assert!(!is_rm_color("#FF0000"));
        assert!(!is_rm_color("255,0"));
    }

    #[test]
    fn test_eval_expr_simple() {
        assert_eq!(eval_expr("42"), Some(42.0));
        assert_eq!(eval_expr("3+4"), Some(7.0));
        assert_eq!(eval_expr("10-3"), Some(7.0));
        assert_eq!(eval_expr("6*7"), Some(42.0));
        assert_eq!(eval_expr("84/2"), Some(42.0));
    }

    #[test]
    fn test_eval_expr_precedence() {
        assert_eq!(eval_expr("2+3*4"), Some(14.0));
        assert_eq!(eval_expr("(2+3)*4"), Some(20.0));
    }

    #[test]
    fn test_eval_expr_nested_parens() {
        assert_eq!(eval_expr("((2+3))"), Some(5.0));
        assert_eq!(eval_expr("(1+(2*3))"), Some(7.0));
    }

    #[test]
    fn test_eval_expr_negative() {
        assert_eq!(eval_expr("-5"), Some(-5.0));
        assert_eq!(eval_expr("-5+10"), Some(5.0));
        assert_eq!(eval_expr("10*-2"), Some(-20.0));
    }

    #[test]
    fn test_eval_expr_decimals() {
        let result = eval_expr("0.26*256");
        assert!(result.is_some());
        assert!((result.unwrap() - 66.56).abs() < 0.001);
    }

    #[test]
    fn test_eval_expr_invalid() {
        assert_eq!(eval_expr("abc"), None);
        assert_eq!(eval_expr(""), None);
        assert_eq!(eval_expr("5/0"), None);
    }

    #[test]
    fn test_substitute_variables() {
        let mut vars = HashMap::new();
        vars.insert("Size".to_string(), "14".to_string());
        vars.insert("Color".to_string(), "255,255,255".to_string());

        assert_eq!(substitute_variables("#Size#", &vars), "14");
        assert_eq!(
            substitute_variables("Color=#Color#", &vars),
            "Color=255,255,255"
        );
        assert_eq!(
            substitute_variables("#Unknown#", &vars),
            "#Unknown#"
        );
        assert_eq!(substitute_variables("no vars here", &vars), "no vars here");
    }

    #[test]
    fn test_substitute_variables_at_sign() {
        let vars = HashMap::new();
        assert_eq!(substitute_variables("#@#path", &vars), "#@#path");
    }

    #[test]
    fn test_parse_numeric_simple() {
        let mut w = Vec::new();
        assert_eq!(parse_numeric("100", &mut w), 100.0);
        assert!(w.is_empty());
    }

    #[test]
    fn test_parse_numeric_relative() {
        let mut w = Vec::new();
        assert_eq!(parse_numeric("50r", &mut w), 50.0);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("Relative"));
    }

    #[test]
    fn test_parse_numeric_expression() {
        let mut w = Vec::new();
        assert!((parse_numeric("(10+5)", &mut w) - 15.0).abs() < 0.001);
        assert!(w.is_empty());
    }

    #[test]
    fn test_decode_ini_bytes_utf8() {
        let text = "[Section]\nKey=Value\n";
        let decoded = decode_ini_bytes(text.as_bytes());
        assert!(decoded.contains("[Section]"));
    }

    #[test]
    fn test_decode_ini_bytes_utf8_bom() {
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(b"[Section]\nKey=Value\n");
        let decoded = decode_ini_bytes(&data);
        assert!(decoded.contains("[Section]"));
    }

    #[test]
    fn test_decode_ini_bytes_utf16le() {
        let text = "[S]\nK=V\n";
        let mut data = vec![0xFF, 0xFE]; // BOM
        for ch in text.encode_utf16() {
            data.push((ch & 0xFF) as u8);
            data.push((ch >> 8) as u8);
        }
        let decoded = decode_ini_bytes(&data);
        assert!(decoded.contains("[S]"));
        assert!(decoded.contains("K=V"));
    }

    #[test]
    fn test_convert_time_format() {
        assert_eq!(convert_time_format("%H:%M"), "HH:mm");
        assert_eq!(convert_time_format("%I:%M %p"), "hh:mm a");
        assert_eq!(convert_time_format("%A, %B %d"), "EEEE, MMMM dd");
    }

    #[test]
    fn test_parse_shape_rectangle() {
        let mut w = Vec::new();
        let shape = parse_shape_string(
            "Rectangle 0,0,200,100,10 | Fill Color 255,255,255,200 | StrokeWidth 2 | Stroke Color 0,0,0,255",
            &mut w,
        );
        assert!(shape.is_some());
        let s = shape.unwrap();
        assert!(matches!(s.kind, ShapeKind::Rectangle));
        assert_eq!(s.width, 200.0);
        assert_eq!(s.height, 100.0);
        assert_eq!(s.corner_radius, Some(10.0));
        assert_eq!(s.fill, Some("#FFFFFFC8".to_string()));
        assert_eq!(s.stroke, Some("#000000".to_string()));
        assert_eq!(s.stroke_width, Some(2.0));
    }

    #[test]
    fn test_parse_shape_ellipse() {
        let mut w = Vec::new();
        let shape = parse_shape_string("Ellipse 50,50,50 | Fill Color 200,100,50", &mut w);
        assert!(shape.is_some());
        let s = shape.unwrap();
        assert!(matches!(s.kind, ShapeKind::Circle));
        assert_eq!(s.width, 100.0);
        assert_eq!(s.height, 100.0);
    }

    #[test]
    fn test_parse_shape_ellipse_oval() {
        let mut w = Vec::new();
        let shape = parse_shape_string("Ellipse 100,50,80,40", &mut w);
        assert!(shape.is_some());
        let s = shape.unwrap();
        assert!(matches!(s.kind, ShapeKind::Oval));
        assert_eq!(s.width, 160.0);
        assert_eq!(s.height, 80.0);
    }

    #[test]
    fn test_extract_globals_filters() {
        let mut vars = HashMap::new();
        vars.insert("FontSize".to_string(), "14".to_string());
        vars.insert("AccentColor".to_string(), "255,100,50".to_string());
        vars.insert("APIKey".to_string(), "secret123".to_string());
        vars.insert("@Include".to_string(), "file.inc".to_string());
        vars.insert("LongValue".to_string(), "x".repeat(201));

        let globals = extract_globals(&vars);

        let names: Vec<&str> = globals.iter().map(|g| g.name.as_str()).collect();
        assert!(names.contains(&"FontSize"));
        assert!(names.contains(&"AccentColor"));
        assert!(!names.contains(&"APIKey"));
        assert!(!names.contains(&"@Include"));
        assert!(!names.contains(&"LongValue"));
    }

    #[test]
    fn test_extract_globals_types() {
        let mut vars = HashMap::new();
        vars.insert("Size".to_string(), "14".to_string());
        vars.insert("Color".to_string(), "255,100,50".to_string());
        vars.insert("Name".to_string(), "Hello".to_string());

        let globals = extract_globals(&vars);

        let size_g = globals.iter().find(|g| g.name == "Size").unwrap();
        assert!(matches!(size_g.var_type, GlobalVarType::Number));

        let color_g = globals.iter().find(|g| g.name == "Color").unwrap();
        assert!(matches!(color_g.var_type, GlobalVarType::Color));
        if let GlobalVarValue::String(ref s) = color_g.value {
            assert_eq!(s, "#FF6432");
        } else {
            panic!("Expected string value for color");
        }

        let name_g = globals.iter().find(|g| g.name == "Name").unwrap();
        assert!(matches!(name_g.var_type, GlobalVarType::Text));
    }

    #[test]
    fn test_resolve_text_content() {
        let mut measures = HashMap::new();
        measures.insert("MeasureTime".to_string(), "$df(HH:mm)$".to_string());
        measures.insert("MeasureCPU".to_string(), "$si(cpuutil)$".to_string());
        measures.insert("MeasureDate".to_string(), "$df(dd)$".to_string());

        let no_measures: Vec<Option<String>> = vec![None];
        let with_time = vec![Some("MeasureTime".to_string())];
        let with_two = vec![
            Some("MeasureTime".to_string()),
            Some("MeasureDate".to_string()),
        ];

        assert_eq!(
            resolve_text_content("[MeasureTime]", &no_measures, &measures),
            "$df(HH:mm)$"
        );
        assert_eq!(
            resolve_text_content("CPU: [MeasureCPU]%", &no_measures, &measures),
            "CPU: $si(cpuutil)$%"
        );
        assert_eq!(
            resolve_text_content("%1", &with_time, &measures),
            "$df(HH:mm)$"
        );
        assert_eq!(
            resolve_text_content("[Unknown]", &no_measures, &measures),
            "{Unknown}"
        );
        // Test multiple measure names (%1 and %2)
        assert_eq!(
            resolve_text_content("%1 - %2", &with_two, &measures),
            "$df(HH:mm)$ - $df(dd)$"
        );
    }
}
