use std::collections::BTreeMap;

use zlf_core::EntityRef;
use zlf_index::{
    bge_m3_dense_v1, content_fingerprint, GenerationId, HnswVectorIndex, HnswVectorOptions,
    IndexDocumentId, VectorKey, VectorQuery, VectorRecord, VECTOR_RECORD_SCHEMA_VERSION,
};

#[test]
#[allow(clippy::too_many_lines)]
fn immutable_hnsw_build_search_replace_reopen_and_corruption_detection() {
    let temp = tempfile::tempdir().unwrap();
    let profile = profile();
    let options = HnswVectorOptions {
        connections: 8,
        ef_construction: 32,
        max_layer: 16,
        ef_search: 32,
    };
    let first = vec![record("a", [1.0, 0.0], 1), record("b", [0.0, 1.0], 1)];
    let index = HnswVectorIndex::build_and_publish(temp.path(), first, &profile, options).unwrap();
    assert_eq!(top(&index, &profile), "a");
    assert_eq!(index.identity().record_count, 2);
    drop(index);

    let reopened = HnswVectorIndex::open(temp.path()).unwrap();
    assert_eq!(top(&reopened, &profile), "a");
    let second = vec![record("a", [0.0, 1.0], 2), record("c", [1.0, 0.0], 1)];
    let replaced =
        HnswVectorIndex::build_and_publish(temp.path(), second, &profile, options).unwrap();
    assert_eq!(top(&replaced, &profile), "c");
    assert_eq!(
        replaced
            .search(&filtered_query(), &profile)
            .unwrap_err()
            .to_string(),
        "Feature not supported: filtered HNSW query requires exact fallback"
    );
    drop(replaced);

    let active = std::fs::read_to_string(temp.path().join("active")).unwrap();
    let data = temp
        .path()
        .join("publications")
        .join(active.trim())
        .join("vectors.hnsw.data");
    std::fs::remove_file(data).unwrap();
    assert!(HnswVectorIndex::open(temp.path()).is_err());
}

fn top(index: &HnswVectorIndex, profile: &zlf_index::EmbeddingModelProfile) -> String {
    index.search(&query(), profile).unwrap()[0]
        .key
        .document_id
        .entity
        .id()
        .to_string()
}

fn profile() -> zlf_index::EmbeddingModelProfile {
    let mut profile = bge_m3_dense_v1();
    profile.dimension = 2;
    profile
}

fn record(id: &str, values: [f32; 2], source_version: u64) -> VectorRecord {
    let profile = profile();
    VectorRecord {
        schema_version: VECTOR_RECORD_SCHEMA_VERSION,
        key: key(id),
        source_version,
        content_fingerprint: content_fingerprint(&format!("{id}-{source_version}")),
        model_revision: profile.model_revision,
        metric: profile.metric,
        normalized: profile.normalize,
        values: values.into(),
        metadata: BTreeMap::new(),
    }
}

fn query() -> VectorQuery {
    let profile = profile();
    VectorQuery {
        generation: GenerationId("g1".into()),
        model_profile: profile.id,
        model_version: profile.version,
        values: vec![1.0, 0.0],
        top_k: 1,
        threshold: None,
        include_sources: vec![],
        exclude_sources: vec![],
        include_entities: vec![],
        exclude_entities: vec![],
        fields: vec![],
        metadata: BTreeMap::new(),
    }
}

fn filtered_query() -> VectorQuery {
    let mut query = query();
    query.fields.push("body".into());
    query
}

fn key(id: &str) -> VectorKey {
    VectorKey {
        generation: GenerationId("g1".into()),
        model_profile: profile().id,
        model_version: profile().version,
        document_id: IndexDocumentId::new(EntityRef::Node(id.into()), "body", "0"),
    }
}
