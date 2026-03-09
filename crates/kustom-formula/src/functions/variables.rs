use crate::evaluator::EvalContext;
use crate::parser::Expr;
use crate::value::Value;

/// Evaluate `gv(name)` - global variable lookup.
pub fn eval_gv(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.is_empty() {
        return Value::None;
    }

    let name = ctx.evaluate(&args[0]).as_text();
    ctx.globals
        .get(&name)
        .cloned()
        .unwrap_or(Value::None)
}

/// Evaluate `lv(name)` - local variable lookup.
pub fn eval_lv(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.is_empty() {
        return Value::None;
    }

    let name = ctx.evaluate(&args[0]).as_text();
    ctx.locals
        .get(&name)
        .cloned()
        .unwrap_or(Value::None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gv_found() {
        let mut ctx = EvalContext::new();
        ctx.globals.insert("myvar".into(), Value::Text("hello".into()));
        let args = vec![Expr::Literal(Value::Text("myvar".into()))];
        assert_eq!(eval_gv(&args, &ctx).as_text(), "hello");
    }

    #[test]
    fn test_gv_not_found() {
        let ctx = EvalContext::new();
        let args = vec![Expr::Literal(Value::Text("missing".into()))];
        assert_eq!(eval_gv(&args, &ctx), Value::None);
    }

    #[test]
    fn test_lv_found() {
        let mut ctx = EvalContext::new();
        ctx.locals.insert("x".into(), Value::Number(42.0));
        let args = vec![Expr::Literal(Value::Text("x".into()))];
        assert_eq!(eval_lv(&args, &ctx).as_number(), 42.0);
    }

    #[test]
    fn test_lv_not_found() {
        let ctx = EvalContext::new();
        let args = vec![Expr::Literal(Value::Text("nope".into()))];
        assert_eq!(eval_lv(&args, &ctx), Value::None);
    }
}
