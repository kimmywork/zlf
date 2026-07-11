use std::collections::BTreeMap;

use chrono::Utc;
use zlf_index::{
    Bm25FieldOptions, EntityMatcher, FieldIndexOptions, IndexProfileArtifact,
    INDEX_PROFILE_SCHEMA_VERSION,
};
use zlf_query::ZlfDatabase;

#[test]
fn immutable_profiles_activate_atomically_and_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let profile = profile("knowledge", 1, 1.0);
    let creation_sequence;
    {
        let db = ZlfDatabase::open(temp.path()).unwrap();
        creation_sequence = db.put_index_profile(&profile).unwrap();
        assert_eq!(db.put_index_profile(&profile).unwrap(), creation_sequence);
        let activation = db.activate_index_profile("knowledge", 1).unwrap();
        assert!(activation > creation_sequence);
        assert_eq!(
            db.active_index_profile("knowledge").unwrap(),
            Some(profile.clone())
        );
    }
    let reopened = ZlfDatabase::open_existing(temp.path()).unwrap();
    assert_eq!(
        reopened.active_index_profile("knowledge").unwrap(),
        Some(profile)
    );
    assert_eq!(reopened.index_profiles().unwrap().len(), 1);
}

#[test]
fn profile_name_and_version_are_immutable() {
    let temp = tempfile::tempdir().unwrap();
    let db = ZlfDatabase::open(temp.path()).unwrap();
    db.put_index_profile(&profile("knowledge", 1, 1.0)).unwrap();
    assert!(db.put_index_profile(&profile("knowledge", 1, 2.0)).is_err());
    assert!(db.activate_index_profile("missing", 1).is_err());
}

fn profile(name: &str, version: u32, weight: f32) -> IndexProfileArtifact {
    IndexProfileArtifact {
        schema_version: INDEX_PROFILE_SCHEMA_VERSION,
        name: name.into(),
        version,
        source_hash: format!("hash-{weight}"),
        matcher: EntityMatcher::NodeLabels {
            labels: vec!["document".into()],
        },
        fields: BTreeMap::from([(
            "body".into(),
            FieldIndexOptions {
                bm25: Some(Bm25FieldOptions {
                    analyzer_id: "unicode_jieba_v1".into(),
                    analyzer_version: 1,
                    weight,
                    k1: 1.2,
                    b: 0.75,
                }),
                vector: None,
                temporal: None,
            },
        )]),
        created_at: Utc::now(),
    }
}
