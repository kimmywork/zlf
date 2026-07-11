use tempfile::TempDir;
use zlf_storage::{BulkSessionState, MutationKind, Storage, StorageRecord, StorageRecordPlan};

#[test]
#[allow(clippy::too_many_lines)]
fn bulk_session_checkpoints_resume_and_publish_one_rebuild_event() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("db");
    {
        let storage = Storage::open(&path).unwrap();
        let session = storage.begin_bulk_session("import-1").unwrap();
        assert_eq!(session.state, BulkSessionState::Started);
        storage
            .write_bulk_plan(
                "import-1",
                &StorageRecordPlan {
                    records: vec![StorageRecord {
                        key: b"custom:first".to_vec(),
                        value: b"one".to_vec(),
                    }],
                },
                1,
            )
            .unwrap();
    }
    let storage = Storage::open_existing(&path).unwrap();
    let sessions = storage.list_bulk_sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].state, BulkSessionState::Writing);
    let resumed = storage.begin_bulk_session("import-1").unwrap();
    assert_eq!(resumed.state, BulkSessionState::Writing);
    assert_eq!(resumed.checkpoint, 1);
    storage
        .write_bulk_plan(
            "import-1",
            &StorageRecordPlan {
                records: vec![StorageRecord {
                    key: b"custom:second".to_vec(),
                    value: b"two".to_vec(),
                }],
            },
            2,
        )
        .unwrap();
    let sequence = storage.complete_bulk_session("import-1").unwrap();
    assert_eq!(storage.complete_bulk_session("import-1").unwrap(), sequence);
    let events = storage.mutation_events_after(0, 10).unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].kind,
        MutationKind::RebuildRequired { ref bulk_id } if bulk_id == "import-1"
    ));
}

#[test]
fn raw_api_rejects_canonical_graph_keys() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    assert!(storage.put_raw("node:bypass", b"bad").is_err());
    assert!(storage.delete_raw("edge:bypass").is_err());
    assert!(storage.put_raw("rule:allowed", b"ok").is_ok());
}

#[test]
fn bulk_checkpoint_cannot_move_backwards() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::open(temp.path().join("db")).unwrap();
    storage.begin_bulk_session("import-2").unwrap();
    let empty = StorageRecordPlan::default();
    storage.write_bulk_plan("import-2", &empty, 10).unwrap();
    assert!(storage.write_bulk_plan("import-2", &empty, 9).is_err());
}
