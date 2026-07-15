use zlf_core::Result;

use crate::{
    EmbeddingModelProfile, ExactVectorStore, VectorHit, VectorQuery, VectorRecord,
    VectorSearchBackend,
};

impl VectorSearchBackend for ExactVectorStore {
    fn search(
        &self,
        query: &VectorQuery,
        profile: &EmbeddingModelProfile,
    ) -> Result<Vec<VectorHit>> {
        ExactVectorStore::search(self, query, profile)
    }

    fn records_for_entity(
        &self,
        generation: &str,
        model_profile: &str,
        model_version: u32,
        entity: &zlf_core::EntityRef,
    ) -> Result<Vec<VectorRecord>> {
        ExactVectorStore::records_for_entity(self, generation, model_profile, model_version, entity)
    }
}
