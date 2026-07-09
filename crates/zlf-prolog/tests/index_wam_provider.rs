use chrono::{DateTime, Utc};
use zlf_index::{BM25Index, TemporalEntry, TemporalIndex, VectorEntry, VectorIndex};
use zlf_prolog::wam::{IndexFactProvider, WamRuntime};
use zlf_prolog::{PrologParser, Term};

#[test]
fn bm25_provider_uses_jieba_chinese_tokenization_in_prolog_query() {
    let dir = tempfile::tempdir().unwrap();
    let bm25 = BM25Index::open(dir.path().join("bm25")).unwrap();
    bm25.index_text("doc1", "软件工程师").unwrap();
    bm25.index_text("doc2", "苹果公司").unwrap();
    let provider = IndexFactProvider::new().with_bm25(&bm25);
    let runtime = WamRuntime::new(12);

    let solutions = runtime
        .query_all_with_provider(&term("bm25(\"软件\", Node, Score)"), &provider)
        .unwrap();

    assert_eq!(solutions[0].get("Node"), Some(&atom("doc1")));
    assert!(solutions[0].contains_key("Score"));
}

#[test]
fn vector_provider_exposes_similarity_to_prolog_query() {
    let dir = tempfile::tempdir().unwrap();
    let vector = VectorIndex::open(dir.path().join("vector")).unwrap();
    vector
        .add_entry(vector_entry("alice", vec![1.0, 0.0]))
        .unwrap();
    vector
        .add_entry(vector_entry("bob", vec![0.9, 0.1]))
        .unwrap();
    let provider = IndexFactProvider::new().with_vector(&vector);
    let runtime = WamRuntime::new(12);

    let solutions = runtime
        .query_all_with_provider(&term("vector_similar(alice, Node, Score)"), &provider)
        .unwrap();

    assert!(solutions
        .iter()
        .any(|row| row.get("Node") == Some(&atom("alice"))));
    assert!(solutions
        .iter()
        .any(|row| row.get("Node") == Some(&atom("bob"))));
}

#[test]
fn temporal_provider_exposes_date_queries_to_prolog() {
    let dir = tempfile::tempdir().unwrap();
    let temporal = TemporalIndex::open(dir.path().join("temporal")).unwrap();
    temporal
        .add_entry(temporal_entry("alice", "2026-01-01T00:00:00Z"))
        .unwrap();
    temporal
        .add_entry(temporal_entry("bob", "2026-02-01T00:00:00Z"))
        .unwrap();
    let provider = IndexFactProvider::new().with_temporal(&temporal);
    let runtime = WamRuntime::new(12);

    let on_date = runtime
        .query_all_with_provider(&term("temporal_on(\"2026-01-01\", Node)"), &provider)
        .unwrap();
    let in_range = runtime
        .query_all_with_provider(
            &term("temporal_between(\"2026-01-01\", \"2026-12-31\", Node)"),
            &provider,
        )
        .unwrap();

    assert_eq!(on_date[0].get("Node"), Some(&atom("alice")));
    assert_eq!(in_range.len(), 2);
}

fn vector_entry(node_id: &str, embedding: Vec<f32>) -> VectorEntry {
    VectorEntry {
        node_id: node_id.to_string(),
        embedding,
        model: "test".to_string(),
    }
}

fn temporal_entry(node_id: &str, timestamp: &str) -> TemporalEntry {
    TemporalEntry {
        node_id: node_id.to_string(),
        valid_from: DateTime::parse_from_rfc3339(timestamp)
            .unwrap()
            .with_timezone(&Utc),
        valid_to: None,
    }
}

fn atom(value: &str) -> Term {
    Term::Atom(value.to_string())
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}
