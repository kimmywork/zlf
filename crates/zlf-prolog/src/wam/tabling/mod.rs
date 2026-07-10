mod evaluator;
mod key;
mod store;

pub use key::{NormalizedTerm, TableKey};
pub use store::{TableAnswer, TableEntry, TableLimits, TableState, TableStore};

pub(crate) use evaluator::evaluate_tabled;
