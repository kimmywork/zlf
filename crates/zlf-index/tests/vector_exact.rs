use std::collections::BTreeMap;

use zlf_core::EntityRef;
use zlf_index::{
    bge_m3_dense_v1, content_fingerprint, ExactVectorStore, GenerationId, IndexDocumentId,
    IndexPageRequest, VectorKey, VectorQuery, VectorRecord, VECTOR_RECORD_SCHEMA_VERSION,
};

#[test]
#[allow(clippy::too_many_lines)]
fn exact_search_filters_ranks_ties_updates_deletes_and_reopens() {
    let temp = tempfile::tempdir().unwrap();
    let profile = profile();
    {
        let store = ExactVectorStore::open(temp.path()).unwrap();
        store
            .put(&record("a", [1.0, 0.0], "keep"), &profile)
            .unwrap();
        store
            .put(&record("b", [1.0, 0.0], "drop"), &profile)
            .unwrap();
        store
            .put(&record("c", [0.0, 1.0], "keep"), &profile)
            .unwrap();
        store
            .put(&record_in("g2", "a", [1.0, 0.0], "keep"), &profile)
            .unwrap();
        let mut query = query();
        assert_eq!(ids(store.search(&query, &profile).unwrap()), ["a", "b"]);
        query.metadata.insert("class".into(), "keep".into());
        assert_eq!(ids(store.search(&query, &profile).unwrap()), ["a", "c"]);
        query.exclude_sources.push(document_id("a"));
        assert_eq!(ids(store.search(&query, &profile).unwrap()), ["c"]);
        query.exclude_sources.clear();
        query.include_sources.push(document_id("c"));
        assert_eq!(ids(store.search(&query, &profile).unwrap()), ["c"]);
        query.include_sources.clear();
        query.include_entities.push(EntityRef::Node("c".into()));
        assert_eq!(ids(store.search(&query, &profile).unwrap()), ["c"]);
        query.include_entities.clear();
        let page = store
            .search_page(
                &query,
                &profile,
                IndexPageRequest {
                    offset: 1,
                    page_size: 1,
                    candidate_limit: 2,
                },
            )
            .unwrap();
        assert_eq!(page.items[0].key.document_id.entity.id(), "c");
        assert!(page.candidate_budget_exhausted);

        store
            .put(&record("a", [0.0, 1.0], "keep"), &profile)
            .unwrap();
        store.delete(&key("g1", "b")).unwrap();
        assert_eq!(store.count("g1", &profile.id, profile.version).unwrap(), 2);
    }
    let reopened = ExactVectorStore::open(temp.path()).unwrap();
    assert_eq!(
        ids(reopened.search(&query(), &profile).unwrap()),
        ["a", "c"]
    );
    assert!(reopened.get(&key("g1", "b")).unwrap().is_none());
}

#[test]
fn exact_search_rejects_invalid_vectors_and_matches_f64_cosine_oracle() {
    let temp = tempfile::tempdir().unwrap();
    let profile = profile();
    let store = ExactVectorStore::open(temp.path()).unwrap();
    let mut invalid = record("bad", [0.0, 0.0], "keep");
    assert!(store.put(&invalid, &profile).is_err());
    invalid.values = vec![f32::NAN, 0.0];
    assert!(store.put(&invalid, &profile).is_err());
    invalid.values = vec![1.0];
    assert!(store.put(&invalid, &profile).is_err());

    let unit = 1.0_f32 / 2.0_f32.sqrt();
    store
        .put(&record("diag", [unit, unit], "keep"), &profile)
        .unwrap();
    let hit = store.search(&query(), &profile).unwrap().remove(0);
    let oracle = f64::from(unit);
    assert!((f64::from(hit.score) - oracle).abs() < 1e-6);
}

#[test]
fn dot_product_uses_the_profile_metric() {
    let temp = tempfile::tempdir().unwrap();
    let mut profile = profile();
    profile.metric = zlf_index::VectorMetric::DotProduct;
    profile.normalize = false;
    let store = ExactVectorStore::open(temp.path()).unwrap();
    let mut item = record("dot", [2.0, 3.0], "keep");
    item.metric = profile.metric;
    item.normalized = false;
    store.put(&item, &profile).unwrap();
    let mut query = query();
    query.values = vec![4.0, 5.0];
    let hit = store.search(&query, &profile).unwrap().remove(0);
    assert!((hit.score - 23.0).abs() < f32::EPSILON);
}

fn profile() -> zlf_index::EmbeddingModelProfile {
    let mut profile = bge_m3_dense_v1();
    profile.dimension = 2;
    profile
}

fn record(id: &str, values: [f32; 2], class: &str) -> VectorRecord {
    record_in("g1", id, values, class)
}

fn record_in(generation: &str, id: &str, values: [f32; 2], class: &str) -> VectorRecord {
    let profile = profile();
    VectorRecord {
        schema_version: VECTOR_RECORD_SCHEMA_VERSION,
        key: key(generation, id),
        source_version: 1,
        content_fingerprint: content_fingerprint(id),
        model_revision: profile.model_revision,
        metric: profile.metric,
        normalized: profile.normalize,
        values: values.into(),
        metadata: BTreeMap::from([("class".into(), class.into())]),
    }
}

fn query() -> VectorQuery {
    let profile = profile();
    VectorQuery {
        generation: GenerationId("g1".into()),
        model_profile: profile.id,
        model_version: profile.version,
        values: vec![1.0, 0.0],
        top_k: 2,
        threshold: Some(0.0),
        include_sources: Vec::new(),
        exclude_sources: Vec::new(),
        include_entities: Vec::new(),
        exclude_entities: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

fn key(generation: &str, id: &str) -> VectorKey {
    VectorKey {
        generation: GenerationId(generation.into()),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        document_id: document_id(id),
    }
}

fn document_id(id: &str) -> IndexDocumentId {
    IndexDocumentId::new(EntityRef::Node(id.into()), "body", "0")
}

fn ids(hits: Vec<zlf_index::VectorHit>) -> Vec<String> {
    hits.into_iter()
        .map(|hit| hit.key.document_id.entity.id().to_string())
        .collect()
}
