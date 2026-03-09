use crate::value::Value;
use std::fmt;

/// AST node for a formula expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Value),
    FunctionCall { name: String, args: Vec<Expr> },
    BinaryOp { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    UnaryNeg(Box<Expr>),
    /// A sequence of literal text and formula segments.
    Template(Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error at position {}: {}", self.position, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Tokenizer token types.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    StringLit(String),
    Ident(String),
    LParen,
    RParen,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Eq,       // =
    Ne,       // !=
    Gt,       // >
    Ge,       // >=
    Lt,       // <
    Le,       // <=
    Amp,      // &
    Pipe,     // |
    Tilde,    // ~
    Eof,
}

struct Tokenizer {
    chars: Vec<char>,
    pos: usize,
}

impl Tokenizer {
    fn new(input: &str) -> Self {
        Tokenizer {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace();

        let ch = match self.peek_char() {
            Some(ch) => ch,
            None => return Ok(Token::Eof),
        };

        match ch {
            '(' => { self.advance(); Ok(Token::LParen) }
            ')' => { self.advance(); Ok(Token::RParen) }
            ',' => { self.advance(); Ok(Token::Comma) }
            '+' => { self.advance(); Ok(Token::Plus) }
            '-' => { self.advance(); Ok(Token::Minus) }
            '*' => { self.advance(); Ok(Token::Star) }
            '/' => { self.advance(); Ok(Token::Slash) }
            '&' => { self.advance(); Ok(Token::Amp) }
            '|' => { self.advance(); Ok(Token::Pipe) }
            '~' => { self.advance(); Ok(Token::Tilde) }
            '!' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    Ok(Token::Ne)
                } else {
                    Err(ParseError {
                        message: "Expected '=' after '!'".to_string(),
                        position: self.pos,
                    })
                }
            }
            '=' => { self.advance(); Ok(Token::Eq) }
            '>' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    Ok(Token::Ge)
                } else {
                    Ok(Token::Gt)
                }
            }
            '<' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    Ok(Token::Le)
                } else {
                    Ok(Token::Lt)
                }
            }
            '"' => self.read_string(),
            c if c.is_ascii_digit() || (c == '.' && matches!(self.chars.get(self.pos + 1), Some(d) if d.is_ascii_digit())) => self.read_number(),
            c if is_ident_start(c) => self.read_ident(),
            '#' => self.read_color_hex(),
            other => Err(ParseError {
                message: format!("Unexpected character: '{}'", other),
                position: self.pos,
            }),
        }
    }

    fn read_string(&mut self) -> Result<Token, ParseError> {
        self.advance(); // consume opening "
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(Token::StringLit(s)),
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(c) => { s.push('\\'); s.push(c); }
                        None => return Err(ParseError {
                            message: "Unterminated string escape".to_string(),
                            position: self.pos,
                        }),
                    }
                }
                Some(c) => s.push(c),
                None => return Err(ParseError {
                    message: "Unterminated string literal".to_string(),
                    position: self.pos,
                }),
            }
        }
    }

    fn read_number(&mut self) -> Result<Token, ParseError> {
        let start = self.pos;
        let mut has_dot = false;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                self.advance();
            } else if ch == '.' && !has_dot {
                // Only treat as decimal if followed by a digit
                if matches!(self.chars.get(self.pos + 1), Some(d) if d.is_ascii_digit()) {
                    has_dot = true;
                    self.advance();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        match s.parse::<f64>() {
            Ok(n) => Ok(Token::Number(n)),
            Err(_) => Err(ParseError {
                message: format!("Invalid number: {}", s),
                position: start,
            }),
        }
    }

    fn read_ident(&mut self) -> Result<Token, ParseError> {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if is_ident_char(ch) {
                self.advance();
            } else {
                break;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        Ok(Token::Ident(s))
    }

    fn read_color_hex(&mut self) -> Result<Token, ParseError> {
        let start = self.pos;
        self.advance(); // consume #
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_hexdigit() {
                self.advance();
            } else {
                break;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        Ok(Token::StringLit(s))
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Extract raw argument strings from a function call.
/// Given the chars of the expression and a starting position right after the opening `(`,
/// returns a Vec of raw argument strings and the position after the closing `)`.
fn extract_raw_args(chars: &[char], start: usize) -> Result<(Vec<String>, usize), ParseError> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut depth = 0;
    let mut i = start;
    let mut in_string = false;

    while i < chars.len() {
        let ch = chars[i];

        if in_string {
            current_arg.push(ch);
            if ch == '\\' && i + 1 < chars.len() {
                i += 1;
                current_arg.push(chars[i]);
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                current_arg.push(ch);
                i += 1;
            }
            '(' => {
                depth += 1;
                current_arg.push(ch);
                i += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
                current_arg.push(ch);
                i += 1;
            }
            ')' if depth == 0 => {
                // End of function call
                let trimmed = current_arg.trim().to_string();
                if !trimmed.is_empty() {
                    args.push(trimmed);
                }
                return Ok((args, i + 1)); // skip the )
            }
            ',' if depth == 0 => {
                let trimmed = current_arg.trim().to_string();
                args.push(trimmed);
                current_arg = String::new();
                i += 1;
            }
            _ => {
                current_arg.push(ch);
                i += 1;
            }
        }
    }

    Err(ParseError {
        message: "Unclosed function call parenthesis".to_string(),
        position: start,
    })
}

/// Try to parse a string as an expression. If it fails, return it as a string literal.
fn parse_arg_or_literal(input: &str) -> Expr {
    if input.is_empty() {
        return Expr::Literal(Value::Text(String::new()));
    }

    // Try to parse as a normal expression
    match parse_expression_inner(input) {
        Ok(expr) => expr,
        Err(_) => {
            // Fall back to treating as a raw string literal
            Expr::Literal(Value::Text(input.to_string()))
        }
    }
}

/// Parser state wrapping a tokenizer with one-token lookahead.
struct Parser {
    tokenizer: Tokenizer,
    current: Token,
    /// The raw input string, used for extracting raw function arguments
    raw_input: Vec<char>,
}

impl Parser {
    fn new(input: &str) -> Result<Self, ParseError> {
        let mut tokenizer = Tokenizer::new(input);
        let current = tokenizer.next_token()?;
        Ok(Parser {
            tokenizer,
            current,
            raw_input: input.chars().collect(),
        })
    }

    fn advance(&mut self) -> Result<Token, ParseError> {
        let prev = std::mem::replace(&mut self.current, Token::Eof);
        self.current = self.tokenizer.next_token()?;
        Ok(prev)
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.current) == std::mem::discriminant(expected) {
            self.advance()?;
            Ok(())
        } else {
            Err(ParseError {
                message: format!("Expected {:?}, got {:?}", expected, self.current),
                position: self.tokenizer.pos,
            })
        }
    }

    /// Parse a full expression with operator precedence.
    /// Precedence (low to high): Or < And < Comparison < AddSub < MulDiv
    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.current == Token::Pipe {
            self.advance()?;
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        while self.current == Token::Amp {
            self.advance()?;
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_add_sub()?;
        let op = match &self.current {
            Token::Eq => Some(BinOp::Eq),
            Token::Ne => Some(BinOp::Ne),
            Token::Gt => Some(BinOp::Gt),
            Token::Ge => Some(BinOp::Ge),
            Token::Lt => Some(BinOp::Lt),
            Token::Le => Some(BinOp::Le),
            // ~= for regex matching (treat as Eq for now)
            Token::Tilde => None,
            _ => None,
        };
        if let Some(op) = op {
            self.advance()?;
            let right = self.parse_add_sub()?;
            Ok(Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_add_sub(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_mul_div()?;
        loop {
            let op = match &self.current {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_mul_div()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_mul_div(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;
        loop {
            let op = match &self.current {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_primary()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.current.clone() {
            Token::Number(n) => {
                self.advance()?;
                Ok(Expr::Literal(Value::Number(n)))
            }
            Token::StringLit(s) => {
                self.advance()?;
                Ok(Expr::Literal(Value::Text(s)))
            }
            Token::Ident(name) => {
                self.advance()?;
                // Check if this is a function call
                if self.current == Token::LParen {
                    // Use raw argument extraction for KLWP compatibility
                    // This handles unquoted format strings like df(hh:mm:ss)
                    let paren_pos = self.tokenizer.pos; // position right after '('
                    let (raw_args, end_pos) = extract_raw_args(&self.raw_input, paren_pos)?;

                    // Parse each raw argument
                    let args: Vec<Expr> = raw_args
                        .iter()
                        .map(|arg| parse_arg_or_literal(arg))
                        .collect();

                    // Advance the tokenizer past the function call
                    self.tokenizer.pos = end_pos;
                    self.current = self.tokenizer.next_token()?;

                    Ok(Expr::FunctionCall { name, args })
                } else {
                    // Bare identifier - treat as a text literal
                    Ok(Expr::Literal(Value::Text(name)))
                }
            }
            Token::LParen => {
                self.advance()?;
                let expr = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::Minus => {
                self.advance()?;
                let expr = self.parse_primary()?;
                Ok(Expr::UnaryNeg(Box::new(expr)))
            }
            Token::Eof => Err(ParseError {
                message: "Unexpected end of input".to_string(),
                position: self.tokenizer.pos,
            }),
            other => Err(ParseError {
                message: format!("Unexpected token: {:?}", other),
                position: self.tokenizer.pos,
            }),
        }
    }
}

/// Parse a top-level formula string that may contain `$...$` delimited expressions
/// mixed with literal text.
pub fn parse(input: &str) -> Result<Expr, ParseError> {
    let chars: Vec<char> = input.chars().collect();
    let mut segments: Vec<Expr> = Vec::new();
    let mut i = 0;
    let len = chars.len();

    while i < len {
        if chars[i] == '$' {
            // Find the matching closing $, respecting parenthesis nesting
            let start = i + 1;
            let mut depth = 0;
            let mut j = start;
            let mut found = false;
            while j < len {
                match chars[j] {
                    '(' => depth += 1,
                    ')' => {
                        if depth > 0 {
                            depth -= 1;
                        }
                    }
                    '"' => {
                        // Skip string literals inside the expression
                        j += 1;
                        while j < len && chars[j] != '"' {
                            if chars[j] == '\\' {
                                j += 1; // skip escaped char
                            }
                            j += 1;
                        }
                    }
                    '$' if depth == 0 => {
                        found = true;
                        break;
                    }
                    _ => {}
                }
                j += 1;
            }
            if found {
                let expr_str: String = chars[start..j].iter().collect();
                // Use parse_expression_inner which handles raw format strings
                let expr = parse_expression_inner(&expr_str)?;
                segments.push(expr);
                i = j + 1; // skip closing $
            } else {
                // No closing $, treat as literal
                segments.push(Expr::Literal(Value::Text("$".to_string())));
                i = start;
            }
        } else {
            // Collect literal text until next $
            let start = i;
            while i < len && chars[i] != '$' {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            segments.push(Expr::Literal(Value::Text(text)));
        }
    }

    match segments.len() {
        0 => Ok(Expr::Literal(Value::Text(String::new()))),
        1 => Ok(segments.into_iter().next().unwrap()),
        _ => Ok(Expr::Template(segments)),
    }
}

/// Parse a raw expression string (without `$` delimiters).
pub fn parse_expression(input: &str) -> Result<Expr, ParseError> {
    parse_expression_inner(input)
}

fn parse_expression_inner(input: &str) -> Result<Expr, ParseError> {
    let mut parser = Parser::new(input)?;
    let expr = parser.parse_expression()?;
    // Ensure the entire input was consumed
    if parser.current != Token::Eof {
        return Err(ParseError {
            message: format!("Unexpected token after expression: {:?}", parser.current),
            position: parser.tokenizer.pos,
        });
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_literal_text() {
        let result = parse("Hello world").unwrap();
        assert_eq!(result, Expr::Literal(Value::Text("Hello world".into())));
    }

    #[test]
    fn test_parse_function_call() {
        let result = parse("$df(hh)$").unwrap();
        match result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "df");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected FunctionCall, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_format_string_with_colon() {
        // This is the key KLWP test — df(hh:mm) should work
        let result = parse("$df(hh:mm)$").unwrap();
        match &result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "df");
                assert_eq!(args.len(), 1);
                assert_eq!(args[0], Expr::Literal(Value::Text("hh:mm".into())));
            }
            _ => panic!("Expected FunctionCall, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_format_string_with_colons() {
        let result = parse("$df(hh:mm:ss)$").unwrap();
        match &result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "df");
                assert_eq!(args.len(), 1);
                assert_eq!(args[0], Expr::Literal(Value::Text("hh:mm:ss".into())));
            }
            _ => panic!("Expected FunctionCall, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_format_string_complex() {
        // df(EEEE, MMMM d, yyyy) — commas separate args
        let result = parse("$df(EEEE)$").unwrap();
        match &result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "df");
                assert_eq!(args.len(), 1);
                assert_eq!(args[0], Expr::Literal(Value::Text("EEEE".into())));
            }
            _ => panic!("Expected FunctionCall, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_df_with_date_offset() {
        let result = parse("$df(EEE, a1d)$").unwrap();
        match &result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "df");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], Expr::Literal(Value::Text("EEE".into())));
            }
            _ => panic!("Expected FunctionCall, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_nested_if() {
        let result = parse("$if(1 > 0, \"yes\", \"no\")$").unwrap();
        match result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "if");
                assert_eq!(args.len(), 3);
                // First arg should be a BinaryOp
                match &args[0] {
                    Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Gt),
                    _ => panic!("Expected BinaryOp, got {:?}", args[0]),
                }
            }
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_parse_mixed_template() {
        let result = parse("Time: $df(hh:mm)$!").unwrap();
        match result {
            Expr::Template(parts) => {
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0], Expr::Literal(Value::Text("Time: ".into())));
                assert_eq!(parts[2], Expr::Literal(Value::Text("!".into())));
                // Middle part should be the df function call
                match &parts[1] {
                    Expr::FunctionCall { name, .. } => assert_eq!(name, "df"),
                    _ => panic!("Expected FunctionCall"),
                }
            }
            _ => panic!("Expected Template"),
        }
    }

    #[test]
    fn test_parse_nested_functions() {
        let result = parse("$tc(up, df(EEEE))$").unwrap();
        match result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "tc");
                assert_eq!(args.len(), 2);
                match &args[1] {
                    Expr::FunctionCall { name, .. } => assert_eq!(name, "df"),
                    _ => panic!("Expected nested FunctionCall, got {:?}", args[1]),
                }
            }
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_operator_precedence() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let result = parse("$1 + 2 * 3$").unwrap();
        match result {
            Expr::BinaryOp { op: BinOp::Add, left, right } => {
                assert_eq!(*left, Expr::Literal(Value::Number(1.0)));
                match *right {
                    Expr::BinaryOp { op: BinOp::Mul, .. } => {}
                    _ => panic!("Expected Mul on right"),
                }
            }
            _ => panic!("Expected Add at top level, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_comparison() {
        let result = parse_expression("x >= 10").unwrap();
        match result {
            Expr::BinaryOp { op: BinOp::Ge, .. } => {}
            _ => panic!("Expected Ge"),
        }
    }

    #[test]
    fn test_parse_boolean_ops() {
        let result = parse_expression("1 > 0 & 2 > 1").unwrap();
        match result {
            Expr::BinaryOp { op: BinOp::And, .. } => {}
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn test_parse_unary_neg() {
        let result = parse_expression("-5").unwrap();
        match result {
            Expr::UnaryNeg(inner) => {
                assert_eq!(*inner, Expr::Literal(Value::Number(5.0)));
            }
            _ => panic!("Expected UnaryNeg"),
        }
    }

    #[test]
    fn test_parse_string_literal() {
        let result = parse_expression("\"hello world\"").unwrap();
        assert_eq!(result, Expr::Literal(Value::Text("hello world".into())));
    }

    #[test]
    fn test_parse_empty_input() {
        let result = parse("").unwrap();
        assert_eq!(result, Expr::Literal(Value::Text(String::new())));
    }

    #[test]
    fn test_parse_mu_function() {
        let result = parse("$mu(round, 3.14, 1)$").unwrap();
        match result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "mu");
                assert_eq!(args.len(), 3);
            }
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_parse_if_with_nested_df() {
        let result = parse("$if(df(H) > 12, \"PM\", \"AM\")$").unwrap();
        match &result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "if");
                assert_eq!(args.len(), 3);
                // First arg: df(H) > 12
                match &args[0] {
                    Expr::BinaryOp { op: BinOp::Gt, left, .. } => {
                        match left.as_ref() {
                            Expr::FunctionCall { name, .. } => assert_eq!(name, "df"),
                            _ => panic!("Expected df function call"),
                        }
                    }
                    _ => panic!("Expected BinaryOp Gt, got {:?}", args[0]),
                }
            }
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_parse_color_literal() {
        let result = parse("$ce(#FF6633, invert)$").unwrap();
        match &result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "ce");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], Expr::Literal(Value::Text("#FF6633".into())));
            }
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_parse_tc_with_nested() {
        let result = parse("$tc(up, \"hello\")$").unwrap();
        match &result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "tc");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_parse_format_with_spaces() {
        // df(d MMM yyyy) — spaces in format string
        let result = parse("$df(d MMM yyyy)$").unwrap();
        match &result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "df");
                assert_eq!(args.len(), 1);
                assert_eq!(args[0], Expr::Literal(Value::Text("d MMM yyyy".into())));
            }
            _ => panic!("Expected FunctionCall, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_fl_for_loop() {
        let result = parse("$fl(1, 5, \"i + 1\", \"#\")$").unwrap();
        match &result {
            Expr::FunctionCall { name, args } => {
                assert_eq!(name, "fl");
                assert_eq!(args.len(), 4);
            }
            _ => panic!("Expected FunctionCall"),
        }
    }
}
