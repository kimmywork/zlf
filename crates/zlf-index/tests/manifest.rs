use zlf_core::EntityRef;
use zlf_index::{
    content_fingerprint, reconcile_manifest, DocumentManifest, IndexDocument, IndexDocumentId,
    SourceRange, INDEX_DOCUMENT_SCHEMA_VERSION,
};

#[test]
fn reconciliation_upserts_changes_and_deletes_removed_chunks() {
    let old = manifest(1, &[doc("0", "old"), doc("1", "removed")]);
    let desired = manifest(2, &[doc("0", "new"), doc("2", "added")]);
    let changes = reconcile_manifest(Some(&old), &desired).unwrap();
    assert_eq!(
        changes
            .upserts
            .iter()
            .map(|document| document.id.chunk_id.as_str())
            .collect::<Vec<_>>(),
        vec!["0", "2"]
    );
    assert_eq!(changes.deletes[0].chunk_id, "1");
}

#[test]
fn identical_manifest_is_idempotent_and_stale_version_fails() {
    let current = manifest(2, &[doc("0", "same")]);
    assert_eq!(
        reconcile_manifest(Some(&current), &current).unwrap(),
        Default::default()
    );
    assert!(reconcile_manifest(Some(&current), &manifest(1, &[doc("0", "same")])).is_err());
}

fn manifest(source_version: u64, documents: &[IndexDocument]) -> DocumentManifest {
    DocumentManifest {
        entity: EntityRef::Node("doc".into()),
        profile_name: "knowledge".into(),
        profile_version: 1,
        source_version,
        documents: documents.to_vec(),
    }
}

fn doc(chunk_id: &str, content: &str) -> IndexDocument {
    IndexDocument {
        schema_version: INDEX_DOCUMENT_SCHEMA_VERSION,
        id: IndexDocumentId::new(EntityRef::Node("doc".into()), "body", chunk_id),
        source_version: 1,
        content_fingerprint: content_fingerprint(content),
        source_range: Some(SourceRange {
            start: 0,
            end: content.len() as u64,
        }),
        chunk_ordinal: chunk_id.parse().unwrap(),
        chunk_profile: "whole_field_v1".into(),
        language: None,
        content: content.into(),
    }
}
