use zlf_index::bge_m3_dense_v1;
use zlf_query::EmbeddingModelProfileStore;
use zlf_storage::Storage;

#[test]
fn model_profiles_are_validated_immutable_ordered_and_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    {
        let storage = Storage::open(&path).unwrap();
        let store = EmbeddingModelProfileStore::new(&storage);
        let profile = bge_m3_dense_v1();
        let first = store.put(&profile).unwrap();
        assert_eq!(store.put(&profile).unwrap(), first);
        let mut changed = profile.clone();
        changed.dimension = 3;
        assert!(store.put(&changed).is_err());
        assert_eq!(store.list().unwrap(), vec![profile]);
    }
    let storage = Storage::open_existing(&path).unwrap();
    assert!(EmbeddingModelProfileStore::new(&storage)
        .get("bge_m3_dense_v1", 1)
        .unwrap()
        .is_some());
}
