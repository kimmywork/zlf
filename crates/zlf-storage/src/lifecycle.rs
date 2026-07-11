use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use zlf_core::EntityRef;

pub const MUTATION_EVENT_SCHEMA_VERSION: u32 = 1;
pub type MutationSequence = u64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityState {
    pub entity: EntityRef,
    pub source_version: MutationSequence,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationKind {
    Upsert {
        changed_fields: BTreeSet<String>,
    },
    Delete,
    RebuildRequired {
        bulk_id: String,
    },
    ConfigurationChanged {
        namespace: String,
        artifact_ref: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationEvent {
    pub schema_version: u32,
    pub sequence: MutationSequence,
    pub entity: Option<EntityRef>,
    pub source_version: MutationSequence,
    pub kind: MutationKind,
    pub occurred_at: DateTime<Utc>,
}

impl MutationEvent {
    pub fn upsert(
        sequence: MutationSequence,
        entity: EntityRef,
        changed_fields: BTreeSet<String>,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schema_version: MUTATION_EVENT_SCHEMA_VERSION,
            sequence,
            entity: Some(entity),
            source_version: sequence,
            kind: MutationKind::Upsert { changed_fields },
            occurred_at,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationReceipt {
    pub sequence: Option<MutationSequence>,
    pub entity_versions: Vec<(EntityRef, MutationSequence)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_round_trip_preserves_typed_entity_and_version() {
        let event = MutationEvent::upsert(
            42,
            EntityRef::Edge("edge:with:separators".into()),
            BTreeSet::from(["title".into()]),
            Utc::now(),
        );
        let bytes = bincode::serialize(&event).unwrap();
        let decoded: MutationEvent = bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded, event);
        assert_eq!(decoded.source_version, 42);
    }
}
