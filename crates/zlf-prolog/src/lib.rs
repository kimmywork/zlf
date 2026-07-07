pub mod parser;
pub mod wam;

pub use parser::{PrologParser, Term, Fact, PrologRule, Query};
pub use wam::WAM;
