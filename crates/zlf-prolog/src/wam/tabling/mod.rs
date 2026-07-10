mod backend;
mod delta;
mod evaluator;
mod fixpoint;
mod key;
mod manager;
mod scc;
mod store;
mod terms;

pub use backend::{PersistedTable, RocksTableBackend, TableBackend};
pub use key::{NormalizedTerm, TableKey};
pub use manager::{TableManager, TableMetricsSnapshot};
pub use store::{TableAnswer, TableEntry, TableLimits, TableState, TableStore};

pub(crate) use evaluator::evaluate_tabled;
