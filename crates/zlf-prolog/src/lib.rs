pub mod parser;
pub mod parser_ast;
pub mod wam;

pub use parser::PrologParser;
pub use parser_ast::{Fact, PrologRule, Query, Term};
