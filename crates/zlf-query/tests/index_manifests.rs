use zlf_core::EntityRef;
use zlf_index::{
    content_fingerprint, DocumentManifest, IndexDocument, IndexDocumentId,
    INDEX_DOCUMENT_SCHEMA_VERSION,
};
use zlf_query::IndexManifestStore;
use zlf_storage::Storage;

#[test]
fn manifest_store_reconciles_and_reopens() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("storage");
    {
        let storage = Storage::open(&path).unwrap();
        let store = IndexManifestStore::new(&storage);
        let first = manifest(1, &[document("0", "old"), document("1", "remove")]);
        assert_eq!(store.reconcile_and_save(&first).unwrap().upserts.len(), 2);
        let second = manifest(2, &[document("0", "new")]);
        let changes = store.reconcile_and_save(&second).unwrap();
        assert_eq!(changes.upserts.len(), 1);
        assert_eq!(changes.deletes.len(), 1);
    }
    let storage = Storage::open_existing(&path).unwrap();
    let loaded = IndexManifestStore::new(&storage)
        .get(&EntityRef::Node("doc".into()), "knowledge", 1)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.source_version, 2);
    assert_eq!(loaded.documents[0].content, "new");
}

fn manifest(source_version: u64, documents: &[IndexDocument]) -> DocumentManifest {
    let mut documents = documents.to_vec();
    for document in &mut documents {
        document.source_version = source_version;
    }
    DocumentManifest {
        entity: EntityRef::Node("doc".into()),
        profile_name: "knowledge".into(),
        profile_version: 1,
        source_version,
        documents,
    }
}

fn document(chunk: &str, content: &str) -> IndexDocument {
    IndexDocument {
        schema_version: INDEX_DOCUMENT_SCHEMA_VERSION,
        id: IndexDocumentId::new(EntityRef::Node("doc".into()), "body", chunk),
        source_version: 0,
        content_fingerprint: content_fingerprint(content),
        source_range: None,
        chunk_ordinal: chunk.parse().unwrap(),
        chunk_profile: "whole_field_v1".into(),
        content: content.into(),
    }
}
