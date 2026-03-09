use crate::evaluator::EvalContext;
use crate::parser::Expr;
use crate::value::Value;

/// Evaluate `wg(url)` - web get (stub).
///
/// The actual HTTP implementation lives in the JS/WASM bridge layer.
/// This stub exists so the Rust dispatch table does not silently drop the call.
pub fn eval_wg(_args: &[Expr], _ctx: &EvalContext) -> Value {
    Value::Text("[wg:client-only]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wg_stub() {
        let ctx = EvalContext::new();
        let args = vec![Expr::Literal(Value::Text("https://example.com".into()))];
        assert_eq!(eval_wg(&args, &ctx).as_text(), "[wg:client-only]");
    }

    #[test]
    fn test_wg_no_args() {
        let ctx = EvalContext::new();
        assert_eq!(eval_wg(&[], &ctx).as_text(), "[wg:client-only]");
    }
}
