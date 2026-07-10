pub mod parser;
pub mod parser_ast;
pub(crate) mod parser_expr;
mod parser_expr_scan;
mod parser_helpers;
pub mod wam;

pub use parser::PrologParser;
pub use parser_ast::{Fact, PrologRule, Query, Term};

#[cfg(test)]
mod builtin_executor_test;
#[cfg(test)]
mod stage4_builtin_test;
