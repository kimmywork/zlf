use std::fs;

use tempfile::tempdir;
use zlf_prolog::bulk_pack::{compile_fact_files, load_fact_pack, BulkCompileOptions};
use zlf_storage::{MutationKind, Storage};

#[test]
#[allow(clippy::too_many_lines)]
fn ground_facts_compile_and_bulk_load_with_indexes() {
    let temp = tempdir().unwrap();
    let facts = temp.path().join("facts.pl");
    write_fixture(&facts);
    let pack = temp.path().join("fixture.zlfpack");
    let manifest = compile_fact_files(&[facts], &pack, &BulkCompileOptions::default()).unwrap();
    assert_eq!(manifest.fact_counts["node"], 2);
    assert_eq!(manifest.fact_counts["taxonomy_parent"], 1);

    let storage = Storage::open(temp.path().join("db")).unwrap();
    let report = load_fact_pack(&storage, &pack, 3).unwrap();
    assert_eq!(report.records_written, manifest.record_count);
    assert!(report.batches_written > 1);
    assert_eq!(
        storage
            .get_outgoing_edges("tax_2", Some("taxonomy_parent"))
            .unwrap()[0]
            .target,
        "tax_1"
    );
    assert_eq!(
        storage
            .get_nodes_by_property("rank", &zlf_core::Value::String("species".into()))
            .unwrap()[0]
            .id,
        "tax_2"
    );
    assert_eq!(
        storage
            .get_entity_state(&zlf_core::EntityRef::Node("tax_2".into()))
            .unwrap()
            .unwrap()
            .source_version,
        1
    );
    let events = storage.mutation_events_after(0, 10).unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].kind,
        MutationKind::RebuildRequired { .. }
    ));
    assert!(load_fact_pack(&storage, &pack, 10).unwrap().already_loaded);
    assert_eq!(storage.mutation_events_after(0, 10).unwrap().len(), 1);
}

fn write_fixture(path: &std::path::Path) {
    fs::write(
        path,
        r#"
        % taxonomy fixture
        node(tax_1, [taxon], {rank: "root", score: 1.5}).
        node(tax_2, [taxon], {rank: "species", scientific_name: "Bacteria. test"}).
        taxonomy_parent(tax_2, tax_1).
        "#,
    )
    .unwrap();
}

#[test]
fn compilation_is_deterministic_for_nested_object_properties() {
    let temp = tempdir().unwrap();
    let facts = temp.path().join("facts.pl");
    write_fixture(&facts);
    let first = compile_fact_files(
        std::slice::from_ref(&facts),
        &temp.path().join("first"),
        &BulkCompileOptions::default(),
    )
    .unwrap();
    let second = compile_fact_files(
        &[facts],
        &temp.path().join("second"),
        &BulkCompileOptions::default(),
    )
    .unwrap();
    assert_eq!(first.records_checksum, second.records_checksum);
    assert_eq!(first.record_count, second.record_count);
}

#[test]
fn compiler_rejects_non_ground_and_incremental_property_facts() {
    let temp = tempdir().unwrap();
    let variable = temp.path().join("variable.pl");
    fs::write(&variable, "node(X).\n").unwrap();
    assert!(compile_fact_files(
        &[variable],
        &temp.path().join("variable-pack"),
        &BulkCompileOptions::default(),
    )
    .is_err());

    let property = temp.path().join("property.pl");
    fs::write(&property, "property(tax_1, rank, species).\n").unwrap();
    assert!(compile_fact_files(
        &[property],
        &temp.path().join("property-pack"),
        &BulkCompileOptions::default(),
    )
    .is_err());
}

#[test]
fn loader_rejects_corrupted_records_before_writing() {
    let temp = tempdir().unwrap();
    let facts = temp.path().join("facts.pl");
    fs::write(&facts, "node(tax_1, [taxon], {rank: root}).\n").unwrap();
    let pack = temp.path().join("pack");
    compile_fact_files(&[facts], &pack, &BulkCompileOptions::default()).unwrap();
    let records = pack.join("records.bin");
    let mut bytes = fs::read(&records).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    fs::write(records, bytes).unwrap();

    let storage = Storage::open(temp.path().join("db")).unwrap();
    assert!(load_fact_pack(&storage, &pack, 100).is_err());
    assert!(storage.get_all_nodes().unwrap().is_empty());
}
