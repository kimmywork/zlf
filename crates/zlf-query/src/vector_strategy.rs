use zlf_index::HnswVectorOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VectorIndexStrategy {
    #[default]
    Disabled,
    Exact,
    Hnsw(HnswVectorOptions),
}

impl VectorIndexStrategy {
    pub fn hnsw() -> Self {
        Self::Hnsw(HnswVectorOptions::default())
    }

    pub fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZlfDatabaseOptions {
    pub vector_index: VectorIndexStrategy,
}
