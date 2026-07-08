pub mod parser;
pub mod wam;
pub mod wam_enhanced;
pub mod wam_v2;

pub use parser::{PrologParser, Term, Fact, PrologRule, Query};
pub use wam::WAM;
pub use wam_enhanced::WAM as EnhancedWAM;
pub use wam_v2::WAMExecutor;
