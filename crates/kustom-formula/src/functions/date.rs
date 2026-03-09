use crate::evaluator::EvalContext;
use crate::parser::Expr;
use crate::value::Value;
use chrono::{Datelike, Local, NaiveDateTime, Timelike};

/// Evaluate `df(format, [date])` - date formatting.
pub fn eval_df(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.is_empty() {
        return Value::Text(String::new());
    }

    let format_str = ctx.evaluate(&args[0]).as_text();
    let now = Local::now().naive_local();
    let dt = if args.len() > 1 {
        let date_str = ctx.evaluate(&args[1]).as_text();
        NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
            .unwrap_or(now)
    } else {
        now
    };

    let chrono_fmt = kustom_format_to_chrono(&format_str, &dt);
    Value::Text(chrono_fmt)
}

/// Evaluate `tf(seconds, format)` - time format from seconds.
pub fn eval_tf(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.len() < 2 {
        return Value::Text(String::new());
    }

    let total_secs = ctx.evaluate(&args[0]).as_number() as i64;
    let format_str = ctx.evaluate(&args[1]).as_text();

    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let result = format_str
        .replace("hh", &format!("{:02}", hours))
        .replace("h", &format!("{}", hours))
        .replace("mm", &format!("{:02}", minutes))
        .replace("m", &format!("{}", minutes))
        .replace("ss", &format!("{:02}", seconds))
        .replace("s", &format!("{}", seconds));

    Value::Text(result)
}

/// Evaluate `dp(value, type)` - date parts (returns numeric).
pub fn eval_dp(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.is_empty() {
        return Value::Number(0.0);
    }

    let part = ctx.evaluate(&args[0]).as_text();
    let now = Local::now().naive_local();

    let val = match part.as_str() {
        "h" => now.hour() as f64,
        "m" => now.minute() as f64,
        "s" => now.second() as f64,
        "d" => now.day() as f64,
        "M" => now.month() as f64,
        "y" | "yyyy" => now.year() as f64,
        "w" => now.iso_week().week() as f64,
        _ => 0.0,
    };

    Value::Number(val)
}

/// Evaluate `tu(timestamp_type)` - unix timestamps.
pub fn eval_tu(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.is_empty() {
        return Value::Number(0.0);
    }

    let mode = ctx.evaluate(&args[0]).as_text();
    let now = Local::now();

    let val = match mode.as_str() {
        "s" => now.timestamp() as f64,
        "ms" => now.timestamp_millis() as f64,
        _ => now.timestamp() as f64,
    };

    Value::Number(val)
}

/// Convert a KLWP date format string to the formatted output using chrono.
/// Each KLWP specifier is mapped to a chrono format and resolved immediately.
fn kustom_format_to_chrono(format: &str, dt: &NaiveDateTime) -> String {
    let mut result = String::new();
    let chars: Vec<char> = format.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Try matching longest patterns first
        if i + 4 <= len {
            let four: String = chars[i..i + 4].iter().collect();
            match four.as_str() {
                "EEEE" => { result.push_str(&dt.format("%A").to_string()); i += 4; continue; }
                "MMMM" => { result.push_str(&dt.format("%B").to_string()); i += 4; continue; }
                "yyyy" => { result.push_str(&dt.format("%Y").to_string()); i += 4; continue; }
                _ => {}
            }
        }

        if i + 3 <= len {
            let three: String = chars[i..i + 3].iter().collect();
            match three.as_str() {
                "EEE" => { result.push_str(&dt.format("%a").to_string()); i += 3; continue; }
                "MMM" => { result.push_str(&dt.format("%b").to_string()); i += 3; continue; }
                "DDD" => { result.push_str(&dt.format("%j").to_string()); i += 3; continue; }
                _ => {}
            }
        }

        if i + 2 <= len {
            let two: String = chars[i..i + 2].iter().collect();
            match two.as_str() {
                "hh" => { result.push_str(&dt.format("%I").to_string()); i += 2; continue; }
                "HH" => { result.push_str(&dt.format("%H").to_string()); i += 2; continue; }
                "mm" => { result.push_str(&dt.format("%M").to_string()); i += 2; continue; }
                "ss" => { result.push_str(&dt.format("%S").to_string()); i += 2; continue; }
                "dd" => { result.push_str(&dt.format("%d").to_string()); i += 2; continue; }
                "MM" => { result.push_str(&dt.format("%m").to_string()); i += 2; continue; }
                "yy" => { result.push_str(&dt.format("%y").to_string()); i += 2; continue; }
                _ => {}
            }
        }

        // Single character patterns
        match chars[i] {
            'h' => result.push_str(&dt.format("%-I").to_string()),
            'H' => result.push_str(&dt.format("%-H").to_string()),
            'm' => result.push_str(&dt.format("%-M").to_string()),
            's' => result.push_str(&dt.format("%-S").to_string()),
            'a' => result.push_str(&dt.format("%P").to_string()),
            'A' => result.push_str(&dt.format("%p").to_string()),
            'd' => result.push_str(&dt.format("%-d").to_string()),
            'M' => result.push_str(&dt.format("%-m").to_string()),
            'D' => result.push_str(&dt.format("%-j").to_string()),
            'E' => {
                // Single E = first char of weekday
                let day_name = dt.format("%A").to_string();
                if let Some(c) = day_name.chars().next() {
                    result.push(c);
                }
            }
            'w' => {
                result.push_str(&format!("{}", dt.iso_week().week()));
            }
            c => result.push(c),
        }
        i += 1;
    }

    result
}

/// Format a NaiveDateTime using a KLWP format string.
/// Exposed for testing with fixed dates.
pub fn format_datetime(dt: &NaiveDateTime, format: &str) -> String {
    kustom_format_to_chrono(format, dt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn test_dt() -> NaiveDateTime {
        // Wednesday, March 8, 2026 14:05:09
        NaiveDate::from_ymd_opt(2026, 3, 8)
            .unwrap()
            .and_hms_opt(14, 5, 9)
            .unwrap()
    }

    #[test]
    fn test_format_hh_mm() {
        let dt = test_dt();
        assert_eq!(format_datetime(&dt, "hh:mm"), "02:05");
    }

    #[test]
    fn test_format_24h() {
        let dt = test_dt();
        assert_eq!(format_datetime(&dt, "HH:mm"), "14:05");
    }

    #[test]
    fn test_format_h_no_pad() {
        let dt = test_dt();
        assert_eq!(format_datetime(&dt, "h:mm"), "2:05");
    }

    #[test]
    fn test_format_full_date() {
        let dt = test_dt();
        let result = format_datetime(&dt, "yyyy-MM-dd");
        assert_eq!(result, "2026-03-08");
    }

    #[test]
    fn test_format_weekday() {
        let dt = test_dt(); // Sunday
        let result = format_datetime(&dt, "EEEE");
        assert_eq!(result, "Sunday");
    }

    #[test]
    fn test_format_short_weekday() {
        let dt = test_dt();
        let result = format_datetime(&dt, "EEE");
        assert_eq!(result, "Sun");
    }

    #[test]
    fn test_format_month_name() {
        let dt = test_dt();
        assert_eq!(format_datetime(&dt, "MMMM"), "March");
        assert_eq!(format_datetime(&dt, "MMM"), "Mar");
    }

    #[test]
    fn test_tf_basic() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Number(3661.0)),
            Expr::Literal(Value::Text("hh:mm:ss".into())),
        ];
        let result = eval_tf(&args, &ctx);
        assert_eq!(result.as_text(), "01:01:01");
    }
}
