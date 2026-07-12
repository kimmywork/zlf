use serde::{Deserialize, Serialize};
use zlf_core::EntityRef;

pub const INDEX_DOCUMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IndexDocumentId {
    pub entity: EntityRef,
    pub field: String,
    pub chunk_id: String,
}

impl IndexDocumentId {
    pub fn new(entity: EntityRef, field: impl Into<String>, chunk_id: impl Into<String>) -> Self {
        Self {
            entity,
            field: field.into(),
            chunk_id: chunk_id.into(),
        }
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(match self.entity {
            EntityRef::Node(_) => 0,
            EntityRef::Edge(_) => 1,
        });
        push_part(&mut bytes, self.entity.id().as_bytes());
        push_part(&mut bytes, self.field.as_bytes());
        push_part(&mut bytes, self.chunk_id.as_bytes());
        bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentFingerprint(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: u64,
    pub end: u64,
}

impl SourceRange {
    pub fn is_valid(self) -> bool {
        self.start <= self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDocument {
    pub schema_version: u32,
    pub id: IndexDocumentId,
    pub source_version: u64,
    pub content_fingerprint: ContentFingerprint,
    pub source_range: Option<SourceRange>,
    pub chunk_ordinal: u32,
    pub chunk_profile: String,
    pub language: Option<String>,
    pub content: String,
}

fn push_part(target: &mut Vec<u8>, part: &[u8]) {
    target.extend_from_slice(&(part.len() as u32).to_be_bytes());
    target.extend_from_slice(part);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_key_does_not_collide_on_separators() {
        let left = IndexDocumentId::new(EntityRef::Node("a:b".into()), "c", "d");
        let right = IndexDocumentId::new(EntityRef::Node("a".into()), "b:c", "d");
        assert_ne!(left.canonical_bytes(), right.canonical_bytes());
    }

    #[test]
    fn entity_kind_is_part_of_key() {
        let node = IndexDocumentId::new(EntityRef::Node("x".into()), "body", "0");
        let edge = IndexDocumentId::new(EntityRef::Edge("x".into()), "body", "0");
        assert_ne!(node.canonical_bytes(), edge.canonical_bytes());
    }
}
