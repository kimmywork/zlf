use std::collections::{BTreeMap, BTreeSet};

use zlf_core::{Result, ZlfError};
use zlf_index::BM25DocumentHit;

use crate::{Bm25IndexTarget, CoordinatorConfig, IndexCoordinator, IndexProfileStore, ZlfDatabase};

const TARGET: &str = "bm25";

impl ZlfDatabase {
    pub fn search_bm25(
        &self,
        query: &str,
        top_k: usize,
        fields: &[String],
        explain: bool,
    ) -> Result<Vec<BM25DocumentHit>> {
        let weights = self.active_bm25_weights()?;
        self.bm25
            .search_document_top_k(query, top_k, fields, &weights, explain)
    }

    pub(crate) fn catch_up_bm25(&self) -> Result<()> {
        let coordinator = IndexCoordinator::new(&self.storage, CoordinatorConfig::default());
        coordinator.register_target(TARGET)?;
        let target = Bm25IndexTarget::new(&self.bm25, TARGET);
        loop {
            let enqueued = coordinator.enqueue_available(TARGET)?;
            while coordinator.process_next(TARGET, &target)? {}
            if enqueued == 0 {
                break;
            }
        }
        let progress = coordinator.progress(TARGET)?;
        if progress.published_watermark < progress.scanned_watermark {
            return Err(ZlfError::Internal(format!(
                "BM25 indexing stopped at watermark {} of {}",
                progress.published_watermark, progress.scanned_watermark
            )));
        }
        Ok(())
    }

    fn active_bm25_weights(&self) -> Result<BTreeMap<String, f32>> {
        let store = IndexProfileStore::new(&self.storage);
        let names = store
            .list()?
            .into_iter()
            .map(|profile| profile.name)
            .collect::<BTreeSet<_>>();
        let mut weights = BTreeMap::new();
        for name in names {
            if let Some(profile) = store.active(&name)? {
                for (field, options) in profile.fields {
                    if let Some(bm25) = options.bm25 {
                        weights.insert(field, bm25.weight);
                    }
                }
            }
        }
        Ok(weights)
    }
}
