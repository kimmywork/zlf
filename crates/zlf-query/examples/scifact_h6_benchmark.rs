#[path = "scifact_h6/support.rs"]
mod support;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::time::Instant;

use chrono::Utc;
use serde_json::json;
use support::{
    document_text, latency_report, load_jsonl, load_qrels, max_input_chars, normalize, provider,
    Quality,
};
use zlf_core::EntityRef;
use zlf_embed::EmbeddingProvider;
use zlf_index::{
    bge_m3_dense_v1, content_fingerprint, BM25Index, DocumentChanges, EmbeddingModelProfile,
    IndexDocument, IndexDocumentId, RankedRetrieverHit, VectorKey, VectorRecord,
    VECTOR_RECORD_SCHEMA_VERSION,
};

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "data/benchmarks/scifact/h6-1000d-100q-v1".into()),
    );
    let dataset_name = std::env::args().nth(2).unwrap_or_else(|| "scifact".into());
    let language = std::env::args().nth(3).unwrap_or_else(|| "en".into());
    let corpus = load_jsonl(&root.join("corpus.jsonl"))?;
    let queries = load_jsonl(&root.join("queries.jsonl"))?;
    let qrels = load_qrels(&root.join("qrels.tsv"))?;
    let directory = tempfile::tempdir()?;
    let bm25 = BM25Index::open(directory.path().join("bm25"))?;
    let vectors = zlf_index::ExactVectorStore::open(directory.path().join("vectors"))?;
    let profile = bge_m3_dense_v1();
    let generation = zlf_index::GenerationId("public-retrieval-v1".into());
    let max_input_chars = max_input_chars();
    let texts = corpus
        .iter()
        .map(|row| document_text(row, max_input_chars))
        .collect::<Vec<_>>();
    let build_started = Instant::now();
    let documents = corpus
        .iter()
        .zip(&texts)
        .map(|(row, text)| index_document(row, text, &language))
        .collect::<Vec<_>>();
    bm25.apply_document_changes(&DocumentChanges {
        upserts: documents,
        deletes: Vec::new(),
    })?;
    let build_ms = build_started.elapsed();
    let provider = provider();
    let embedding_started = Instant::now();
    let embeddings = embed_batches(&provider, &profile, &texts).await?;
    let embedding_ms = embedding_started.elapsed();
    let records = corpus
        .iter()
        .zip(&texts)
        .zip(embeddings)
        .map(|((row, text), values)| vector_record(row, text, values, &profile, &generation))
        .collect::<Vec<_>>();
    vectors.apply(&records, &[], &profile)?;
    let mut query_vectors = Vec::new();
    let query_embedding_started = Instant::now();
    for query in &queries {
        query_vectors.push(
            provider
                .embed(query["text"].as_str().unwrap_or_default())
                .await?,
        );
    }
    let query_embedding_ms = query_embedding_started.elapsed();
    let mut qualities = BTreeMap::from([
        ("lexical".to_string(), Quality::default()),
        ("vector".to_string(), Quality::default()),
        ("hybrid".to_string(), Quality::default()),
    ]);
    let mut latencies = BTreeMap::<String, Vec<std::time::Duration>>::new();
    let mut candidate_totals = BTreeMap::from([
        ("lexical", 0_usize),
        ("vector", 0),
        ("fused_union", 0),
        ("answers", 0),
    ]);
    for (query, values) in queries.iter().zip(query_vectors) {
        let query_id = query["_id"].as_str().unwrap_or_default();
        let started = Instant::now();
        let lexical = page_lexical(&bm25, query["text"].as_str().unwrap_or_default())?;
        let lexical_latency = started.elapsed();
        latencies
            .entry("lexical".into())
            .or_default()
            .push(lexical_latency);
        let started = Instant::now();
        let vector = page_vector(&vectors, &profile, &generation, normalize(values)?)?;
        let vector_latency = started.elapsed();
        latencies
            .entry("vector".into())
            .or_default()
            .push(vector_latency);
        *candidate_totals.get_mut("lexical").unwrap() += lexical.len();
        *candidate_totals.get_mut("vector").unwrap() += vector.len();
        let fused_union = lexical
            .iter()
            .chain(&vector)
            .map(|hit| hit.document_id.clone())
            .collect::<BTreeSet<_>>()
            .len();
        *candidate_totals.get_mut("fused_union").unwrap() += fused_union;
        let started = Instant::now();
        let hybrid =
            zlf_index::reciprocal_rank_fusion(&lexical, &vector, 100, zlf_index::DEFAULT_RRF_K)?;
        let fusion_latency = started.elapsed();
        *candidate_totals.get_mut("answers").unwrap() += hybrid.len();
        latencies
            .entry("fusion_only".into())
            .or_default()
            .push(fusion_latency);
        latencies
            .entry("hybrid_end_to_end".into())
            .or_default()
            .push(lexical_latency + vector_latency + fusion_latency);
        let empty = HashMap::new();
        for (name, ranking) in [
            (
                "lexical",
                lexical
                    .iter()
                    .map(|hit| hit.document_id.entity.id().to_string())
                    .collect::<Vec<_>>(),
            ),
            (
                "vector",
                vector
                    .iter()
                    .map(|hit| hit.document_id.entity.id().to_string())
                    .collect::<Vec<_>>(),
            ),
            (
                "hybrid",
                hybrid
                    .iter()
                    .map(|hit| hit.document_id.entity.id().to_string())
                    .collect::<Vec<_>>(),
            ),
        ] {
            qualities
                .get_mut(name)
                .unwrap()
                .observe(&ranking, qrels.get(query_id).unwrap_or(&empty));
        }
    }
    let disk = support::directory_size(directory.path())?;
    let reports = qualities
        .into_iter()
        .map(|(name, quality)| (name, quality.report()))
        .collect::<serde_json::Map<_, _>>();
    let query_count = queries.len().max(1) as f64;
    let candidate_report = candidate_totals
        .into_iter()
        .map(|(name, total)| {
            (
                name.to_string(),
                json!({"total": total, "average_per_query": total as f64 / query_count}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"zlf-public-retrieval-raw-v1", "dataset":root, "dataset_name":dataset_name,
            "language":language, "documents":corpus.len(), "queries":queries.len(),
            "model":"bge-m3:latest", "dimension":profile.dimension, "metric":"cosine", "rrf_k":60,
            "max_input_chars":max_input_chars,
            "build_ms":build_ms.as_secs_f64()*1000.0, "embedding_ms":embedding_ms.as_secs_f64()*1000.0,
            "query_embedding_ms":query_embedding_ms.as_secs_f64()*1000.0, "retrieval_quality":reports,
            "retrieval_latency":latencies.into_iter().map(|(name, values)| (name, latency_report(&values))).collect::<serde_json::Map<_,_>>(),
            "candidate_counts":candidate_report, "candidate_limit":100, "answer_limit":100,
            "peak_materialized_answers":100, "disk_bytes":disk, "peak_rss_bytes":support::peak_rss_bytes(), "created_at":Utc::now(),
        }))?
    );
    Ok(())
}

fn index_document(row: &serde_json::Value, text: &str, language: &str) -> IndexDocument {
    IndexDocument {
        schema_version: zlf_index::INDEX_DOCUMENT_SCHEMA_VERSION,
        id: IndexDocumentId::new(
            EntityRef::Node(row["_id"].as_str().unwrap().into()),
            "body",
            "0",
        ),
        source_version: 1,
        content_fingerprint: content_fingerprint(text),
        source_range: None,
        chunk_ordinal: 0,
        chunk_profile: "scifact-h6".into(),
        language: Some(language.into()),
        content: text.into(),
    }
}

fn vector_record(
    row: &serde_json::Value,
    text: &str,
    values: Vec<f32>,
    profile: &EmbeddingModelProfile,
    generation: &zlf_index::GenerationId,
) -> VectorRecord {
    VectorRecord {
        schema_version: VECTOR_RECORD_SCHEMA_VERSION,
        key: VectorKey {
            generation: generation.clone(),
            model_profile: profile.id.clone(),
            model_version: profile.version,
            document_id: IndexDocumentId::new(
                EntityRef::Node(row["_id"].as_str().unwrap().into()),
                "body",
                "0",
            ),
        },
        source_version: 1,
        content_fingerprint: content_fingerprint(text),
        model_revision: profile.model_revision.clone(),
        metric: profile.metric,
        normalized: profile.normalize,
        values: normalize(values).unwrap(),
        metadata: BTreeMap::new(),
    }
}

async fn embed_batches<P: EmbeddingProvider>(
    provider: &P,
    _profile: &EmbeddingModelProfile,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, zlf_embed::EmbedError> {
    let mut output = Vec::new();
    for batch in texts.chunks(32) {
        let refs = batch.iter().map(String::as_str).collect::<Vec<_>>();
        output.extend(provider.embed_batch(&refs).await?);
    }
    Ok(output)
}

fn page_lexical(
    index: &BM25Index,
    text: &str,
) -> Result<Vec<RankedRetrieverHit>, Box<dyn std::error::Error>> {
    let hits = index.search_document_top_k(text, 100, &[], &BTreeMap::new(), false)?;
    Ok(hits
        .into_iter()
        .map(|hit| RankedRetrieverHit {
            document_id: hit.document_id,
            score: hit.score,
            generation: zlf_index::GenerationId("scifact-h6-v1".into()),
            watermark: 0,
            source_range: None,
        })
        .collect())
}

fn page_vector(
    store: &zlf_index::ExactVectorStore,
    profile: &EmbeddingModelProfile,
    generation: &zlf_index::GenerationId,
    values: Vec<f32>,
) -> Result<Vec<RankedRetrieverHit>, Box<dyn std::error::Error>> {
    let query = zlf_index::VectorQuery {
        generation: generation.clone(),
        model_profile: profile.id.clone(),
        model_version: profile.version,
        values,
        top_k: 100,
        threshold: None,
        include_sources: Vec::new(),
        exclude_sources: Vec::new(),
        include_entities: Vec::new(),
        exclude_entities: Vec::new(),
        fields: vec!["body".into()],
        metadata: BTreeMap::new(),
    };
    Ok(store
        .search(&query, profile)?
        .into_iter()
        .map(|hit| RankedRetrieverHit {
            document_id: hit.key.document_id,
            score: hit.score,
            generation: generation.clone(),
            watermark: 0,
            source_range: None,
        })
        .collect())
}
