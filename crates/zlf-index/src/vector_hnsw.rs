use std::fs;
use std::path::Path;

use hnsw_rs::prelude::{AnnT, DistCosine, Hnsw, HnswIo};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zlf_core::{Result, ZlfError};

use crate::{EmbeddingModelProfile, VectorHit, VectorMetric, VectorQuery, VectorRecord};

const BASENAME: &str = "vectors";
const ACTIVE: &str = "active";
const IDENTITY: &str = "identity.bin";
const RECORDS: &str = "records.bin";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HnswVectorOptions {
    pub connections: usize,
    pub ef_construction: usize,
    pub max_layer: usize,
    pub ef_search: usize,
}

impl Default for HnswVectorOptions {
    fn default() -> Self {
        Self {
            connections: 48,
            ef_construction: 400,
            max_layer: 16,
            ef_search: 2048,
        }
    }
}

impl HnswVectorOptions {
    pub fn validate(self) -> Result<Self> {
        if self.connections == 0
            || self.connections > 128
            || self.ef_construction < self.connections
            || self.ef_construction > 4096
            || self.max_layer != 16
            || self.ef_search == 0
            || self.ef_search > 16_384
        {
            return Err(ZlfError::Internal("invalid bounded HNSW options".into()));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HnswVectorIdentity {
    pub schema_version: u32,
    pub generation: String,
    pub model_profile: String,
    pub model_version: u32,
    pub model_revision: String,
    pub dimension: usize,
    pub record_count: usize,
    pub source_checksum: String,
    pub options: HnswVectorOptions,
}

pub struct HnswVectorIndex {
    index: Hnsw<'static, f32, DistCosine>,
    records: Vec<VectorRecord>,
    identity: HnswVectorIdentity,
}

impl HnswVectorIndex {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let publication = fs::read_to_string(root.join(ACTIVE)).map_err(internal)?;
        let path = root.join("publications").join(publication.trim());
        let identity = deserialize(&fs::read(path.join(IDENTITY)).map_err(internal)?)?;
        let records: Vec<VectorRecord> =
            deserialize(&fs::read(path.join(RECORDS)).map_err(internal)?)?;
        validate_publication(&path, &identity, &records)?;
        let index = load_owned_index(&path)?;
        Ok(Self {
            index,
            records,
            identity,
        })
    }

    pub fn build_and_publish(
        root: impl AsRef<Path>,
        mut records: Vec<VectorRecord>,
        profile: &EmbeddingModelProfile,
        options: HnswVectorOptions,
    ) -> Result<Self> {
        let options = options.validate()?;
        validate_records(&records, profile)?;
        records.sort_by(|left, right| left.key.cmp(&right.key));
        let identity = identity(&records, profile, options)?;
        let root = root.as_ref();
        let name = identity.source_checksum.clone();
        let publication = root.join("publications").join(&name);
        if !publication.is_dir() {
            build_publication(root, &publication, &records, &identity)?;
        }
        fs::create_dir_all(root).map_err(internal)?;
        let active = root.join(format!("{ACTIVE}.{}", std::process::id()));
        fs::write(&active, &name).map_err(internal)?;
        fs::rename(active, root.join(ACTIVE)).map_err(internal)?;
        Self::open(root)
    }

    pub fn identity(&self) -> &HnswVectorIdentity {
        &self.identity
    }

    pub fn matches_records(
        &self,
        records: &[VectorRecord],
        profile: &EmbeddingModelProfile,
    ) -> bool {
        identity(records, profile, self.identity.options).is_ok_and(|value| value == self.identity)
    }

    #[allow(clippy::too_many_lines)]
    pub fn search(
        &self,
        query: &VectorQuery,
        profile: &EmbeddingModelProfile,
    ) -> Result<Vec<VectorHit>> {
        query.validate(profile).map_err(ZlfError::Internal)?;
        if !query.include_sources.is_empty()
            || !query.exclude_sources.is_empty()
            || !query.include_entities.is_empty()
            || !query.exclude_entities.is_empty()
            || !query.fields.is_empty()
            || !query.metadata.is_empty()
        {
            return Err(ZlfError::UnsupportedFeature(
                "filtered HNSW query requires exact fallback".into(),
            ));
        }
        if !self.matches(query, profile) {
            return Err(ZlfError::Internal("stale HNSW publication identity".into()));
        }
        let mut hits = self
            .index
            .search(&query.values, query.top_k, self.identity.options.ef_search)
            .into_iter()
            .filter_map(|hit| {
                let record = self.records.get(hit.d_id)?;
                let score = 1.0 - hit.distance;
                (!query.threshold.is_some_and(|threshold| score < threshold)).then(|| VectorHit {
                    key: record.key.clone(),
                    score,
                    source_version: record.source_version,
                })
            })
            .collect::<Vec<_>>();
        hits.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.key.cmp(&right.key))
        });
        Ok(hits)
    }

    fn matches(&self, query: &VectorQuery, profile: &EmbeddingModelProfile) -> bool {
        self.identity.generation == query.generation.0
            && self.identity.model_profile == query.model_profile
            && self.identity.model_version == query.model_version
            && self.identity.model_revision == profile.model_revision
            && self.identity.dimension == profile.dimension
    }
}

fn load_owned_index(path: &Path) -> Result<Hnsw<'static, f32, DistCosine>> {
    let mut io = HnswIo::new(path, BASENAME);
    let index = io.load_hnsw().map_err(internal)?;
    // SAFETY: default HnswIo reload does not enable mmap, so points own Vec<f32>
    // values and do not borrow HnswIo. hnsw_rs exposes the same lifetime for both
    // modes; pinning 0.3.4 and keeping mmap disabled makes this ownership explicit.
    Ok(unsafe {
        std::mem::transmute::<Hnsw<'_, f32, DistCosine>, Hnsw<'static, f32, DistCosine>>(index)
    })
}

fn build_publication(
    root: &Path,
    publication: &Path,
    records: &[VectorRecord],
    identity: &HnswVectorIdentity,
) -> Result<()> {
    let temporary = root.join(format!("building-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary).map_err(internal)?;
    }
    fs::create_dir_all(&temporary).map_err(internal)?;
    let mut index = Hnsw::<f32, DistCosine>::new(
        identity.options.connections,
        records.len().max(1),
        identity.options.max_layer,
        identity.options.ef_construction,
        DistCosine {},
    );
    let references = records
        .iter()
        .enumerate()
        .map(|(id, record)| (&record.values, id))
        .collect::<Vec<_>>();
    index.parallel_insert(&references);
    index.set_searching_mode(true);
    index.file_dump(&temporary, BASENAME).map_err(internal)?;
    fs::write(temporary.join(RECORDS), serialize(records)?).map_err(internal)?;
    fs::write(temporary.join(IDENTITY), serialize(identity)?).map_err(internal)?;
    fs::create_dir_all(root.join("publications")).map_err(internal)?;
    fs::rename(temporary, publication).map_err(internal)
}

fn identity(
    records: &[VectorRecord],
    profile: &EmbeddingModelProfile,
    options: HnswVectorOptions,
) -> Result<HnswVectorIdentity> {
    let first = records
        .first()
        .ok_or_else(|| ZlfError::Internal("cannot build empty HNSW publication".into()))?;
    let mut digest = Sha256::new();
    digest.update(options.connections.to_be_bytes());
    digest.update(options.ef_construction.to_be_bytes());
    digest.update(options.max_layer.to_be_bytes());
    digest.update(options.ef_search.to_be_bytes());
    for record in records {
        digest.update(record.key.canonical_bytes());
        digest.update(record.source_version.to_be_bytes());
        digest.update(record.content_fingerprint.0.as_bytes());
    }
    Ok(HnswVectorIdentity {
        schema_version: SCHEMA_VERSION,
        generation: first.key.generation.0.clone(),
        model_profile: profile.id.clone(),
        model_version: profile.version,
        model_revision: profile.model_revision.clone(),
        dimension: profile.dimension,
        record_count: records.len(),
        source_checksum: format!("{:x}", digest.finalize()),
        options,
    })
}

fn validate_records(records: &[VectorRecord], profile: &EmbeddingModelProfile) -> Result<()> {
    if profile.metric != VectorMetric::Cosine || !profile.normalize || records.is_empty() {
        return Err(ZlfError::UnsupportedFeature(
            "HNSW requires non-empty normalized cosine vectors".into(),
        ));
    }
    for record in records {
        record.validate(profile).map_err(ZlfError::Internal)?;
    }
    let first = &records[0].key;
    if records.iter().any(|record| {
        record.key.generation != first.generation
            || record.key.model_profile != first.model_profile
            || record.key.model_version != first.model_version
    }) {
        return Err(ZlfError::Internal("mixed HNSW identity".into()));
    }
    Ok(())
}

fn validate_publication(
    path: &Path,
    identity: &HnswVectorIdentity,
    records: &[VectorRecord],
) -> Result<()> {
    if identity.schema_version != SCHEMA_VERSION || identity.record_count != records.len() {
        return Err(ZlfError::Internal("invalid HNSW publication".into()));
    }
    for suffix in ["graph", "data"] {
        if !path.join(format!("{BASENAME}.hnsw.{suffix}")).is_file() {
            return Err(ZlfError::Internal("incomplete HNSW publication".into()));
        }
    }
    Ok(())
}

fn serialize<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    bincode::serialize(value).map_err(|error| ZlfError::Serialization(error.to_string()))
}

fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    bincode::deserialize(bytes).map_err(|error| ZlfError::Serialization(error.to_string()))
}

fn internal(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Internal(error.to_string())
}
