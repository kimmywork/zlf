use zlf_core::Result;
use zlf_index::VectorEntry;

use crate::{helpers::lock_error, ZlfDatabase};

impl ZlfDatabase {
    pub fn search(&self, query: &str) -> Result<Vec<(String, f32)>> {
        Ok(self
            .search_bm25(query, 100, &[], false)?
            .into_iter()
            .map(|hit| (hit.document_id.entity.id().to_string(), hit.score))
            .collect())
    }

    pub fn index_text(&self, node_id: &str, text: &str) -> Result<()> {
        self.bm25
            .read()
            .map_err(lock_error)?
            .index_text(node_id, text)
    }

    pub fn index_embedding(&self, node_id: &str, embedding: &[f32], model: &str) -> Result<()> {
        self.vector.add_entry(VectorEntry {
            node_id: node_id.to_string(),
            embedding: embedding.to_vec(),
            model: model.to_string(),
        })
    }

    pub fn similar(
        &self,
        node_id: &str,
        threshold: f32,
        limit: usize,
    ) -> Result<Vec<(String, f32)>> {
        match self.vector.get_entry(node_id)? {
            Some(entry) => self.vector.find_similar(&entry.embedding, threshold, limit),
            None => Ok(Vec::new()),
        }
    }
}
