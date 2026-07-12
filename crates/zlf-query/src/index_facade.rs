use crate::ZlfDatabase;
use zlf_core::Result;

impl ZlfDatabase {
    pub fn search(&self, query: &str) -> Result<Vec<(String, f32)>> {
        Ok(self
            .search_bm25(query, 100, &[], false)?
            .into_iter()
            .map(|hit| (hit.document_id.entity.id().to_string(), hit.score))
            .collect())
    }
}
