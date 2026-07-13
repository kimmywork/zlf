use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::path::Path;
use std::sync::Arc;

use rocksdb::{IteratorMode, Options, WriteBatch, DB};
use zlf_core::{Result, ZlfError};

use crate::vector_exact_support::{matches_filters, DocumentFilters};
use crate::{
    ranked_page, validate_query_vector, EmbeddingModelProfile, IndexPage, IndexPageRequest,
    VectorHit, VectorKey, VectorMetric, VectorQuery, VectorRecord,
};

const PREFIX: &[u8] = b"vector:exact:v1:";
const SCHEMA_KEY: &[u8] = b"vector:exact:schema";
const SCHEMA_VALUE: &[u8] = b"1";

#[derive(Clone)]
pub struct ExactVectorStore {
    db: Arc<DB>,
}

impl ExactVectorStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut options = Options::default();
        options.create_if_missing(true);
        let db = DB::open(&options, path).map_err(internal)?;
        match db.get(SCHEMA_KEY).map_err(internal)? {
            Some(value) if value.as_slice() != SCHEMA_VALUE => {
                return Err(ZlfError::Internal(
                    "incompatible exact vector schema".into(),
                ))
            }
            None => db.put(SCHEMA_KEY, SCHEMA_VALUE).map_err(internal)?,
            _ => {}
        }
        Ok(Self { db: Arc::new(db) })
    }

    pub fn put(&self, record: &VectorRecord, profile: &EmbeddingModelProfile) -> Result<()> {
        self.apply(std::slice::from_ref(record), &[], profile)
    }

    pub fn apply(
        &self,
        upserts: &[VectorRecord],
        deletes: &[VectorKey],
        profile: &EmbeddingModelProfile,
    ) -> Result<()> {
        let mut batch = WriteBatch::default();
        for record in upserts {
            record.validate(profile).map_err(ZlfError::Internal)?;
            batch.put(
                storage_key(&record.key),
                bincode::serialize(record).map_err(serialization)?,
            );
        }
        for key in deletes {
            batch.delete(storage_key(key));
        }
        self.db.write(batch).map_err(internal)
    }

    pub fn get(&self, key: &VectorKey) -> Result<Option<VectorRecord>> {
        self.db
            .get(storage_key(key))
            .map_err(internal)?
            .map(|bytes| bincode::deserialize(&bytes).map_err(serialization))
            .transpose()
    }

    pub fn delete(&self, key: &VectorKey) -> Result<()> {
        self.db.delete(storage_key(key)).map_err(internal)
    }

    pub fn search(
        &self,
        query: &VectorQuery,
        profile: &EmbeddingModelProfile,
    ) -> Result<Vec<VectorHit>> {
        query.validate(profile).map_err(ZlfError::Internal)?;
        validate_query_vector(&query.values, profile).map_err(ZlfError::Internal)?;
        let includes = query.include_sources.iter().collect::<HashSet<_>>();
        let excludes = query.exclude_sources.iter().collect::<HashSet<_>>();
        let include_entities = query.include_entities.iter().collect::<HashSet<_>>();
        let exclude_entities = query.exclude_entities.iter().collect::<HashSet<_>>();
        let filters = DocumentFilters {
            includes: &includes,
            excludes: &excludes,
            include_entities: &include_entities,
            exclude_entities: &exclude_entities,
        };
        let mut hits = self
            .collect_search_hits(query, profile, filters)?
            .into_iter()
            .map(|hit| hit.0)
            .collect::<Vec<_>>();
        hits.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.key.cmp(&right.key))
        });
        Ok(hits)
    }

    fn collect_search_hits(
        &self,
        query: &VectorQuery,
        profile: &EmbeddingModelProfile,
        filters: DocumentFilters<'_>,
    ) -> Result<BinaryHeap<HeapHit>> {
        let mut heap = BinaryHeap::with_capacity(query.top_k + 1);
        let prefix = search_prefix(query);
        for item in self
            .db
            .iterator(IteratorMode::From(&prefix, rocksdb::Direction::Forward))
        {
            let (key, value) = item.map_err(internal)?;
            if !key.starts_with(&prefix) {
                break;
            }
            let record = bincode::deserialize(&value).map_err(serialization)?;
            consider_record(&mut heap, record, query, profile, filters)?;
        }
        Ok(heap)
    }

    pub fn search_page(
        &self,
        query: &VectorQuery,
        profile: &EmbeddingModelProfile,
        page: IndexPageRequest,
    ) -> Result<IndexPage<VectorHit>> {
        page.validate()
            .map_err(|error| ZlfError::Internal(error.to_string()))?;
        let mut query = query.clone();
        query.top_k = page.probe_limit();
        let hits = self.search(&query, profile)?;
        ranked_page(hits, page).map_err(|error| ZlfError::Internal(error.to_string()))
    }

    pub fn records_for_entity(
        &self,
        generation: &str,
        model_profile: &str,
        model_version: u32,
        entity: &zlf_core::EntityRef,
    ) -> Result<Vec<VectorRecord>> {
        let prefix = identity_prefix(generation, model_profile, model_version);
        let mut records = Vec::new();
        for item in self
            .db
            .iterator(IteratorMode::From(&prefix, rocksdb::Direction::Forward))
        {
            let (key, value) = item.map_err(internal)?;
            if !key.starts_with(&prefix) {
                break;
            }
            let record: VectorRecord = bincode::deserialize(&value).map_err(serialization)?;
            if record.key.document_id.entity == *entity {
                records.push(record);
            }
        }
        records.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(records)
    }

    pub fn count(&self, generation: &str, model_profile: &str, model_version: u32) -> Result<u64> {
        let prefix = identity_prefix(generation, model_profile, model_version);
        let mut count = 0;
        for item in self
            .db
            .iterator(IteratorMode::From(&prefix, rocksdb::Direction::Forward))
        {
            let (key, _) = item.map_err(internal)?;
            if !key.starts_with(&prefix) {
                break;
            }
            count += 1;
        }
        Ok(count)
    }
}

fn consider_record(
    heap: &mut BinaryHeap<HeapHit>,
    record: VectorRecord,
    query: &VectorQuery,
    profile: &EmbeddingModelProfile,
    filters: DocumentFilters<'_>,
) -> Result<()> {
    if !matches_filters(&record, filters, &query.metadata) {
        return Ok(());
    }
    let score = similarity(&query.values, &record.values, profile.metric)?;
    if query.threshold.is_some_and(|threshold| score < threshold) {
        return Ok(());
    }
    heap.push(HeapHit(VectorHit {
        key: record.key,
        score,
        source_version: record.source_version,
    }));
    if heap.len() > query.top_k {
        heap.pop();
    }
    Ok(())
}

fn similarity(query: &[f32], vector: &[f32], metric: VectorMetric) -> Result<f32> {
    let dot = query
        .iter()
        .zip(vector)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum::<f64>();
    let score = match metric {
        VectorMetric::DotProduct => dot,
        VectorMetric::Cosine => {
            let query_norm = norm(query);
            let vector_norm = norm(vector);
            if query_norm == 0.0 || vector_norm == 0.0 {
                return Err(ZlfError::Internal("cosine vector must be nonzero".into()));
            }
            dot / (query_norm * vector_norm)
        }
    };
    Ok(score as f32)
}

fn norm(values: &[f32]) -> f64 {
    values
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt()
}

#[derive(Debug)]
struct HeapHit(VectorHit);

impl PartialEq for HeapHit {
    fn eq(&self, other: &Self) -> bool {
        self.0.score == other.0.score && self.0.key == other.0.key
    }
}
impl Eq for HeapHit {}
impl PartialOrd for HeapHit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HeapHit {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .score
            .total_cmp(&self.0.score)
            .then_with(|| self.0.key.cmp(&other.0.key))
    }
}

fn storage_key(key: &VectorKey) -> Vec<u8> {
    let mut value = identity_prefix(&key.generation.0, &key.model_profile, key.model_version);
    value.extend_from_slice(&key.document_id.canonical_bytes());
    value
}

fn search_prefix(query: &VectorQuery) -> Vec<u8> {
    identity_prefix(
        &query.generation.0,
        &query.model_profile,
        query.model_version,
    )
}

fn identity_prefix(generation: &str, model_profile: &str, model_version: u32) -> Vec<u8> {
    let mut value = PREFIX.to_vec();
    push_part(&mut value, generation.as_bytes());
    push_part(&mut value, model_profile.as_bytes());
    push_part(&mut value, &model_version.to_be_bytes());
    value
}

fn push_part(target: &mut Vec<u8>, part: &[u8]) {
    target.extend_from_slice(&(part.len() as u32).to_be_bytes());
    target.extend_from_slice(part);
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
