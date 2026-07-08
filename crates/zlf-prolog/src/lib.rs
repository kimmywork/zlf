pub mod parser;
pub mod wam;
pub mod wam_enhanced;

pub use parser::{PrologParser, Term, Fact, PrologRule, Query};
pub use wam::WAM;
pub use wam_enhanced::WAM as EnhancedWAM;
