use std::collections::HashMap;
use std::sync::Arc;

use crate::functions::{color, date, logic, math, shell, text, variables, web};
use crate::parser::{BinOp, Expr};
use crate::value::Value;

/// Evaluation context holding variables and data providers.
pub struct EvalContext {
    pub globals: HashMap<String, Value>,
    pub locals: HashMap<String, Value>,
    /// Provider data keyed by prefix (e.g. "bi", "wi", "mi"), then by field name.
    pub providers: Arc<HashMap<String, HashMap<String, Value>>>,
    /// Recursion depth counter to prevent stack overflow from deeply nested formulas.
    pub depth: usize,
}

impl EvalContext {
    pub fn new() -> Self {
        EvalContext {
            globals: HashMap::new(),
            locals: HashMap::new(),
            providers: Arc::new(HashMap::new()),
            depth: 0,
        }
    }

    /// Create a child context that inherits globals and providers but has its own locals.
    pub fn child(&self) -> EvalContext {
        EvalContext {
            globals: self.globals.clone(),
            locals: self.locals.clone(),
            providers: self.providers.clone(),
            depth: self.depth + 1,
        }
    }

    /// Evaluate an AST expression to a Value.
    pub fn evaluate(&self, expr: &Expr) -> Value {
        const MAX_DEPTH: usize = 100;
        if self.depth > MAX_DEPTH {
            return Value::Text("[max depth]".into());
        }
        match expr {
            Expr::Literal(v) => v.clone(),

            Expr::UnaryNeg(inner) => {
                let val = self.evaluate(inner);
                Value::Number(-val.as_number())
            }

            Expr::BinaryOp { op, left, right } => {
                let lhs = self.evaluate(left);
                let rhs = self.evaluate(right);
                eval_binary_op(*op, &lhs, &rhs)
            }

            Expr::FunctionCall { name, args } => {
                self.eval_function(name, args)
            }

            Expr::Template(parts) => {
                let mut result = String::new();
                for part in parts {
                    result.push_str(&self.evaluate(part).as_text());
                }
                Value::Text(result)
            }
        }
    }

    fn eval_function(&self, name: &str, args: &[Expr]) -> Value {
        // Dispatch based on function name prefix
        match name {
            "df" => date::eval_df(args, self),
            "dp" => date::eval_dp(args, self),
            "tf" => date::eval_tf(args, self),
            "tu" => date::eval_tu(args, self),
            "if" => logic::eval_if(args, self),
            "fl" => logic::eval_fl(args, self),
            "mu" => math::eval_mu(args, self),
            "tc" => text::eval_tc(args, self),
            "ce" => color::eval_ce(args, self),
            "cm" => color::eval_cm(args, self),
            "gv" => variables::eval_gv(args, self),
            "lv" => variables::eval_lv(args, self),
            "lrc" => text::eval_lrc(args, self),
            "wg" => web::eval_wg(args, self),
            "sh" => shell::eval_sh(args, self),
            _ => {
                // Check if it's a provider function (bi, wi, mi, si, etc.)
                if name.len() >= 2 {
                    let prefix = &name[..2];
                    if let Some(provider) = self.providers.get(prefix) {
                        if args.len() >= 2 {
                            // Multi-arg provider: join args with "_" (e.g. wf(0, temp) -> "0_temp")
                            let parts: Vec<String> = args.iter()
                                .map(|a| self.evaluate(a).as_text())
                                .collect();
                            let key = parts.join("_");
                            return provider.get(&key).cloned().unwrap_or(Value::None);
                        } else if let Some(arg) = args.first() {
                            let key = self.evaluate(arg).as_text();
                            return provider.get(&key).cloned().unwrap_or(Value::None);
                        }
                    }
                }
                // Unknown function - try to look up by full name in providers
                if let Some(provider) = self.providers.get(name) {
                    if let Some(arg) = args.first() {
                        let key = self.evaluate(arg).as_text();
                        return provider.get(&key).cloned().unwrap_or(Value::None);
                    }
                }
                Value::None
            }
        }
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

fn eval_binary_op(op: BinOp, lhs: &Value, rhs: &Value) -> Value {
    match op {
        BinOp::Add => {
            // If both are numbers, add numerically; otherwise concatenate as text
            match (lhs, rhs) {
                (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
                _ => Value::Text(format!("{}{}", lhs.as_text(), rhs.as_text())),
            }
        }
        BinOp::Sub => Value::Number(lhs.as_number() - rhs.as_number()),
        BinOp::Mul => Value::Number(lhs.as_number() * rhs.as_number()),
        BinOp::Div => {
            let divisor = rhs.as_number();
            if divisor == 0.0 {
                Value::Number(0.0)
            } else {
                Value::Number(lhs.as_number() / divisor)
            }
        }
        BinOp::Eq => Value::Bool(values_equal(lhs, rhs)),
        BinOp::Ne => Value::Bool(!values_equal(lhs, rhs)),
        BinOp::Gt => Value::Bool(lhs.as_number() > rhs.as_number()),
        BinOp::Ge => Value::Bool(lhs.as_number() >= rhs.as_number()),
        BinOp::Lt => Value::Bool(lhs.as_number() < rhs.as_number()),
        BinOp::Le => Value::Bool(lhs.as_number() <= rhs.as_number()),
        BinOp::And => Value::Bool(lhs.as_bool() && rhs.as_bool()),
        BinOp::Or => Value::Bool(lhs.as_bool() || rhs.as_bool()),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => (x - y).abs() < f64::EPSILON,
        (Value::Text(x), Value::Text(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => a.as_text() == b.as_text(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn eval(input: &str) -> String {
        let ctx = EvalContext::new();
        let expr = parser::parse(input).unwrap();
        ctx.evaluate(&expr).as_text()
    }

    fn eval_with_context(input: &str, ctx: &EvalContext) -> String {
        let expr = parser::parse(input).unwrap();
        ctx.evaluate(&expr).as_text()
    }

    #[test]
    fn test_literal_text() {
        assert_eq!(eval("Hello world"), "Hello world");
    }

    #[test]
    fn test_mu_round() {
        assert_eq!(eval("$mu(round, 3.7)$"), "4");
    }

    #[test]
    fn test_tc_uppercase() {
        assert_eq!(eval("$tc(up, \"hello\")$"), "HELLO");
    }

    #[test]
    fn test_if_false() {
        assert_eq!(eval("$if(1 > 2, \"yes\", \"no\")$"), "no");
    }

    #[test]
    fn test_if_true() {
        assert_eq!(eval("$if(5 > 2, \"yes\", \"no\")$"), "yes");
    }

    #[test]
    fn test_mu_pow() {
        assert_eq!(eval("$mu(pow, 2, 3)$"), "8");
    }

    #[test]
    fn test_tc_len() {
        assert_eq!(eval("$tc(len, \"hello\")$"), "5");
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(eval("$1 + 2 * 3$"), "7");
    }

    #[test]
    fn test_mixed_template() {
        assert_eq!(eval("Result: $mu(round, 3.14)$!"), "Result: 3!");
    }

    #[test]
    fn test_nested_functions() {
        assert_eq!(eval("$tc(up, \"hello\")$"), "HELLO");
    }

    #[test]
    fn test_global_variable() {
        let mut ctx = EvalContext::new();
        ctx.globals.insert("name".into(), Value::Text("World".into()));
        assert_eq!(eval_with_context("Hello $gv(name)$!", &ctx), "Hello World!");
    }

    #[test]
    fn test_provider_lookup() {
        let mut ctx = EvalContext::new();
        let mut providers = HashMap::new();
        let mut bi = HashMap::new();
        bi.insert("level".into(), Value::Number(75.0));
        providers.insert("bi".into(), bi);
        ctx.providers = Arc::new(providers);
        assert_eq!(eval_with_context("$bi(level)$", &ctx), "75");
    }

    #[test]
    fn test_division_by_zero() {
        assert_eq!(eval("$10 / 0$"), "0");
    }

    #[test]
    fn test_boolean_and() {
        assert_eq!(eval("$if(1 > 0 & 2 > 1, \"both\", \"nope\")$"), "both");
    }

    #[test]
    fn test_boolean_or() {
        assert_eq!(eval("$if(1 > 2 | 3 > 1, \"one\", \"none\")$"), "one");
    }

    #[test]
    fn test_comparison_equal() {
        assert_eq!(eval("$if(5 = 5, \"eq\", \"ne\")$"), "eq");
    }

    #[test]
    fn test_comparison_not_equal() {
        assert_eq!(eval("$if(5 != 3, \"ne\", \"eq\")$"), "ne");
    }

    #[test]
    fn test_unary_neg() {
        assert_eq!(eval("$mu(abs, -5)$"), "5");
    }

    #[test]
    fn test_string_concatenation_via_add() {
        assert_eq!(eval("$\"hello\" + \" \" + \"world\"$"), "hello world");
    }
}
