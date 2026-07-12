use std::collections::{BTreeMap, HashMap};

use zlf_core::{Node, Value};
use zlf_index::{
    BM25Index, Bm25FieldOptions, EntityMatcher, FieldIndexOptions, IndexProfileArtifact,
    INDEX_PROFILE_SCHEMA_VERSION,
};
use zlf_query::{
    Bm25IndexTarget, CoordinatorConfig, IndexCoordinator, IndexProfileStore, ZlfDatabase,
};
use zlf_storage::Storage;

#[test]
fn generation_rebuild_activates_and_reopens_the_physical_tantivy_index() {
    let temp = tempfile::tempdir().unwrap();
    let active;
    {
        let db = ZlfDatabase::open(temp.path()).unwrap();
        db.put_index_profile(&profile()).unwrap();
        db.activate_index_profile("knowledge", 1).unwrap();
        db.add_node(Node::with_id(
            "doc".into(),
            vec!["document".into()],
            HashMap::from([("body".into(), Value::String("durable text".into()))]),
        ))
        .unwrap();
        let previous = db.index_status("bm25").unwrap().active_generation.unwrap();
        active = db.rebuild_bm25_generation().unwrap();
        assert_ne!(active, previous);
        assert_eq!(db.search("durable").unwrap()[0].0, "doc");
    }
    let reopened = ZlfDatabase::open_existing(temp.path()).unwrap();
    assert_eq!(
        reopened.index_status("bm25").unwrap().active_generation,
        Some(active)
    );
    assert_eq!(reopened.search("durable").unwrap()[0].0, "doc");
}

#[test]
fn database_facade_routes_writes_through_bm25_lifecycle() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
    db.put_index_profile(&profile()).unwrap();
    db.activate_index_profile("knowledge", 1).unwrap();
    db.add_node(Node::with_id(
        "doc".into(),
        vec!["document".into()],
        HashMap::from([("body".into(), Value::String("old content".into()))]),
    ))
    .unwrap();
    assert_eq!(db.search("old").unwrap()[0].0, "doc");
    db.set_node_property("doc", "body", Value::String("new content".into()))
        .unwrap();
    assert!(db.search("old").unwrap().is_empty());
    assert_eq!(db.search("new").unwrap()[0].0, "doc");
    let mut second = profile();
    second.version = 2;
    let options = second.fields.remove("body").unwrap();
    second.fields.insert("title".into(), options);
    second.refresh_source_hash();
    db.put_index_profile(&second).unwrap();
    db.activate_index_profile("knowledge", 2).unwrap();
    assert!(db.search("new").unwrap().is_empty());
    db.query_prolog("? retract(node(doc)).").unwrap();
    assert!(db.search("new").unwrap().is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn profile_activation_update_and_delete_converge_tantivy_documents() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path().join("storage")).unwrap();
    let index = BM25Index::open(temp.path().join("bm25")).unwrap();
    storage
        .create_node(Node::with_id(
            "doc".into(),
            vec!["document".into()],
            HashMap::from([("body".into(), Value::String("old content".into()))]),
        ))
        .unwrap();
    let coordinator = IndexCoordinator::new(&storage, CoordinatorConfig::default());
    coordinator.register_target("bm25").unwrap();
    coordinator.enqueue_available("bm25").unwrap();
    let target = Bm25IndexTarget::new(&index, "bm25");
    while coordinator.process_next("bm25", &target).unwrap() {}
    assert!(index.search("old").unwrap().is_empty());

    let profiles = IndexProfileStore::new(&storage);
    profiles.put(&profile()).unwrap();
    profiles.activate("knowledge", 1).unwrap();
    coordinator.enqueue_available("bm25").unwrap();
    while coordinator.process_next("bm25", &target).unwrap() {}
    assert_eq!(index.search("old").unwrap()[0].0, "doc");

    storage
        .update_node(
            "doc",
            HashMap::from([("body".into(), Value::String("new content".into()))]),
        )
        .unwrap();
    coordinator.enqueue_available("bm25").unwrap();
    coordinator.process_next("bm25", &target).unwrap();
    assert!(index.search("old").unwrap().is_empty());
    assert_eq!(index.search("new").unwrap()[0].0, "doc");

    storage.delete_node("doc").unwrap();
    coordinator.enqueue_available("bm25").unwrap();
    coordinator.process_next("bm25", &target).unwrap();
    assert!(index.search("new").unwrap().is_empty());
}

fn profile() -> IndexProfileArtifact {
    let mut profile = IndexProfileArtifact {
        schema_version: INDEX_PROFILE_SCHEMA_VERSION,
        name: "knowledge".into(),
        version: 1,
        source_hash: String::new(),
        matcher: EntityMatcher::NodeLabels {
            labels: vec!["document".into()],
        },
        fields: BTreeMap::from([(
            "body".into(),
            FieldIndexOptions {
                bm25: Some(Bm25FieldOptions {
                    analyzer_id: "unicode_jieba_v1".into(),
                    analyzer_version: 1,
                    weight: 1.0,
                    k1: 1.2,
                    b: 0.75,
                }),
                vector: None,
                temporal: None,
            },
        )]),
        created_at: chrono::Utc::now(),
    };
    profile.refresh_source_hash();
    profile
}
