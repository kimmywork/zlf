use std::collections::BTreeMap;

use zlf_core::EntityRef;
use zlf_index::{
    bge_m3_dense_v1, content_fingerprint, GenerationId, IndexDocumentId, VectorKey, VectorQuery,
    VectorRecord, VECTOR_RECORD_SCHEMA_VERSION,
};

#[test]
fn vector_identity_separates_generation_model_and_document_parts() {
    let base = key("g1", "body", "0");
    let mut generation = base.clone();
    generation.generation = GenerationId("g2".into());
    let mut model = base.clone();
    model.model_version = 2;
    let field = key("g1", "title", "0");
    assert_ne!(base.canonical_bytes(), generation.canonical_bytes());
    assert_ne!(base.canonical_bytes(), model.canonical_bytes());
    assert_ne!(base.canonical_bytes(), field.canonical_bytes());
}

#[test]
#[allow(clippy::too_many_lines)]
fn record_and_query_validation_reject_dimension_nonfinite_zero_and_normalization() {
    let mut profile = bge_m3_dense_v1();
    profile.dimension = 2;
    let mut record = VectorRecord {
        schema_version: VECTOR_RECORD_SCHEMA_VERSION,
        key: key("g1", "body", "0"),
        source_version: 1,
        content_fingerprint: content_fingerprint("text"),
        model_revision: profile.model_revision.clone(),
        metric: profile.metric,
        normalized: true,
        values: vec![1.0, 0.0],
        metadata: BTreeMap::new(),
    };
    assert!(record.validate(&profile).is_ok());
    record.values = vec![0.0, 0.0];
    assert!(record.validate(&profile).is_err());
    record.values = vec![f32::NAN, 0.0];
    assert!(record.validate(&profile).is_err());
    record.values = vec![1.0];
    assert!(record.validate(&profile).is_err());

    let mut query = VectorQuery {
        generation: GenerationId("g1".into()),
        model_profile: profile.id.clone(),
        model_version: profile.version,
        values: vec![1.0, 0.0],
        top_k: 10,
        threshold: Some(0.5),
        include_sources: Vec::new(),
        exclude_sources: Vec::new(),
        metadata: BTreeMap::new(),
    };
    assert!(query.validate(&profile).is_ok());
    query.top_k = 0;
    assert!(query.validate(&profile).is_err());
}

fn key(generation: &str, field: &str, chunk: &str) -> VectorKey {
    VectorKey {
        generation: GenerationId(generation.into()),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        document_id: IndexDocumentId::new(EntityRef::Node("node".into()), field, chunk),
    }
}
