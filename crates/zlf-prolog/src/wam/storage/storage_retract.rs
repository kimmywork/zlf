use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_key::{term_to_delete_pattern, DeletePattern, FactKey};
use super::storage_writer::StorageFactWriter;
use zlf_storage::Storage;

impl<'a> StorageFactWriter<'a> {
    /// Delete a fact from storage.  Returns the canonical FactKey if
    /// the term was recognized and deleted, or an error if the term
    /// is not a supported fact form.
    pub fn retract_fact(&self, fact: &Term) -> WamResult<Option<FactKey>> {
        let pattern = term_to_delete_pattern(fact)
            .ok_or_else(|| WamError::Provider("unsupported retract fact form".to_string()))?;
        self.retract_pattern(&pattern)
    }

    /// Delete facts matching a DeletePattern.  Returns the canonical
    /// FactKey if the pattern was resolved and deletion succeeded.
    #[allow(clippy::too_many_lines)]
    fn retract_pattern(&self, pattern: &DeletePattern) -> WamResult<Option<FactKey>> {
        match pattern {
            DeletePattern::Node { id } => {
                let existed = self
                    .storage
                    .delete_node_cascade(id)
                    .map_err(provider_error)?;
                if existed {
                    Ok(Some(FactKey::Node { id: id.clone() }))
                } else {
                    Ok(None)
                }
            }
            DeletePattern::Label { node, label } => {
                let existed = self
                    .storage
                    .remove_node_label(node, label)
                    .map_err(provider_error)?;
                if existed {
                    Ok(Some(FactKey::Label {
                        node: node.clone(),
                        label: label.clone(),
                    }))
                } else {
                    Ok(None)
                }
            }
            DeletePattern::Property { entity, key } => {
                let existed = self
                    .storage
                    .remove_entity_property(entity, key)
                    .map(|receipt| receipt.sequence.is_some())
                    .map_err(provider_error)?;
                if existed {
                    Ok(Some(FactKey::Property {
                        entity: entity.clone(),
                        key: key.clone(),
                    }))
                } else {
                    Ok(None)
                }
            }
            DeletePattern::Edge {
                source,
                edge_type,
                target,
            } => {
                let existed = self
                    .storage
                    .delete_edge_by_triple(source, edge_type, target)
                    .map_err(provider_error)?;
                if existed {
                    Ok(Some(FactKey::Edge {
                        source: source.clone(),
                        edge_type: edge_type.clone(),
                        target: target.clone(),
                    }))
                } else {
                    Ok(None)
                }
            }
            DeletePattern::EdgeTypeFromSource { edge_type, source } => {
                let edges = self
                    .storage
                    .get_edges_by_type(edge_type)
                    .map_err(provider_error)?;
                let mut deleted = false;
                for edge in edges {
                    if edge.source == *source {
                        self.storage
                            .delete_edge_by_triple(&edge.source, &edge.edge_type, &edge.target)
                            .map_err(provider_error)?;
                        deleted = true;
                    }
                }
                if deleted {
                    Ok(Some(FactKey::Edge {
                        source: source.clone(),
                        edge_type: edge_type.clone(),
                        target: String::new(),
                    }))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

pub struct StorageRetractWriter<'a> {
    pub storage: &'a Storage,
}

impl<'a> StorageRetractWriter<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    /// Delete a fact from storage, using StorageFactWriter internally.
    pub fn retract_fact(&self, fact: &Term) -> WamResult<Option<FactKey>> {
        let writer = StorageFactWriter::new(self.storage);
        writer.retract_fact(fact)
    }
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
