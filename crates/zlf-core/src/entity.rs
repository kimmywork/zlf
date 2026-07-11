use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Result, Value, ZlfError};

/// Stable reference to a canonical graph entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EntityRef {
    Node(String),
    Edge(String),
}

impl EntityRef {
    pub fn id(&self) -> &str {
        match self {
            Self::Node(id) | Self::Edge(id) => id,
        }
    }
}

/// Atomic property changes. A null value in `set` remains a value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyPatch {
    #[serde(default)]
    pub set: BTreeMap<String, Value>,
    #[serde(default)]
    pub remove: BTreeSet<String>,
}

impl PropertyPatch {
    pub fn validate(&self) -> Result<()> {
        if let Some(key) = self.set.keys().find(|key| self.remove.contains(*key)) {
            return Err(ZlfError::InvalidPropertyValue(key.clone()));
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty() && self.remove.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_refs_are_typed_and_ordered() {
        let node = EntityRef::Node("same:id".into());
        let edge = EntityRef::Edge("same:id".into());
        assert_ne!(node, edge);
        assert_eq!(node.id(), "same:id");
        assert_ne!(edge.cmp(&node), std::cmp::Ordering::Equal);
    }

    #[test]
    fn patch_rejects_conflicting_keys_but_accepts_null() {
        let mut patch = PropertyPatch::default();
        patch.set.insert("value".into(), Value::Null);
        assert!(patch.validate().is_ok());
        patch.remove.insert("value".into());
        assert!(matches!(
            patch.validate(),
            Err(ZlfError::InvalidPropertyValue(key)) if key == "value"
        ));
    }
}
