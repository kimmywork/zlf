use zlf_core::EntityRef;
use zlf_index::{
    reciprocal_rank_fusion, GenerationId, IndexDocumentId, RankedRetrieverHit, ResultAggregation,
    RetrievalBudgets, RetrievalMode, RetrievalQuery, RetrievalRequest, TemporalFilter,
    DEFAULT_RRF_K,
};

#[test]
fn rrf_uses_ranks_deduplicates_and_breaks_fused_ties_by_document_identity() {
    let generation = GenerationId("g1".into());
    let a = document("a");
    let b = document("b");
    let lexical = vec![
        ranked(a.clone(), 100.0, &generation, 7),
        ranked(b.clone(), 50.0, &generation, 7),
        ranked(a.clone(), 1.0, &generation, 7),
    ];
    let vector = vec![
        ranked(b.clone(), 0.99, &generation, 8),
        ranked(a.clone(), 0.01, &generation, 8),
    ];
    let hits = reciprocal_rank_fusion(&lexical, &vector, 10, DEFAULT_RRF_K).unwrap();
    assert_eq!(
        hits.iter().map(|hit| &hit.document_id).collect::<Vec<_>>(),
        vec![&a, &b]
    );
    assert_eq!(hits[0].fused_rank, 1);
    assert_eq!(hits[0].lexical.as_ref().unwrap().rank, 1);
    assert_eq!(hits[0].vector.as_ref().unwrap().rank, 2);
    assert_eq!(hits[0].lexical.as_ref().unwrap().watermark, 7);
    assert_eq!(hits[0].vector.as_ref().unwrap().watermark, 8);
    assert_eq!(hits[0].fused_score, hits[1].fused_score);
}

#[test]
fn rrf_preserves_missing_retrievers_limits_and_rejects_invalid_inputs() {
    let generation = GenerationId("lexical".into());
    let lexical = vec![
        ranked(document("b"), 4.0, &generation, 2),
        ranked(document("a"), 3.0, &generation, 2),
    ];
    let hits = reciprocal_rank_fusion(&lexical, &[], 1, DEFAULT_RRF_K).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].lexical.is_some());
    assert!(hits[0].vector.is_none());
    assert!(reciprocal_rank_fusion(&lexical, &[], 0, DEFAULT_RRF_K).is_err());
    assert!(reciprocal_rank_fusion(&lexical, &[], 1, f64::NAN).is_err());
    let invalid = vec![ranked(document("bad"), f32::NAN, &generation, 2)];
    assert!(reciprocal_rank_fusion(&invalid, &[], 1, DEFAULT_RRF_K).is_err());
}

#[test]
#[allow(clippy::too_many_lines)]
fn request_contract_rejects_unbounded_and_incompatible_shapes() {
    let valid = RetrievalRequest {
        query: RetrievalQuery::Text {
            text: "knowledge".into(),
        },
        mode: RetrievalMode::Hybrid,
        profiles: vec!["default".into()],
        top_k: 10,
        budgets: RetrievalBudgets {
            candidate_k: 100,
            page_size: 20,
            max_pages: 5,
            max_answers: 10,
        },
        threshold: None,
        fields: vec!["body".into()],
        model_generation: Some(GenerationId("model".into())),
        analyzer_generation: Some(GenerationId("analyzer".into())),
        temporal_filter: Some(TemporalFilter::ValidOverlaps {
            start_micros: 10,
            end_micros: 20,
        }),
        exclude_source: None,
        graph_filter_goal: Some("allowed(User, Entity)".into()),
        aggregation: ResultAggregation::Document,
        explain: true,
    };
    valid.validate().unwrap();
    let mut invalid = valid.clone();
    invalid.budgets.page_size = 101;
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.temporal_filter = Some(TemporalFilter::EventRange {
        start_micros: 20,
        end_micros: 20,
    });
    assert!(invalid.validate().is_err());
    invalid = valid;
    invalid.query = RetrievalQuery::Text { text: " ".into() };
    assert!(invalid.validate().is_err());
}

fn ranked(
    document_id: IndexDocumentId,
    score: f32,
    generation: &GenerationId,
    watermark: u64,
) -> RankedRetrieverHit {
    RankedRetrieverHit {
        document_id,
        score,
        generation: generation.clone(),
        watermark,
        source_range: None,
    }
}

fn document(id: &str) -> IndexDocumentId {
    IndexDocumentId::new(EntityRef::Node(id.into()), "body", "0")
}
