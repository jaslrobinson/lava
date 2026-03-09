use crate::evaluator::EvalContext;
use crate::parser::Expr;
use crate::value::Value;

/// Evaluate `sh(command)` - shell execution (disabled).
///
/// Shell execution inside a formula engine is a security risk.
/// This stub returns a sentinel value so callers know the function
/// was recognized but intentionally blocked.
pub fn eval_sh(_args: &[Expr], _ctx: &EvalContext) -> Value {
    Value::Text("[sh:disabled]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sh_disabled() {
        let ctx = EvalContext::new();
        let args = vec![Expr::Literal(Value::Text("echo hello".into()))];
        assert_eq!(eval_sh(&args, &ctx).as_text(), "[sh:disabled]");
    }

    #[test]
    fn test_sh_no_args() {
        let ctx = EvalContext::new();
        assert_eq!(eval_sh(&[], &ctx).as_text(), "[sh:disabled]");
    }
}
