use zlf_core::EntityRef;
use zlf_index::{
    parse_utc_micros, EventRecord, EventTimeStore, GenerationId, IndexDocumentId, IndexPageRequest,
    TemporalRecordId, TEMPORAL_RECORD_SCHEMA_VERSION,
};

#[test]
#[allow(clippy::too_many_lines)]
fn bounded_event_seeks_preserve_duplicates_boundaries_entities_updates_and_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let generation = GenerationId("g1".into());
    let at_10 = parse_utc_micros("2026-01-01T10:00:00Z").unwrap();
    let at_20 = parse_utc_micros("2026-01-01T20:00:00Z").unwrap();
    let at_next = parse_utc_micros("2026-01-02T00:00:00Z").unwrap();
    let first = event("b", "doc", at_10, &generation);
    let second = event("a", "doc", at_10, &generation);
    let boundary = event("c", "other", at_20, &generation);
    {
        let store = EventTimeStore::open(temp.path()).unwrap();
        store.put(&first).unwrap();
        store.put(&second).unwrap();
        store.put(&boundary).unwrap();
        store
            .put(&event(
                "other-generation",
                "doc",
                at_10,
                &GenerationId("g2".into()),
            ))
            .unwrap();

        let range = store.range(&generation, at_10, at_20, 10).unwrap();
        assert_eq!(ids(&range.records), ["a", "b"]);
        assert_eq!(range.candidates_scanned, 2);
        assert_eq!(
            ids(&store.after(&generation, at_10, 10).unwrap().records),
            ["c"]
        );
        assert_eq!(
            ids(&store.before(&generation, at_20, 10).unwrap().records),
            ["a", "b"]
        );
        assert_eq!(
            ids(&store
                .for_document(&generation, &document("doc"), 10)
                .unwrap()
                .records),
            ["a", "b"]
        );
        assert_eq!(
            ids(&store.day(&generation, "2026-01-01", 10).unwrap().records),
            ["a", "b", "c"]
        );
        let page = store
            .range_page(
                &generation,
                at_10,
                at_next,
                IndexPageRequest {
                    offset: 1,
                    page_size: 1,
                    candidate_limit: 3,
                },
            )
            .unwrap();
        assert_eq!(ids(&page.items), ["b"]);
        assert_eq!(page.next_offset, Some(2));
        assert_eq!(
            ids(&store
                .range_for_entity(
                    &generation,
                    &EntityRef::Node("other".into()),
                    at_10,
                    at_next,
                    10,
                )
                .unwrap()
                .records),
            ["c"]
        );
        assert!(store.range(&generation, at_20, at_20, 10).is_err());
        assert!(store.range(&generation, at_10, at_next, 0).is_err());

        let mut moved = second.clone();
        moved.at_micros = at_20;
        store
            .apply(&[moved], &[first.clone(), second.clone()])
            .unwrap();
    }
    let reopened = EventTimeStore::open(temp.path()).unwrap();
    assert_eq!(
        ids(&reopened
            .range(&generation, at_10, at_next, 10)
            .unwrap()
            .records),
        ["a", "c"]
    );
    assert_eq!(
        ids(&reopened.before(&generation, i64::MAX, 1).unwrap().records),
        ["c"]
    );
    assert!(reopened
        .after(&generation, i64::MAX, 10)
        .unwrap()
        .records
        .is_empty());
}

fn event(id: &str, node: &str, at_micros: i64, generation: &GenerationId) -> EventRecord {
    EventRecord {
        schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
        generation: generation.clone(),
        id: TemporalRecordId(id.into()),
        document_id: document(node),
        source_version: 1,
        at_micros,
    }
}

fn document(node: &str) -> IndexDocumentId {
    IndexDocumentId::new(EntityRef::Node(node.into()), "occurred_at", "0")
}

fn ids(records: &[EventRecord]) -> Vec<String> {
    records.iter().map(|record| record.id.0.clone()).collect()
}
