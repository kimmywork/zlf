use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexJobMetrics {
    pub pending: u64,
    pub claimed: u64,
    pub retried: u64,
    pub dead: u64,
    pub stale: u64,
    pub lag: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexInventory {
    pub documents: u64,
    pub chunks: u64,
    pub vectors: u64,
    pub temporal_records: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexMetricsSnapshot {
    pub jobs: IndexJobMetrics,
    pub inventory: IndexInventory,
    pub query_count: u64,
    pub candidate_count: u64,
}
