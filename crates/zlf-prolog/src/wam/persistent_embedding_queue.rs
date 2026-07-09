use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use zlf_index::{VectorEntry, VectorIndex};
use zlf_storage::Storage;

use super::error::{WamError, WamResult};
use super::storage_index_writer::Embedder;

const PREFIX: &str = "embedq:";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistentEmbeddingJob {
    pub id: String,
    pub node_id: String,
    pub text: String,
}

pub struct PersistentEmbeddingQueue<'a> {
    storage: &'a Storage,
}

impl<'a> PersistentEmbeddingQueue<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn enqueue(
        &self,
        node_id: impl Into<String>,
        text: impl Into<String>,
    ) -> WamResult<PersistentEmbeddingJob> {
        let job = PersistentEmbeddingJob {
            id: next_id(),
            node_id: node_id.into(),
            text: text.into(),
        };
        self.put_job(&job)?;
        Ok(job)
    }

    pub fn pending(&self) -> WamResult<Vec<PersistentEmbeddingJob>> {
        self.storage
            .scan_prefix(PREFIX)
            .map_err(provider_error)?
            .into_iter()
            .map(|(_, value)| bincode::deserialize(&value).map_err(provider_error))
            .collect()
    }

    pub fn process_all(&self, embedder: &dyn Embedder, index: &VectorIndex) -> WamResult<usize> {
        let mut processed = 0;
        for job in self.pending()? {
            index
                .add_entry(VectorEntry {
                    node_id: job.node_id.clone(),
                    embedding: embedder.embed(&job.text)?,
                    model: embedder.model().to_string(),
                })
                .map_err(provider_error)?;
            self.ack(&job.id)?;
            processed += 1;
        }
        Ok(processed)
    }

    pub fn ack(&self, id: &str) -> WamResult<()> {
        self.storage
            .delete_raw(&queue_key(id))
            .map_err(provider_error)
    }

    fn put_job(&self, job: &PersistentEmbeddingJob) -> WamResult<()> {
        let data = bincode::serialize(job).map_err(provider_error)?;
        self.storage
            .put_raw(&queue_key(&job.id), &data)
            .map_err(provider_error)
    }
}

fn queue_key(id: &str) -> String {
    format!("{PREFIX}{id}")
}

fn next_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{nanos}")
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
