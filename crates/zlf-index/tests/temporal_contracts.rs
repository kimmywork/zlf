use zlf_core::EntityRef;
use zlf_index::{
    decode_ordered_micros, encode_ordered_micros, event_range_oracle, parse_utc_micros,
    utc_day_range, valid_at_oracle, valid_overlaps_oracle, validate_half_open_range, EventRecord,
    GenerationId, IndexDocumentId, TemporalRecordId, ValidityRecord,
    TEMPORAL_RECORD_SCHEMA_VERSION,
};

#[test]
fn signed_microsecond_encoding_preserves_full_chronological_order() {
    let values = [i64::MIN, -1_000_000, -1, 0, 1, 1_000_000, i64::MAX];
    let encoded = values.map(encode_ordered_micros);
    assert!(encoded.windows(2).all(|pair| pair[0] < pair[1]));
    for (value, bytes) in values.into_iter().zip(encoded) {
        assert_eq!(decode_ordered_micros(bytes), value);
    }
}

#[test]
fn parser_converts_offsets_dates_leap_days_and_rejects_ambiguous_input() {
    assert_eq!(
        parse_utc_micros("2026-03-08T01:30:00-05:00").unwrap(),
        parse_utc_micros("2026-03-08T06:30:00Z").unwrap()
    );
    assert_eq!(
        parse_utc_micros("2026-11-01T01:30:00-04:00").unwrap(),
        parse_utc_micros("2026-11-01T05:30:00Z").unwrap()
    );
    let (start, end) = utc_day_range("2024-02-29").unwrap();
    assert_eq!(end - start, 86_400_000_000);
    assert_eq!(parse_utc_micros("2024-02-29").unwrap(), start);
    assert!(parse_utc_micros("2026-01-01 12:00:00").is_err());
    assert!(utc_day_range("2026-02-30").is_err());
}

#[test]
fn half_open_event_and_validity_oracles_cover_boundaries_duplicates_and_open_ends() {
    assert!(validate_half_open_range(1, 1).is_err());
    assert!(validate_half_open_range(2, 1).is_err());
    let events = vec![event("b", 10), event("a", 10), event("c", 20)];
    assert_eq!(
        event_range_oracle(&events, 10, 20)
            .into_iter()
            .map(|record| record.id.0)
            .collect::<Vec<_>>(),
        ["a", "b"]
    );

    let records = vec![
        validity("closed", 10, Some(20)),
        validity("adjacent", 20, Some(30)),
        validity("open", 5, None),
    ];
    assert_eq!(ids(valid_at_oracle(&records, 19)), ["open", "closed"]);
    assert_eq!(ids(valid_at_oracle(&records, 20)), ["open", "adjacent"]);
    assert_eq!(
        ids(valid_overlaps_oracle(&records, 20, 25)),
        ["open", "adjacent"]
    );
    assert!(validity("empty", 10, Some(10)).validate().is_err());
}

fn event(id: &str, at_micros: i64) -> EventRecord {
    EventRecord {
        schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
        generation: GenerationId("g1".into()),
        id: TemporalRecordId(id.into()),
        document_id: document_id(),
        source_version: 1,
        at_micros,
    }
}

fn validity(id: &str, from: i64, to: Option<i64>) -> ValidityRecord {
    ValidityRecord {
        schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
        generation: GenerationId("g1".into()),
        id: TemporalRecordId(id.into()),
        document_id: document_id(),
        source_version: 1,
        valid_from_micros: from,
        valid_to_micros: to,
    }
}

fn document_id() -> IndexDocumentId {
    IndexDocumentId::new(EntityRef::Node("node".into()), "time", "0")
}

fn ids(records: Vec<ValidityRecord>) -> Vec<String> {
    records.into_iter().map(|record| record.id.0).collect()
}
