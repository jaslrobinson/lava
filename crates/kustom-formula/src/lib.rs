pub mod evaluator;
pub mod functions;
pub mod parser;
pub mod value;

/// Parse and evaluate a KLWP-style formula string.
///
/// Formula strings can contain literal text mixed with `$...$` delimited expressions.
///
/// # Examples
/// ```
/// use kustom_formula::{evaluate, EvalContext};
///
/// let ctx = EvalContext::new();
/// assert_eq!(evaluate("$mu(round, 3.7)$", &ctx), "4");
/// assert_eq!(evaluate("$tc(up, \"hello\")$", &ctx), "HELLO");
/// assert_eq!(evaluate("Hello world", &ctx), "Hello world");
/// ```
pub fn evaluate(formula: &str, context: &evaluator::EvalContext) -> String {
    match parser::parse(formula) {
        Ok(expr) => context.evaluate(&expr).as_text(),
        Err(e) => format!("ERROR: {}", e),
    }
}

// Re-export key types at crate root
pub use evaluator::EvalContext;
pub use parser::{parse, BinOp, Expr, ParseError};
pub use value::Value;
