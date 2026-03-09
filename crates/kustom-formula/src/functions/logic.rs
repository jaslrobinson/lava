use crate::evaluator::EvalContext;
use crate::parser::Expr;
use crate::value::Value;

/// Evaluate `if(condition, then_value, [else_value])`.
pub fn eval_if(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.is_empty() {
        return Value::None;
    }

    let condition = ctx.evaluate(&args[0]);

    if condition.as_bool() {
        args.get(1).map(|a| ctx.evaluate(a)).unwrap_or(Value::None)
    } else {
        args.get(2).map(|a| ctx.evaluate(a)).unwrap_or(Value::None)
    }
}

/// Evaluate `fl(init, stop, incr, body, [separator])` - for loop.
/// The loop variable `i` is made available in context.locals during body evaluation.
pub fn eval_fl(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.len() < 4 {
        return Value::Text(String::new());
    }

    let init = ctx.evaluate(&args[0]).as_number();
    let stop = ctx.evaluate(&args[1]).as_number();
    let incr = ctx.evaluate(&args[2]).as_number();
    let sep = args.get(4).map(|a| ctx.evaluate(a).as_text()).unwrap_or_default();

    if incr == 0.0 {
        return Value::Text(String::new());
    }

    let mut results = Vec::new();
    let mut i = init;

    let max_iterations = 10000; // safety limit
    let mut count = 0;

    while (incr > 0.0 && i <= stop) || (incr < 0.0 && i >= stop) {
        if count >= max_iterations {
            break;
        }

        // Create a child context with i set as a local variable
        let mut child_ctx = ctx.child();
        child_ctx.locals.insert("i".to_string(), Value::Number(i));

        let val = child_ctx.evaluate(&args[3]);
        results.push(val.as_text());

        i += incr;
        count += 1;
    }

    Value::Text(results.join(&sep))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::BinOp;

    #[test]
    fn test_if_true() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Bool(true)),
            Expr::Literal(Value::Text("yes".into())),
            Expr::Literal(Value::Text("no".into())),
        ];
        assert_eq!(eval_if(&args, &ctx).as_text(), "yes");
    }

    #[test]
    fn test_if_false() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Bool(false)),
            Expr::Literal(Value::Text("yes".into())),
            Expr::Literal(Value::Text("no".into())),
        ];
        assert_eq!(eval_if(&args, &ctx).as_text(), "no");
    }

    #[test]
    fn test_if_numeric_truthy() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Number(1.0)),
            Expr::Literal(Value::Text("yes".into())),
            Expr::Literal(Value::Text("no".into())),
        ];
        assert_eq!(eval_if(&args, &ctx).as_text(), "yes");
    }

    #[test]
    fn test_if_no_else() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Bool(false)),
            Expr::Literal(Value::Text("yes".into())),
        ];
        let result = eval_if(&args, &ctx);
        assert_eq!(result, Value::None);
    }

    #[test]
    fn test_if_with_comparison() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::BinaryOp {
                op: BinOp::Gt,
                left: Box::new(Expr::Literal(Value::Number(10.0))),
                right: Box::new(Expr::Literal(Value::Number(5.0))),
            },
            Expr::Literal(Value::Text("bigger".into())),
            Expr::Literal(Value::Text("smaller".into())),
        ];
        assert_eq!(eval_if(&args, &ctx).as_text(), "bigger");
    }

    #[test]
    fn test_fl_basic() {
        let ctx = EvalContext::new();
        // fl(1, 3, 1, "x", ",") => "x,x,x"
        let args2 = vec![
            Expr::Literal(Value::Number(1.0)),
            Expr::Literal(Value::Number(3.0)),
            Expr::Literal(Value::Number(1.0)),
            Expr::Literal(Value::Text("x".into())),
            Expr::Literal(Value::Text(",".into())),
        ];
        assert_eq!(eval_fl(&args2, &ctx).as_text(), "x,x,x");
    }

    #[test]
    fn test_fl_zero_incr() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Number(0.0)),
            Expr::Literal(Value::Number(10.0)),
            Expr::Literal(Value::Number(0.0)),
            Expr::Literal(Value::Text("x".into())),
        ];
        assert_eq!(eval_fl(&args, &ctx).as_text(), "");
    }
}
