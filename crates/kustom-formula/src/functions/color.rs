use crate::evaluator::EvalContext;
use crate::parser::Expr;
use crate::value::Value;

/// Evaluate `ce(color, filter, [amount])` - color editing.
pub fn eval_ce(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.len() < 2 {
        return Value::Color("#000000".to_string());
    }

    let color_str = ctx.evaluate(&args[0]).as_text();
    let filter = ctx.evaluate(&args[1]).as_text().to_lowercase();
    let amount = args.get(2).map(|a| ctx.evaluate(a).as_number()).unwrap_or(0.0);

    let (a, r, g, b) = parse_color(&color_str);

    let result = match filter.as_str() {
        "invert" => {
            format_color(a, 255 - r, 255 - g, 255 - b)
        }
        "comp" => {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let new_h = (h + 180.0) % 360.0;
            let (nr, ng, nb) = hsl_to_rgb(new_h, s, l);
            format_color(a, nr, ng, nb)
        }
        "contrast" => {
            let luminance = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
            if luminance > 128.0 {
                format_color(255, 0, 0, 0) // black
            } else {
                format_color(255, 255, 255, 255) // white
            }
        }
        "alpha" => {
            let new_alpha = ((amount / 100.0) * 255.0).clamp(0.0, 255.0) as u8;
            format_color(new_alpha, r, g, b)
        }
        "sat" => {
            let (h, _s, l) = rgb_to_hsl(r, g, b);
            let new_s = (amount / 100.0).clamp(0.0, 1.0);
            let (nr, ng, nb) = hsl_to_rgb(h, new_s, l);
            format_color(a, nr, ng, nb)
        }
        "lum" => {
            let (h, s, _l) = rgb_to_hsl(r, g, b);
            let new_l = (amount / 100.0).clamp(0.0, 1.0);
            let (nr, ng, nb) = hsl_to_rgb(h, s, new_l);
            format_color(a, nr, ng, nb)
        }
        _ if filter.starts_with('#') => {
            // Gradient mix: ce(color1, color2, percent)
            // percent 0 = color1, 100 = color2
            let (a2, r2, g2, b2) = parse_color(&filter);
            let t = (amount / 100.0).clamp(0.0, 1.0);
            let mix = |c1: u8, c2: u8| -> u8 {
                ((c1 as f64) * (1.0 - t) + (c2 as f64) * t).round() as u8
            };
            format_color(mix(a, a2), mix(r, r2), mix(g, g2), mix(b, b2))
        }
        _ => color_str,
    };

    Value::Color(result)
}

/// Evaluate `cm(h, s, l)` - create color from HSL.
pub fn eval_cm(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.len() < 3 {
        return Value::Color("#000000".to_string());
    }

    let h = ctx.evaluate(&args[0]).as_number();
    let s = ctx.evaluate(&args[1]).as_number() / 100.0;
    let l = ctx.evaluate(&args[2]).as_number() / 100.0;

    let (r, g, b) = hsl_to_rgb(h, s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    Value::Color(format_color(255, r, g, b))
}

/// Parse a hex color string into (alpha, red, green, blue) components.
fn parse_color(s: &str) -> (u8, u8, u8, u8) {
    let s = s.trim_start_matches('#');
    match s.len() {
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
            (255, r, g, b)
        }
        8 => {
            let a = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
            let r = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
            let g = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[6..8], 16).unwrap_or(0);
            (a, r, g, b)
        }
        _ => (255, 0, 0, 0),
    }
}

fn format_color(a: u8, r: u8, g: u8, b: u8) -> String {
    if a == 255 {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", a, r, g, b)
    }
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f64::EPSILON {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h * 60.0, s, l)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    if s.abs() < f64::EPSILON {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h = h / 360.0;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_6_digit_color() {
        assert_eq!(parse_color("#FF0000"), (255, 255, 0, 0));
    }

    #[test]
    fn test_parse_8_digit_color() {
        assert_eq!(parse_color("#80FF0000"), (128, 255, 0, 0));
    }

    #[test]
    fn test_invert() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("#FF0000".into())),
            Expr::Literal(Value::Text("invert".into())),
        ];
        assert_eq!(eval_ce(&args, &ctx).as_text(), "#00FFFF");
    }

    #[test]
    fn test_contrast_dark() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("#000000".into())),
            Expr::Literal(Value::Text("contrast".into())),
        ];
        assert_eq!(eval_ce(&args, &ctx).as_text(), "#FFFFFF");
    }

    #[test]
    fn test_contrast_light() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("#FFFFFF".into())),
            Expr::Literal(Value::Text("contrast".into())),
        ];
        assert_eq!(eval_ce(&args, &ctx).as_text(), "#000000");
    }

    #[test]
    fn test_alpha() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("#FF0000".into())),
            Expr::Literal(Value::Text("alpha".into())),
            Expr::Literal(Value::Number(50.0)),
        ];
        let result = eval_ce(&args, &ctx).as_text();
        // 50% alpha = 127 or 128
        assert!(result.starts_with("#7F") || result.starts_with("#80"));
    }

    #[test]
    fn test_cm_red() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Number(0.0)),   // hue
            Expr::Literal(Value::Number(100.0)), // saturation
            Expr::Literal(Value::Number(50.0)),  // lightness
        ];
        assert_eq!(eval_cm(&args, &ctx).as_text(), "#FF0000");
    }

    #[test]
    fn test_gradient_mix_0() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("#FF0000".into())),
            Expr::Literal(Value::Text("#0000FF".into())),
            Expr::Literal(Value::Number(0.0)),
        ];
        assert_eq!(eval_ce(&args, &ctx).as_text(), "#FF0000");
    }

    #[test]
    fn test_gradient_mix_100() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("#FF0000".into())),
            Expr::Literal(Value::Text("#0000FF".into())),
            Expr::Literal(Value::Number(100.0)),
        ];
        assert_eq!(eval_ce(&args, &ctx).as_text(), "#0000FF");
    }

    #[test]
    fn test_gradient_mix_50() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("#FF0000".into())),
            Expr::Literal(Value::Text("#0000FF".into())),
            Expr::Literal(Value::Number(50.0)),
        ];
        let result = eval_ce(&args, &ctx).as_text();
        // 50% mix of red and blue: R=128, G=0, B=128
        assert_eq!(result, "#800080");
    }

    #[test]
    fn test_hsl_roundtrip() {
        let (h, s, l) = rgb_to_hsl(255, 0, 0);
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 1.0).abs() < 0.01);
        assert!((l - 0.5).abs() < 0.01);

        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }
}
