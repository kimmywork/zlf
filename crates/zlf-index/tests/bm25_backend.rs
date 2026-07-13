use std::collections::BTreeMap;

use zlf_core::EntityRef;
use zlf_index::{
    bm25_term_score, content_fingerprint, BM25Index, DocumentChanges, IndexDocument,
    IndexDocumentId, IndexPageRequest, INDEX_DOCUMENT_SCHEMA_VERSION,
};

#[test]
fn real_bm25_replace_delete_limit_tie_and_reopen() {
    let temp = tempfile::tempdir().unwrap();
    {
        let index = BM25Index::open(temp.path()).unwrap();
        index
            .index_texts_batch(&[("b", "same"), ("a", "same"), ("old", "obsolete")])
            .unwrap();
        index.index_text("old", "current").unwrap();
        assert!(index.search("obsolete").unwrap().is_empty());
        assert_eq!(
            index
                .search_top_k("same", 2)
                .unwrap()
                .into_iter()
                .map(|hit| hit.0)
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        index.remove_all_for_node("old").unwrap();
    }
    let reopened = BM25Index::open(temp.path()).unwrap();
    assert!(reopened.search("current").unwrap().is_empty());
    assert_eq!(reopened.document_count(), 2);
}

#[test]
#[allow(clippy::too_many_lines)]
fn chunks_fields_weights_and_explanation_are_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let index = BM25Index::open(temp.path()).unwrap();
    index
        .index_document(&document("title", "0", "rust database"))
        .unwrap();
    index
        .index_document(&document("body", "1", "rust rust graph"))
        .unwrap();
    let title_only = index
        .search_document_top_k("rust", 10, &["title".into()], &BTreeMap::new(), true)
        .unwrap();
    assert_eq!(title_only.len(), 1);
    assert_eq!(title_only[0].document_id.field, "title");
    let explanation = title_only[0].explanation.as_ref().unwrap();
    assert_eq!(explanation.document_length, 2);
    assert!((explanation.average_document_length - 2.5).abs() < 0.001);
    assert_eq!(explanation.terms[0].term_frequency, 1);
    assert_eq!(explanation.terms[0].document_frequency, 2);
    assert!(explanation.terms[0].score > 0.0);
    assert!(index
        .search_document_top_k_filtered("rust", 10, &[], &["zh".into()], &BTreeMap::new(), false,)
        .unwrap()
        .is_empty());

    let weighted = index
        .search_document_top_k(
            "rust",
            10,
            &[],
            &BTreeMap::from([("title".into(), 10.0), ("body".into(), 1.0)]),
            false,
        )
        .unwrap();
    assert_eq!(weighted[0].document_id.field, "title");
    assert_eq!(weighted.len(), 2);
    index.remove_document(&weighted[0].document_id).unwrap();
    assert_eq!(index.search("rust").unwrap().len(), 1);
}

#[test]
#[allow(clippy::too_many_lines)]
fn entity_filter_and_ranked_pages_are_pushed_into_tantivy() {
    let temp = tempfile::tempdir().unwrap();
    let index = BM25Index::open(temp.path()).unwrap();
    index
        .index_texts_batch(&[("a", "rust rust"), ("b", "rust"), ("c", "rust")])
        .unwrap();
    let bound = index
        .search_document_top_k_for_entities("rust", 1, &[], &["c".into()], &BTreeMap::new(), false)
        .unwrap();
    assert_eq!(bound[0].document_id.entity.id(), "c");
    let first = index
        .search_document_page_for_entities(
            "rust",
            IndexPageRequest {
                offset: 0,
                page_size: 1,
                candidate_limit: 3,
            },
            &[],
            &[],
            &BTreeMap::new(),
            false,
        )
        .unwrap();
    let second = index
        .search_document_page_for_entities(
            "rust",
            IndexPageRequest {
                offset: first.next_offset.unwrap(),
                page_size: 1,
                candidate_limit: 3,
            },
            &[],
            &[],
            &BTreeMap::new(),
            false,
        )
        .unwrap();
    assert_ne!(first.items[0].document_id, second.items[0].document_id);
}

#[test]
fn tantivy_scores_match_the_independent_formula_fixture() {
    let temp = tempfile::tempdir().unwrap();
    let index = BM25Index::open(temp.path()).unwrap();
    index
        .apply_document_changes(&DocumentChanges {
            upserts: vec![
                document("body", "0", "rust rust graph"),
                document_for("other", "body", "0", "rust data graph"),
            ],
            deletes: Vec::new(),
        })
        .unwrap();
    let hits = index
        .search_document_top_k("rust", 2, &[], &BTreeMap::new(), false)
        .unwrap();
    let expected = [
        bm25_term_score(2, 2, 2, 3, 3.0, 1.2, 0.75),
        bm25_term_score(1, 2, 2, 3, 3.0, 1.2, 0.75),
    ];
    assert_eq!(hits[0].document_id.entity.id(), "doc");
    for (hit, score) in hits.iter().zip(expected) {
        assert!((f64::from(hit.score) - score).abs() < 0.001);
    }
}

#[test]
fn chinese_and_english_queries_share_versioned_analysis() {
    let temp = tempfile::tempdir().unwrap();
    let index = BM25Index::open(temp.path()).unwrap();
    index.index_text("alice", "Alice 是软件工程师").unwrap();
    assert_eq!(index.search("软件").unwrap()[0].0, "alice");
    assert_eq!(index.search("ALICE").unwrap()[0].0, "alice");
}

fn document(field: &str, chunk: &str, content: &str) -> IndexDocument {
    document_for("doc", field, chunk, content)
}

fn document_for(node: &str, field: &str, chunk: &str, content: &str) -> IndexDocument {
    IndexDocument {
        schema_version: INDEX_DOCUMENT_SCHEMA_VERSION,
        id: IndexDocumentId::new(EntityRef::Node(node.into()), field, chunk),
        source_version: 1,
        content_fingerprint: content_fingerprint(content),
        source_range: None,
        chunk_ordinal: chunk.parse().unwrap(),
        chunk_profile: "whole_field_v1".into(),
        language: Some("en".into()),
        content: content.into(),
    }
}
