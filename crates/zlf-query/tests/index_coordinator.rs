use std::collections::HashMap;

use chrono::Duration;
use zlf_core::{Node, Value};
use zlf_query::{
    CoordinatorConfig, FakeFailureMode, FakeIndexTarget, IndexCoordinator, IndexJobState,
};
use zlf_storage::Storage;

fn config() -> CoordinatorConfig {
    CoordinatorConfig {
        event_batch: 32,
        max_attempts: 3,
        lease: Duration::zero(),
        retry_delay: Duration::zero(),
    }
}

#[test]
fn stale_events_are_suppressed_and_latest_event_is_published() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage
        .create_node(Node::with_id("doc".into(), vec![], HashMap::new()))
        .unwrap();
    storage
        .update_node(
            "doc",
            HashMap::from([("body".into(), Value::String("latest".into()))]),
        )
        .unwrap();
    let coordinator = IndexCoordinator::new(&storage, config());
    coordinator.register_target("fake").unwrap();
    assert_eq!(coordinator.enqueue_available("fake").unwrap(), 2);
    let target = FakeIndexTarget::new("fake");
    assert!(coordinator.process_next("fake", &target).unwrap());
    assert!(coordinator.process_next("fake", &target).unwrap());
    assert_eq!(target.applied_sequences(&storage).unwrap(), vec![2]);
    assert_eq!(coordinator.progress("fake").unwrap().published_watermark, 2);
    assert_eq!(
        coordinator.jobs("fake").unwrap()[0].state,
        IndexJobState::Stale
    );
}

#[test]
fn crash_after_target_write_recovers_idempotently() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage
        .create_node(Node::with_id("doc".into(), vec![], HashMap::new()))
        .unwrap();
    let coordinator = IndexCoordinator::new(&storage, config());
    coordinator.register_target("fake").unwrap();
    coordinator.enqueue_available("fake").unwrap();
    let target = FakeIndexTarget::new("fake");
    target.fail(1, FakeFailureMode::RetryAfterWrite, 1);
    coordinator.process_next("fake", &target).unwrap();
    assert_eq!(coordinator.progress("fake").unwrap().published_watermark, 0);
    coordinator.process_next("fake", &target).unwrap();
    assert_eq!(target.applied_sequences(&storage).unwrap(), vec![1]);
    assert_eq!(coordinator.progress("fake").unwrap().published_watermark, 1);
}

#[test]
fn permanent_failure_dead_letters_and_blocks_publication() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage
        .create_node(Node::with_id("doc".into(), vec![], HashMap::new()))
        .unwrap();
    storage
        .create_node(Node::with_id("other".into(), vec![], HashMap::new()))
        .unwrap();
    let coordinator = IndexCoordinator::new(&storage, config());
    coordinator.register_target("fake").unwrap();
    coordinator.enqueue_available("fake").unwrap();
    let target = FakeIndexTarget::new("fake");
    target.fail(1, FakeFailureMode::Permanent, 1);
    coordinator.process_next("fake", &target).unwrap();
    assert_eq!(
        coordinator.jobs("fake").unwrap()[0].state,
        IndexJobState::Dead
    );
    assert!(!coordinator.process_next("fake", &target).unwrap());
    assert!(target.applied_sequences(&storage).unwrap().is_empty());
    assert_eq!(coordinator.progress("fake").unwrap().published_watermark, 0);
    let metrics = coordinator.metrics("fake").unwrap();
    assert_eq!(metrics.dead, 1);
    assert_eq!(metrics.lag, 2);
}

#[test]
fn retryable_failure_moves_to_dead_letter_at_attempt_limit() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage
        .create_node(Node::with_id("doc".into(), vec![], HashMap::new()))
        .unwrap();
    let coordinator = IndexCoordinator::new(&storage, config());
    coordinator.register_target("fake").unwrap();
    coordinator.enqueue_available("fake").unwrap();
    let target = FakeIndexTarget::new("fake");
    target.fail(1, FakeFailureMode::RetryBeforeWrite, 5);
    for _ in 0..3 {
        coordinator.process_next("fake", &target).unwrap();
    }
    assert_eq!(
        coordinator.jobs("fake").unwrap()[0].state,
        IndexJobState::Dead
    );
    assert_eq!(coordinator.metrics("fake").unwrap().retried, 2);
}

#[test]
fn pending_jobs_and_progress_survive_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    {
        let storage = Storage::open(&path).unwrap();
        storage
            .create_node(Node::with_id("doc".into(), vec![], HashMap::new()))
            .unwrap();
        let coordinator = IndexCoordinator::new(&storage, config());
        coordinator.register_target("fake").unwrap();
        coordinator.enqueue_available("fake").unwrap();
    }
    let storage = Storage::open_existing(&path).unwrap();
    let coordinator = IndexCoordinator::new(&storage, config());
    assert_eq!(coordinator.jobs("fake").unwrap().len(), 1);
    coordinator
        .process_next("fake", &FakeIndexTarget::new("fake"))
        .unwrap();
    assert_eq!(coordinator.progress("fake").unwrap().published_watermark, 1);
}

#[test]
fn compaction_waits_for_every_registered_target() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage
        .create_node(Node::with_id("doc".into(), vec![], HashMap::new()))
        .unwrap();
    let coordinator = IndexCoordinator::new(&storage, config());
    let first = FakeIndexTarget::new("first");
    let second = FakeIndexTarget::new("second");
    for name in ["first", "second"] {
        coordinator.register_target(name).unwrap();
        coordinator.enqueue_available(name).unwrap();
    }
    coordinator.process_next("first", &first).unwrap();
    assert_eq!(coordinator.compact_outbox().unwrap(), 0);
    assert_eq!(storage.mutation_events_after(0, 10).unwrap().len(), 1);
    coordinator.process_next("second", &second).unwrap();
    assert_eq!(coordinator.compact_outbox().unwrap(), 1);
    assert!(storage.mutation_events_after(0, 10).unwrap().is_empty());
}
