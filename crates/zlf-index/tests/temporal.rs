use chrono::{DateTime, NaiveDate, Utc};
use tempfile::TempDir;
use zlf_index::{TemporalEntry, TemporalIndex};

fn create_test_index() -> (TemporalIndex, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let index = TemporalIndex::open(temp_dir.path().join("temporal")).unwrap();
    (index, temp_dir)
}

fn entry(node_id: &str, timestamp: &str) -> TemporalEntry {
    TemporalEntry {
        node_id: node_id.to_string(),
        valid_from: DateTime::parse_from_rfc3339(timestamp)
            .unwrap()
            .with_timezone(&Utc),
        valid_to: None,
    }
}

#[test]
fn add_and_get_entry() {
    let (index, _temp) = create_test_index();
    let entry = TemporalEntry {
        node_id: "alice".to_string(),
        valid_from: Utc::now(),
        valid_to: None,
    };

    index.add_entry(entry.clone()).unwrap();

    let entries = index
        .get_entries_for_date(entry.valid_from.date_naive())
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].node_id, "alice");
}

#[test]
fn get_entries_in_range() {
    let (index, _temp) = create_test_index();
    index
        .add_entry(entry("alice", "2026-01-01T00:00:00Z"))
        .unwrap();
    index
        .add_entry(entry("bob", "2026-06-15T00:00:00Z"))
        .unwrap();

    let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2026, 6, 30).unwrap();

    assert_eq!(index.get_entries_in_range(start, end).unwrap().len(), 2);
}

#[test]
fn remove_entry() {
    let (index, _temp) = create_test_index();
    let entry = TemporalEntry {
        node_id: "alice".to_string(),
        valid_from: Utc::now(),
        valid_to: None,
    };

    index.add_entry(entry.clone()).unwrap();
    index.remove_entry("alice", entry.valid_from).unwrap();

    let entries = index
        .get_entries_for_date(entry.valid_from.date_naive())
        .unwrap();
    assert!(entries.is_empty());
}

#[test]
fn time_range() {
    let (index, _temp) = create_test_index();
    index
        .add_entry(entry("alice", "2026-01-01T00:00:00Z"))
        .unwrap();
    index
        .add_entry(entry("bob", "2026-06-15T00:00:00Z"))
        .unwrap();

    let start = entry("start", "2026-01-01T00:00:00Z").valid_from;
    let end = entry("end", "2026-06-30T00:00:00Z").valid_from;

    assert_eq!(index.time_range(start, end).unwrap().len(), 2);
}

#[test]
fn before_after_between() {
    let (index, _temp) = create_test_index();
    index
        .add_entry(entry("alice", "2026-01-01T00:00:00Z"))
        .unwrap();
    index
        .add_entry(entry("bob", "2026-06-15T00:00:00Z"))
        .unwrap();
    index
        .add_entry(entry("charlie", "2026-12-01T00:00:00Z"))
        .unwrap();
    let march = entry("march", "2026-03-01T00:00:00Z").valid_from;
    let end = entry("end", "2026-06-30T00:00:00Z").valid_from;

    let before = index.before(march).unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].node_id, "alice");

    let after = index.after(march).unwrap();
    assert_eq!(after.len(), 2);
    assert!(after.iter().any(|entry| entry.node_id == "bob"));
    assert!(after.iter().any(|entry| entry.node_id == "charlie"));

    assert_eq!(
        index
            .between(entry("start", "2026-01-01T00:00:00Z").valid_from, end)
            .unwrap()
            .len(),
        2
    );
}
