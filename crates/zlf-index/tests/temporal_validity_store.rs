use zlf_core::EntityRef;
use zlf_index::{
    valid_at_oracle, valid_overlaps_oracle, GenerationId, IndexDocumentId, TemporalAccessPath,
    TemporalRecordId, ValidityRecord, ValidityStore, TEMPORAL_RECORD_SCHEMA_VERSION,
};

#[test]
#[allow(clippy::too_many_lines)]
fn validity_indexes_match_oracle_choose_endpoints_and_survive_updates_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let generation = GenerationId("g1".into());
    let mut records = vec![
        validity("early", "doc", 0, Some(10), &generation),
        validity("middle", "doc", 10, Some(30), &generation),
        validity("late", "other", 20, Some(40), &generation),
        validity("open", "doc", 5, None, &generation),
    ];
    {
        let store = ValidityStore::open(temp.path()).unwrap();
        for record in &records {
            store.put(record).unwrap();
        }
        store
            .put(&validity(
                "isolated",
                "doc",
                0,
                None,
                &GenerationId("g2".into()),
            ))
            .unwrap();

        let at = store.valid_at(&generation, 25, 10).unwrap();
        assert_eq!(at.records, valid_at_oracle(&records, 25));
        assert_eq!(at.access_path, TemporalAccessPath::ValidByEnd);
        assert!(at.candidates_scanned <= 3);

        let overlap = store.overlaps(&generation, 9, 21, 10).unwrap();
        assert_eq!(overlap.records, valid_overlaps_oracle(&records, 9, 21));
        assert!(overlap.candidates_scanned <= records.len() as u64);
        assert_eq!(
            ids(&store
                .for_document(&generation, &document("doc"), 10)
                .unwrap()
                .records),
            ["early", "open", "middle"]
        );
        assert!(store.overlaps(&generation, 10, 10, 10).is_err());
        assert!(store.valid_at(&generation, 10, 0).is_err());

        let old = records.remove(1);
        store.delete(&old).unwrap();
        let moved = validity("middle", "doc", 30, Some(50), &generation);
        store.put(&moved).unwrap();
        records.push(moved);
        store.delete(&records[0]).unwrap();
        records.remove(0);
    }
    let reopened = ValidityStore::open(temp.path()).unwrap();
    assert_eq!(
        reopened.overlaps(&generation, 25, 35, 10).unwrap().records,
        valid_overlaps_oracle(&records, 25, 35)
    );
    assert_eq!(
        ids(&reopened.valid_at(&generation, 10, 1).unwrap().records),
        ["open"]
    );
}

#[test]
fn validity_boundaries_are_half_open_and_open_end_contains_i64_max() {
    let temp = tempfile::tempdir().unwrap();
    let generation = GenerationId("g1".into());
    let store = ValidityStore::open(temp.path()).unwrap();
    store
        .put(&validity("closed", "doc", 10, Some(20), &generation))
        .unwrap();
    store
        .put(&validity("adjacent", "doc", 20, Some(30), &generation))
        .unwrap();
    store
        .put(&validity("open", "doc", i64::MAX, None, &generation))
        .unwrap();
    assert_eq!(
        ids(&store.valid_at(&generation, 19, 10).unwrap().records),
        ["closed"]
    );
    assert_eq!(
        ids(&store.valid_at(&generation, 20, 10).unwrap().records),
        ["adjacent"]
    );
    assert_eq!(
        ids(&store.valid_at(&generation, i64::MAX, 10).unwrap().records),
        ["open"]
    );
}

fn validity(
    id: &str,
    node: &str,
    from: i64,
    to: Option<i64>,
    generation: &GenerationId,
) -> ValidityRecord {
    ValidityRecord {
        schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
        generation: generation.clone(),
        id: TemporalRecordId(id.into()),
        document_id: document(node),
        source_version: 1,
        valid_from_micros: from,
        valid_to_micros: to,
    }
}

fn document(node: &str) -> IndexDocumentId {
    IndexDocumentId::new(EntityRef::Node(node.into()), "validity", "0")
}

fn ids(records: &[ValidityRecord]) -> Vec<String> {
    records.iter().map(|record| record.id.0.clone()).collect()
}
