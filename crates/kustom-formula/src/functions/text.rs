use std::cell::RefCell;
use std::collections::HashMap;

use crate::evaluator::EvalContext;
use crate::parser::Expr;
use crate::value::Value;
use regex::Regex;

thread_local! {
    static REGEX_CACHE: RefCell<HashMap<String, Regex>> = RefCell::new(HashMap::new());
}

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
            let result = REGEX_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                if !cache.contains_key(&pattern) {
                    match Regex::new(&pattern) {
                        Ok(re) => { cache.insert(pattern.clone(), re); }
                        Err(_) => return text.clone(),
                    }
                }
                let re = cache.get(&pattern).unwrap();
                re.replace_all(&text, replacement.as_str()).to_string()
            });
            Value::Text(result)
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
        "roman" => {
            let n = get_num_arg(args, 1, ctx) as i64;
            Value::Text(to_roman(n))
        }
        "url" => {
            let text = get_text_arg(args, 1, ctx);
            Value::Text(url_encode(&text))
        }
        "html" => {
            let text = get_text_arg(args, 1, ctx);
            Value::Text(strip_html(&text))
        }
        "json" => {
            let text = get_text_arg(args, 1, ctx);
            let path = get_text_arg(args, 2, ctx);
            Value::Text(json_extract(&text, &path))
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

fn to_roman(n: i64) -> String {
    if n < 1 || n > 3999 {
        return String::new();
    }
    let table: &[(i64, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut result = String::new();
    let mut remaining = n;
    for &(value, symbol) in table {
        while remaining >= value {
            result.push_str(symbol);
            remaining -= value;
        }
    }
    result
}

fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

fn strip_html(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

fn json_extract(text: &str, path: &str) -> String {
    // Minimal JSON value extraction supporting dot-separated paths.
    // Handles nested objects, strings, numbers, booleans, and null.
    let trimmed = text.trim();
    if path.is_empty() {
        return json_value_to_string(trimmed);
    }

    let keys: Vec<&str> = path.split('.').collect();
    let mut current = trimmed;

    for key in &keys {
        // Find key in current JSON object
        current = current.trim();
        if !current.starts_with('{') {
            return String::new();
        }
        match json_find_key(current, key) {
            Some(val) => current = val,
            None => return String::new(),
        }
    }

    json_value_to_string(current.trim())
}

/// Find the value for a given key in a JSON object string, returning the raw value substring.
fn json_find_key<'a>(obj: &'a str, key: &str) -> Option<&'a str> {
    let bytes = obj.as_bytes();
    let len = bytes.len();
    // Skip the opening '{'
    let mut i = 1;

    loop {
        // Skip whitespace
        while i < len && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= len || bytes[i] == b'}' {
            return None;
        }
        // Skip comma
        if bytes[i] == b',' {
            i += 1;
            continue;
        }
        // Expect a key string
        if bytes[i] != b'"' {
            return None;
        }
        let key_start = i + 1;
        i = key_start;
        while i < len && bytes[i] != b'"' {
            if bytes[i] == b'\\' {
                i += 1;
            }
            i += 1;
        }
        if i >= len {
            return None;
        }
        let found_key = &obj[key_start..i];
        i += 1; // skip closing '"'
        // Skip whitespace and colon
        while i < len && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        if i >= len || bytes[i] != b':' {
            return None;
        }
        i += 1;
        while i < len && (bytes[i] as char).is_whitespace() {
            i += 1;
        }
        // Find the extent of the value
        let val_start = i;
        i = json_skip_value(obj, i)?;
        if found_key == key {
            return Some(obj[val_start..i].trim());
        }
    }
}

/// Skip over a JSON value starting at position `i`, return position after value.
fn json_skip_value(s: &str, i: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if i >= len {
        return None;
    }
    match bytes[i] {
        b'"' => {
            let mut j = i + 1;
            while j < len {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == b'"' {
                    return Some(j + 1);
                }
                j += 1;
            }
            None
        }
        b'{' => {
            let mut depth = 1;
            let mut j = i + 1;
            while j < len && depth > 0 {
                match bytes[j] {
                    b'"' => {
                        j += 1;
                        while j < len && bytes[j] != b'"' {
                            if bytes[j] == b'\\' {
                                j += 1;
                            }
                            j += 1;
                        }
                    }
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            Some(j)
        }
        b'[' => {
            let mut depth = 1;
            let mut j = i + 1;
            while j < len && depth > 0 {
                match bytes[j] {
                    b'"' => {
                        j += 1;
                        while j < len && bytes[j] != b'"' {
                            if bytes[j] == b'\\' {
                                j += 1;
                            }
                            j += 1;
                        }
                    }
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            Some(j)
        }
        _ => {
            // number, bool, null
            let mut j = i;
            while j < len && bytes[j] != b',' && bytes[j] != b'}' && bytes[j] != b']'
                && !(bytes[j] as char).is_whitespace()
            {
                j += 1;
            }
            Some(j)
        }
    }
}

fn json_value_to_string(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        // Strip quotes
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
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

    #[test]
    fn test_roman() {
        assert_eq!(to_roman(1), "I");
        assert_eq!(to_roman(4), "IV");
        assert_eq!(to_roman(9), "IX");
        assert_eq!(to_roman(42), "XLII");
        assert_eq!(to_roman(1994), "MCMXCIV");
        assert_eq!(to_roman(3999), "MMMCMXCIX");
        assert_eq!(to_roman(0), "");
        assert_eq!(to_roman(4000), "");
    }

    #[test]
    fn test_tc_roman() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("roman".into())),
            Expr::Literal(Value::Number(42.0)),
        ];
        assert_eq!(eval_tc(&args, &ctx).as_text(), "XLII");
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(url_encode("abc"), "abc");
    }

    #[test]
    fn test_tc_url() {
        assert_eq!(eval_tc_simple("url", "hello world").as_text(), "hello%20world");
    }

    #[test]
    fn test_strip_html() {
        assert_eq!(strip_html("<b>bold</b>"), "bold");
        assert_eq!(strip_html("no tags"), "no tags");
        assert_eq!(strip_html("<p>one</p><p>two</p>"), "onetwo");
        assert_eq!(strip_html("<a href=\"x\">link</a>"), "link");
    }

    #[test]
    fn test_tc_html() {
        assert_eq!(eval_tc_simple("html", "<b>bold</b>").as_text(), "bold");
    }

    #[test]
    fn test_json_extract() {
        assert_eq!(json_extract(r#"{"a":1}"#, "a"), "1");
        assert_eq!(json_extract(r#"{"a":{"b":2}}"#, "a.b"), "2");
        assert_eq!(json_extract(r#"{"a":"hello"}"#, "a"), "hello");
        assert_eq!(json_extract(r#"{"x":true}"#, "x"), "true");
        assert_eq!(json_extract(r#"{"a":1}"#, "b"), "");
    }

    #[test]
    fn test_tc_json() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("json".into())),
            Expr::Literal(Value::Text(r#"{"a":{"b":1}}"#.into())),
            Expr::Literal(Value::Text("a.b".into())),
        ];
        assert_eq!(eval_tc(&args, &ctx).as_text(), "1");
    }
}
