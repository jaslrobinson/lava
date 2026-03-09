use crate::evaluator::EvalContext;
use crate::parser::Expr;
use crate::value::Value;
use regex::Regex;

/// Evaluate `tc(mode, text, ...)` - text functions.
pub fn eval_tc(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.is_empty() {
        return Value::Text(String::new());
    }

    let mode = ctx.evaluate(&args[0]).as_text().to_lowercase();

    match mode.as_str() {
        "low" => {
            let text = get_text_arg(args, 1, ctx);
            Value::Text(text.to_lowercase())
        }
        "up" => {
            let text = get_text_arg(args, 1, ctx);
            Value::Text(text.to_uppercase())
        }
        "cap" => {
            let text = get_text_arg(args, 1, ctx);
            Value::Text(capitalize_words(&text))
        }
        "cut" => {
            let text = get_text_arg(args, 1, ctx);
            let start = get_num_arg(args, 2, ctx) as usize;
            let len = get_num_arg(args, 3, ctx) as usize;
            let chars: Vec<char> = text.chars().collect();
            let end = (start + len).min(chars.len());
            let start = start.min(chars.len());
            Value::Text(chars[start..end].iter().collect())
        }
        "ell" => {
            let text = get_text_arg(args, 1, ctx);
            let max_len = get_num_arg(args, 2, ctx) as usize;
            if text.chars().count() > max_len {
                let truncated: String = text.chars().take(max_len.saturating_sub(3)).collect();
                Value::Text(format!("{}...", truncated))
            } else {
                Value::Text(text)
            }
        }
        "split" => {
            let text = get_text_arg(args, 1, ctx);
            let delim = get_text_arg(args, 2, ctx);
            let index = get_num_arg(args, 3, ctx) as usize;
            let parts: Vec<&str> = text.split(&delim).collect();
            Value::Text(parts.get(index).unwrap_or(&"").to_string())
        }
        "len" => {
            let text = get_text_arg(args, 1, ctx);
            Value::Number(text.chars().count() as f64)
        }
        "count" => {
            let text = get_text_arg(args, 1, ctx);
            let ch = get_text_arg(args, 2, ctx);
            let count = text.matches(&ch).count();
            Value::Number(count as f64)
        }
        "lines" => {
            let text = get_text_arg(args, 1, ctx);
            let count = if text.is_empty() { 0 } else { text.lines().count() };
            Value::Number(count as f64)
        }
        "reg" => {
            let text = get_text_arg(args, 1, ctx);
            let pattern = get_text_arg(args, 2, ctx);
            let replacement = get_text_arg(args, 3, ctx);
            match Regex::new(&pattern) {
                Ok(re) => Value::Text(re.replace_all(&text, replacement.as_str()).to_string()),
                Err(_) => Value::Text(text),
            }
        }
        "n2w" => {
            let num = get_num_arg(args, 1, ctx) as i64;
            Value::Text(number_to_words(num))
        }
        "ord" => {
            let num = get_num_arg(args, 1, ctx) as i64;
            Value::Text(ordinal(num))
        }
        "utf" => {
            let code = get_num_arg(args, 1, ctx) as u32;
            match char::from_u32(code) {
                Some(c) => Value::Text(c.to_string()),
                None => Value::Text(String::new()),
            }
        }
        _ => Value::Text(String::new()),
    }
}

fn get_text_arg(args: &[Expr], index: usize, ctx: &EvalContext) -> String {
    args.get(index).map(|a| ctx.evaluate(a).as_text()).unwrap_or_default()
}

fn get_num_arg(args: &[Expr], index: usize, ctx: &EvalContext) -> f64 {
    args.get(index).map(|a| ctx.evaluate(a).as_number()).unwrap_or(0.0)
}

fn capitalize_words(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    format!("{}{}", upper, chars.as_str().to_lowercase())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn number_to_words(n: i64) -> String {
    if n < 0 {
        return format!("negative {}", number_to_words(-n));
    }

    match n {
        0 => "zero".to_string(),
        1 => "one".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        6 => "six".to_string(),
        7 => "seven".to_string(),
        8 => "eight".to_string(),
        9 => "nine".to_string(),
        10 => "ten".to_string(),
        11 => "eleven".to_string(),
        12 => "twelve".to_string(),
        13 => "thirteen".to_string(),
        14 => "fourteen".to_string(),
        15 => "fifteen".to_string(),
        16 => "sixteen".to_string(),
        17 => "seventeen".to_string(),
        18 => "eighteen".to_string(),
        19 => "nineteen".to_string(),
        20..=99 => {
            let tens = match n / 10 {
                2 => "twenty",
                3 => "thirty",
                4 => "forty",
                5 => "fifty",
                6 => "sixty",
                7 => "seventy",
                8 => "eighty",
                9 => "ninety",
                _ => "",
            };
            let ones = n % 10;
            if ones == 0 {
                tens.to_string()
            } else {
                format!("{}-{}", tens, number_to_words(ones))
            }
        }
        100..=999 => {
            let hundreds = n / 100;
            let remainder = n % 100;
            if remainder == 0 {
                format!("{} hundred", number_to_words(hundreds))
            } else {
                format!("{} hundred {}", number_to_words(hundreds), number_to_words(remainder))
            }
        }
        _ => n.to_string(),
    }
}

fn ordinal(n: i64) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, 11) => "th",
        (2, 12) => "th",
        (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{}{}", n, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_tc_simple(mode: &str, text: &str) -> Value {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text(mode.into())),
            Expr::Literal(Value::Text(text.into())),
        ];
        eval_tc(&args, &ctx)
    }

    #[test]
    fn test_uppercase() {
        assert_eq!(eval_tc_simple("up", "hello").as_text(), "HELLO");
    }

    #[test]
    fn test_lowercase() {
        assert_eq!(eval_tc_simple("low", "HELLO").as_text(), "hello");
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(eval_tc_simple("cap", "hello world").as_text(), "Hello World");
    }

    #[test]
    fn test_len() {
        let result = eval_tc_simple("len", "hello");
        assert_eq!(result.as_number(), 5.0);
    }

    #[test]
    fn test_cut() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("cut".into())),
            Expr::Literal(Value::Text("hello world".into())),
            Expr::Literal(Value::Number(0.0)),
            Expr::Literal(Value::Number(5.0)),
        ];
        assert_eq!(eval_tc(&args, &ctx).as_text(), "hello");
    }

    #[test]
    fn test_ellipsis() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("ell".into())),
            Expr::Literal(Value::Text("hello world".into())),
            Expr::Literal(Value::Number(8.0)),
        ];
        assert_eq!(eval_tc(&args, &ctx).as_text(), "hello...");
    }

    #[test]
    fn test_split() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("split".into())),
            Expr::Literal(Value::Text("a,b,c".into())),
            Expr::Literal(Value::Text(",".into())),
            Expr::Literal(Value::Number(1.0)),
        ];
        assert_eq!(eval_tc(&args, &ctx).as_text(), "b");
    }

    #[test]
    fn test_count() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("count".into())),
            Expr::Literal(Value::Text("banana".into())),
            Expr::Literal(Value::Text("a".into())),
        ];
        assert_eq!(eval_tc(&args, &ctx).as_number(), 3.0);
    }

    #[test]
    fn test_ordinal() {
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(4), "4th");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(21), "21st");
    }

    #[test]
    fn test_n2w() {
        assert_eq!(number_to_words(0), "zero");
        assert_eq!(number_to_words(42), "forty-two");
        assert_eq!(number_to_words(100), "one hundred");
        assert_eq!(number_to_words(123), "one hundred twenty-three");
    }

    #[test]
    fn test_regex_replace() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("reg".into())),
            Expr::Literal(Value::Text("hello 123 world".into())),
            Expr::Literal(Value::Text("\\d+".into())),
            Expr::Literal(Value::Text("NUM".into())),
        ];
        assert_eq!(eval_tc(&args, &ctx).as_text(), "hello NUM world");
    }

    #[test]
    fn test_utf() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("utf".into())),
            Expr::Literal(Value::Number(65.0)),
        ];
        assert_eq!(eval_tc(&args, &ctx).as_text(), "A");
    }
}
