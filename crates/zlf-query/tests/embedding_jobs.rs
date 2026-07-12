use chrono::{Duration, TimeZone, Utc};
use zlf_core::EntityRef;
use zlf_index::{
    content_fingerprint, EmbeddingJob, EmbeddingJobState, GenerationId, IndexDocumentId,
    EMBEDDING_JOB_SCHEMA_VERSION,
};
use zlf_query::EmbeddingJobStore;
use zlf_storage::Storage;

#[test]
#[allow(clippy::too_many_lines)]
fn jobs_dedupe_lease_retry_dead_complete_stale_and_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let storage = Storage::open(&path).unwrap();
    let store = EmbeddingJobStore::new(&storage);
    let now = Utc.with_ymd_and_hms(2026, 7, 11, 0, 0, 0).unwrap();
    let first = job("a", 1);
    assert!(store.enqueue(first.clone()).unwrap());
    assert!(!store.enqueue(first.clone()).unwrap());

    let claimed = store
        .claim_ready(now, 10, Duration::seconds(30))
        .unwrap()
        .remove(0);
    assert_eq!(claimed.attempts, 1);
    store
        .fail(&claimed, "timeout", now + Duration::seconds(5), 2, true)
        .unwrap();
    assert!(store
        .claim_ready(now, 10, Duration::seconds(30))
        .unwrap()
        .is_empty());
    let retried = store
        .claim_ready(now + Duration::seconds(5), 10, Duration::seconds(30))
        .unwrap()
        .remove(0);
    store
        .fail(&retried, &"x".repeat(200), now, 2, true)
        .unwrap();
    let dead = store.get(&first).unwrap().unwrap();
    assert_eq!(dead.state, EmbeddingJobState::Dead);
    assert_eq!(dead.last_error_class.unwrap().len(), 128);

    let second = job("b", 1);
    store.enqueue(second.clone()).unwrap();
    let leased = store
        .claim_ready(now, 1, Duration::seconds(1))
        .unwrap()
        .remove(0);
    let reclaimed = store
        .claim_ready(now + Duration::seconds(1), 1, Duration::seconds(1))
        .unwrap()
        .remove(0);
    assert_eq!(reclaimed.document_id, leased.document_id);
    store.complete(&reclaimed, now).unwrap();

    let third = job("c", 1);
    store.enqueue(third.clone()).unwrap();
    let leased = store
        .claim_ready(now, 1, Duration::seconds(1))
        .unwrap()
        .remove(0);
    store.stale(&leased, now).unwrap();
    drop(storage);
    let reopened = Storage::open_existing(&path).unwrap();
    let states = EmbeddingJobStore::new(&reopened)
        .list()
        .unwrap()
        .into_iter()
        .map(|job| job.state)
        .collect::<Vec<_>>();
    assert!(states.contains(&EmbeddingJobState::Completed));
    assert!(states.contains(&EmbeddingJobState::Stale));
    assert!(states.contains(&EmbeddingJobState::Dead));
}

fn job(id: &str, source_version: u64) -> EmbeddingJob {
    EmbeddingJob {
        schema_version: EMBEDDING_JOB_SCHEMA_VERSION,
        generation: GenerationId("g1".into()),
        document_id: IndexDocumentId::new(EntityRef::Node(id.into()), "body", "0"),
        source_version,
        content_fingerprint: content_fingerprint(id),
        model_profile: "bge_m3_dense_v1".into(),
        model_version: 1,
        expected_dimension: 2,
        attempts: 0,
        state: EmbeddingJobState::Pending,
        created_at: Utc::now(),
        lease_until: None,
        retry_at: None,
        completed_at: None,
        last_error_class: None,
    }
}
