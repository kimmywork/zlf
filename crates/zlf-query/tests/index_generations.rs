use std::collections::HashMap;
use std::time::Duration;

use chrono::Utc;
use zlf_core::Node;
use zlf_index::{GenerationId, GenerationMetadata, GenerationState, GENERATION_SCHEMA_VERSION};
use zlf_query::{
    wait_for_indexes, CoordinatorConfig, FakeIndexTarget, GenerationManager, IndexCoordinator,
};
use zlf_storage::Storage;

#[test]
fn failed_build_leaves_active_generation_and_reopen_preserves_state() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    {
        let storage = Storage::open(&path).unwrap();
        let manager = GenerationManager::new(&storage);
        publish(&manager, generation("g1"));
        let second = generation("g2");
        manager.create(&second).unwrap();
        manager.start_build("fake", &second.id).unwrap();
        manager.checkpoint("fake", &second.id, 7).unwrap();
        manager
            .fail("fake", &second.id, "validation failed")
            .unwrap();
        assert_eq!(manager.active("fake").unwrap().unwrap().id.0, "g1");
    }
    let storage = Storage::open_existing(&path).unwrap();
    let manager = GenerationManager::new(&storage);
    assert_eq!(manager.active("fake").unwrap().unwrap().id.0, "g1");
    let failed = manager
        .get("fake", &GenerationId("g2".into()))
        .unwrap()
        .unwrap();
    assert_eq!(failed.state, GenerationState::Failed);
    assert_eq!(failed.build_checkpoint, 7);
}

#[test]
fn activation_is_atomic_and_retention_keeps_active_plus_previous() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    let manager = GenerationManager::new(&storage);
    publish(&manager, generation("g1"));
    publish(&manager, generation("g2"));
    publish(&manager, generation("g3"));
    assert_eq!(manager.active("fake").unwrap().unwrap().id.0, "g3");
    assert_eq!(manager.prune("fake", Utc::now()).unwrap(), 1);
    let ids = manager
        .list("fake")
        .unwrap()
        .into_iter()
        .map(|metadata| metadata.id.0)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["g2", "g3"]);
}

#[test]
fn invalid_activation_fails_and_expired_failure_metadata_is_pruned() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    let manager = GenerationManager::new(&storage);
    let mut failed = generation("failed");
    failed.created_at = Utc::now() - chrono::Duration::days(31);
    manager.create(&failed).unwrap();
    assert!(manager.activate("fake", &failed.id).is_err());
    manager.fail("fake", &failed.id, "broken").unwrap();
    assert_eq!(manager.prune("fake", Utc::now()).unwrap(), 1);
    assert!(manager.get("fake", &failed.id).unwrap().is_none());
}

#[test]
fn status_and_wait_report_reached_and_pending_targets() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage
        .create_node(Node::with_id("doc".into(), vec![], HashMap::new()))
        .unwrap();
    let coordinator = IndexCoordinator::new(&storage, CoordinatorConfig::default());
    coordinator.register_target("fake").unwrap();
    coordinator.enqueue_available("fake").unwrap();
    coordinator
        .process_next("fake", &FakeIndexTarget::new("fake"))
        .unwrap();
    let reached = wait_for_indexes(&storage, &["fake".into()], 1, Duration::ZERO).unwrap();
    assert!(reached.reached);
    let pending = wait_for_indexes(&storage, &["missing".into()], 1, Duration::ZERO).unwrap();
    assert!(!pending.reached);
    assert_eq!(pending.pending_targets, vec!["missing"]);
    let status = GenerationManager::new(&storage).status("fake").unwrap();
    assert_eq!(status.published_watermark, 1);
}

fn publish(manager: &GenerationManager<'_>, metadata: GenerationMetadata) {
    manager.create(&metadata).unwrap();
    manager.start_build("fake", &metadata.id).unwrap();
    manager.checkpoint("fake", &metadata.id, 10).unwrap();
    manager.begin_validation("fake", &metadata.id).unwrap();
    manager
        .validation_passed("fake", &metadata.id, 3, "checksum")
        .unwrap();
    manager.activate("fake", &metadata.id).unwrap();
}

fn generation(id: &str) -> GenerationMetadata {
    GenerationMetadata {
        schema_version: GENERATION_SCHEMA_VERSION,
        id: GenerationId(id.into()),
        target: "fake".into(),
        profile_name: "knowledge".into(),
        profile_version: 1,
        backend_schema: "fake-v1".into(),
        source_snapshot_sequence: 0,
        state: GenerationState::Draft,
        build_checkpoint: 0,
        document_count: 0,
        checksum: None,
        failure: None,
        created_at: Utc::now(),
        validated_at: None,
    }
}
