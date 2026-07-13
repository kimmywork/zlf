use std::collections::{BTreeMap, HashMap};

use chrono::Utc;
use zlf_core::{Node, Value};
use zlf_index::{
    parse_utc_micros, EntityMatcher, EventTimeStore, FieldIndexOptions, GenerationId,
    IndexProfileArtifact, TemporalRole, ValidityStore, INDEX_PROFILE_SCHEMA_VERSION,
};
use zlf_query::{
    CoordinatorConfig, IndexCoordinator, IndexProfileStore, TemporalIndexTarget, ZlfDatabase,
};
use zlf_storage::Storage;

#[test]
#[allow(clippy::too_many_lines)]
fn database_facade_exposes_event_and_validity_wam_predicates() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
    db.put_index_profile(&profile()).unwrap();
    db.activate_index_profile("temporal", 1).unwrap();
    db.add_node(node(
        "2026-01-01T10:00:00Z",
        "2026-01-01T00:00:00Z",
        Some("2026-02-01T00:00:00Z"),
    ))
    .unwrap();
    assert_eq!(
        db.query_prolog("? temporal_on(\"2026-01-01\", Node).")
            .unwrap()[0]["Node"],
        "doc"
    );
    assert_eq!(
        db.query_prolog("? temporal_between(\"2026-01-01\", \"2026-01-02\", Node).")
            .unwrap()[0]["Node"],
        "doc"
    );
    assert_eq!(
        db.query_prolog("? valid_at(\"2026-01-15T00:00:00Z\", Node).")
            .unwrap()[0]["Node"],
        "doc"
    );
    assert_eq!(
        db.query_prolog(
            "? valid_overlaps(\"2026-01-31T00:00:00Z\", \"2026-03-01T00:00:00Z\", Node)."
        )
        .unwrap()[0]["Node"],
        "doc"
    );
    assert!(db
        .query_prolog("? valid_at(\"2026-02-01T00:00:00Z\", Node).")
        .unwrap()
        .is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn profile_declared_event_and_validity_records_converge_on_update_delete_replay() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    let events = EventTimeStore::open(temp.path().join("events")).unwrap();
    let validities = ValidityStore::open(temp.path().join("validities")).unwrap();
    storage
        .create_node(node("2026-01-01T10:00:00Z", "2026-01-01", None))
        .unwrap();
    let profiles = IndexProfileStore::new(&storage);
    profiles.put(&profile()).unwrap();
    profiles.activate("temporal", 1).unwrap();
    let coordinator = IndexCoordinator::new(&storage, CoordinatorConfig::default());
    coordinator.register_target("temporal").unwrap();
    coordinator.enqueue_available("temporal").unwrap();
    let generation = GenerationId("g1".into());
    let target = TemporalIndexTarget::new(&events, &validities, generation.clone());
    while coordinator.process_next("temporal", &target).unwrap() {}

    let day = events.day(&generation, "2026-01-01", 10).unwrap();
    assert_eq!(day.records.len(), 1);
    assert_eq!(
        validities
            .valid_at(&generation, parse_utc_micros("2030-01-01").unwrap(), 10)
            .unwrap()
            .records
            .len(),
        1
    );

    storage
        .update_node(
            "doc",
            HashMap::from([
                (
                    "occurred_at".into(),
                    Value::Array(vec![
                        Value::String("2026-01-02T10:00:00Z".into()),
                        Value::String("2026-01-02T11:00:00Z".into()),
                    ]),
                ),
                ("valid_from".into(), Value::String("2026-02-01".into())),
                ("valid_to".into(), Value::String("2026-03-01".into())),
            ]),
        )
        .unwrap();
    coordinator.enqueue_available("temporal").unwrap();
    coordinator.process_next("temporal", &target).unwrap();
    assert!(events
        .day(&generation, "2026-01-01", 10)
        .unwrap()
        .records
        .is_empty());
    assert_eq!(
        events
            .day(&generation, "2026-01-02", 10)
            .unwrap()
            .records
            .len(),
        2
    );
    assert!(validities
        .valid_at(&generation, parse_utc_micros("2030-01-01").unwrap(), 10)
        .unwrap()
        .records
        .is_empty());
    assert_eq!(
        validities
            .valid_at(&generation, parse_utc_micros("2026-02-15").unwrap(), 10)
            .unwrap()
            .records
            .len(),
        1
    );

    storage.delete_node("doc").unwrap();
    coordinator.enqueue_available("temporal").unwrap();
    coordinator.process_next("temporal", &target).unwrap();
    assert!(events
        .day(&generation, "2026-01-02", 10)
        .unwrap()
        .records
        .is_empty());
    assert!(validities
        .valid_at(&generation, parse_utc_micros("2026-02-15").unwrap(), 10)
        .unwrap()
        .records
        .is_empty());
    coordinator.enqueue_available("temporal").unwrap();
    assert!(!coordinator.process_next("temporal", &target).unwrap());
}

fn node(event: &str, from: &str, to: Option<&str>) -> Node {
    let mut properties = HashMap::from([
        ("occurred_at".into(), Value::String(event.into())),
        ("valid_from".into(), Value::String(from.into())),
    ]);
    if let Some(to) = to {
        properties.insert("valid_to".into(), Value::String(to.into()));
    }
    Node::with_id("doc".into(), vec!["record".into()], properties)
}

fn profile() -> IndexProfileArtifact {
    let options = |role| FieldIndexOptions {
        bm25: None,
        vector: None,
        temporal: Some(role),
    };
    let mut profile = IndexProfileArtifact {
        schema_version: INDEX_PROFILE_SCHEMA_VERSION,
        name: "temporal".into(),
        version: 1,
        source_hash: String::new(),
        matcher: EntityMatcher::NodeLabels {
            labels: vec!["record".into()],
        },
        fields: BTreeMap::from([
            ("occurred_at".into(), options(TemporalRole::Event)),
            ("valid_from".into(), options(TemporalRole::ValidFrom)),
            ("valid_to".into(), options(TemporalRole::ValidTo)),
        ]),
        created_at: Utc::now(),
    };
    profile.refresh_source_hash();
    profile
}
