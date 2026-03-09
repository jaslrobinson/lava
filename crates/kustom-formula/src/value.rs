use std::fmt;

/// The universal value type used throughout the formula engine.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Number(f64),
    Bool(bool),
    /// Hex color string: #RRGGBB or #AARRGGBB
    Color(String),
    None,
}

impl Value {
    pub fn as_text(&self) -> String {
        match self {
            Value::Text(s) => s.clone(),
            Value::Number(n) => {
                if !n.is_finite() {
                    return "0".to_string();
                }
                if *n == n.floor() {
                    format!("{}", *n as i64)
                } else {
                    // Cap at 2 decimal places for clean display, strip trailing zeros
                    let s = format!("{:.2}", n);
                    let s = s.trim_end_matches('0');
                    let s = s.trim_end_matches('.');
                    s.to_string()
                }
            }
            Value::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
            Value::Color(c) => c.clone(),
            Value::None => String::new(),
        }
    }

    pub fn as_number(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::Text(s) => s.parse::<f64>().unwrap_or(0.0),
            Value::Bool(b) => if *b { 1.0 } else { 0.0 },
            Value::Color(_) => 0.0,
            Value::None => 0.0,
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::Text(s) => !s.is_empty() && s != "0" && s.to_lowercase() != "false",
            Value::Color(_) => true,
            Value::None => false,
        }
    }

    pub fn as_color(&self) -> String {
        match self {
            Value::Color(c) => c.clone(),
            Value::Text(s) if s.starts_with('#') => s.clone(),
            _ => "#00000000".to_string(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_as_text_integer() {
        assert_eq!(Value::Number(42.0).as_text(), "42");
    }

    #[test]
    fn test_number_as_text_float() {
        assert_eq!(Value::Number(3.14).as_text(), "3.14");
    }

    #[test]
    fn test_text_as_number() {
        assert_eq!(Value::Text("3.14".into()).as_number(), 3.14);
    }

    #[test]
    fn test_text_as_number_invalid() {
        assert_eq!(Value::Text("abc".into()).as_number(), 0.0);
    }

    #[test]
    fn test_bool_as_text() {
        assert_eq!(Value::Bool(true).as_text(), "1");
        assert_eq!(Value::Bool(false).as_text(), "0");
    }

    #[test]
    fn test_none_conversions() {
        assert_eq!(Value::None.as_text(), "");
        assert_eq!(Value::None.as_number(), 0.0);
        assert!(!Value::None.as_bool());
    }

    #[test]
    fn test_color_as_color() {
        assert_eq!(Value::Color("#FF0000".into()).as_color(), "#FF0000");
    }

    #[test]
    fn test_text_as_color() {
        assert_eq!(Value::Text("#FF0000".into()).as_color(), "#FF0000");
        assert_eq!(Value::Text("notacolor".into()).as_color(), "#00000000");
    }
}
