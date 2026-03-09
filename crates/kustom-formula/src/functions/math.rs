use crate::evaluator::EvalContext;
use crate::parser::Expr;
use crate::value::Value;
use rand::Rng;

/// Evaluate `mu(func, args...)` - math utilities.
pub fn eval_mu(args: &[Expr], ctx: &EvalContext) -> Value {
    if args.is_empty() {
        return Value::Number(0.0);
    }

    let func = ctx.evaluate(&args[0]).as_text().to_lowercase();
    let nums: Vec<f64> = args[1..].iter().map(|a| ctx.evaluate(a).as_number()).collect();

    let result = match func.as_str() {
        "ceil" => nums.first().map(|n| n.ceil()).unwrap_or(0.0),
        "floor" => nums.first().map(|n| n.floor()).unwrap_or(0.0),
        "round" => {
            if let Some(&n) = nums.first() {
                if let Some(&places) = nums.get(1) {
                    let factor = 10f64.powi(places as i32);
                    (n * factor).round() / factor
                } else {
                    n.round()
                }
            } else {
                0.0
            }
        }
        "abs" => nums.first().map(|n| n.abs()).unwrap_or(0.0),
        "sin" => nums.first().map(|n| n.to_radians().sin()).unwrap_or(0.0),
        "cos" => nums.first().map(|n| n.to_radians().cos()).unwrap_or(0.0),
        "tan" => nums.first().map(|n| n.to_radians().tan()).unwrap_or(0.0),
        "asin" => nums.first().map(|n| n.asin().to_degrees()).unwrap_or(0.0),
        "acos" => nums.first().map(|n| n.acos().to_degrees()).unwrap_or(0.0),
        "atan" => nums.first().map(|n| n.atan().to_degrees()).unwrap_or(0.0),
        "log" => nums.first().map(|n| n.log10()).unwrap_or(0.0),
        "ln" => nums.first().map(|n| n.ln()).unwrap_or(0.0),
        "pow" => {
            if nums.len() >= 2 {
                nums[0].powf(nums[1])
            } else {
                0.0
            }
        }
        "sqrt" => nums.first().map(|n| n.sqrt()).unwrap_or(0.0),
        "min" => {
            if nums.len() >= 2 {
                nums[0].min(nums[1])
            } else {
                nums.first().copied().unwrap_or(0.0)
            }
        }
        "max" => {
            if nums.len() >= 2 {
                nums[0].max(nums[1])
            } else {
                nums.first().copied().unwrap_or(0.0)
            }
        }
        "rnd" => {
            if nums.len() >= 2 {
                let mut rng = rand::thread_rng();
                let lo = nums[0];
                let hi = nums[1];
                rng.gen_range(lo..=hi)
            } else {
                0.0
            }
        }
        "h2d" => {
            // Hex string to decimal (the argument is passed as a text, re-evaluate)
            let hex_str = if args.len() > 1 {
                ctx.evaluate(&args[1]).as_text()
            } else {
                return Value::Number(0.0);
            };
            let hex_str = hex_str.trim_start_matches("0x").trim_start_matches("0X");
            i64::from_str_radix(hex_str, 16).unwrap_or(0) as f64
        }
        "d2h" => {
            let n = nums.first().copied().unwrap_or(0.0) as i64;
            return Value::Text(format!("{:X}", n));
        }
        _ => 0.0,
    };

    Value::Number(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_mu_simple(func: &str, args: &[f64]) -> Value {
        let ctx = EvalContext::new();
        let mut expr_args: Vec<Expr> = vec![Expr::Literal(Value::Text(func.into()))];
        for &n in args {
            expr_args.push(Expr::Literal(Value::Number(n)));
        }
        eval_mu(&expr_args, &ctx)
    }

    #[test]
    fn test_round() {
        assert_eq!(eval_mu_simple("round", &[3.7]).as_number(), 4.0);
    }

    #[test]
    fn test_round_with_places() {
        assert_eq!(eval_mu_simple("round", &[3.456, 2.0]).as_number(), 3.46);
    }

    #[test]
    fn test_ceil() {
        assert_eq!(eval_mu_simple("ceil", &[3.2]).as_number(), 4.0);
    }

    #[test]
    fn test_floor() {
        assert_eq!(eval_mu_simple("floor", &[3.9]).as_number(), 3.0);
    }

    #[test]
    fn test_abs() {
        assert_eq!(eval_mu_simple("abs", &[-5.0]).as_number(), 5.0);
    }

    #[test]
    fn test_pow() {
        assert_eq!(eval_mu_simple("pow", &[2.0, 3.0]).as_number(), 8.0);
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(eval_mu_simple("sqrt", &[9.0]).as_number(), 3.0);
    }

    #[test]
    fn test_min_max() {
        assert_eq!(eval_mu_simple("min", &[3.0, 7.0]).as_number(), 3.0);
        assert_eq!(eval_mu_simple("max", &[3.0, 7.0]).as_number(), 7.0);
    }

    #[test]
    fn test_sin_cos() {
        let sin90 = eval_mu_simple("sin", &[90.0]).as_number();
        assert!((sin90 - 1.0).abs() < 1e-10);
        let cos0 = eval_mu_simple("cos", &[0.0]).as_number();
        assert!((cos0 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_log() {
        let log100 = eval_mu_simple("log", &[100.0]).as_number();
        assert!((log100 - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_d2h() {
        let result = eval_mu_simple("d2h", &[255.0]);
        assert_eq!(result.as_text(), "FF");
    }

    #[test]
    fn test_h2d() {
        let ctx = EvalContext::new();
        let args = vec![
            Expr::Literal(Value::Text("h2d".into())),
            Expr::Literal(Value::Text("FF".into())),
        ];
        let result = eval_mu(&args, &ctx);
        assert_eq!(result.as_number(), 255.0);
    }
}
