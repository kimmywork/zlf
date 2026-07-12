use std::collections::BTreeMap;

use zlf_core::EntityRef;
use zlf_index::{
    content_fingerprint, BM25Index, IndexDocument, IndexDocumentId, INDEX_DOCUMENT_SCHEMA_VERSION,
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
fn chinese_and_english_queries_share_versioned_analysis() {
    let temp = tempfile::tempdir().unwrap();
    let index = BM25Index::open(temp.path()).unwrap();
    index.index_text("alice", "Alice 是软件工程师").unwrap();
    assert_eq!(index.search("软件").unwrap()[0].0, "alice");
    assert_eq!(index.search("ALICE").unwrap()[0].0, "alice");
}

fn document(field: &str, chunk: &str, content: &str) -> IndexDocument {
    IndexDocument {
        schema_version: INDEX_DOCUMENT_SCHEMA_VERSION,
        id: IndexDocumentId::new(EntityRef::Node("doc".into()), field, chunk),
        source_version: 1,
        content_fingerprint: content_fingerprint(content),
        source_range: None,
        chunk_ordinal: chunk.parse().unwrap(),
        chunk_profile: "whole_field_v1".into(),
        content: content.into(),
    }
}
