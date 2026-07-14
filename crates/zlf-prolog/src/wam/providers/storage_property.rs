use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::storage_provider::{value_term, StorageFactProvider};

impl StorageFactProvider<'_> {
    pub(crate) fn bound_canonical_property_goal(
        &self,
        name: &str,
        args: &[Term],
    ) -> WamResult<Option<Vec<Term>>> {
        let [id, key, _value] = args else {
            return Ok(None);
        };
        if name != "property" || atom(id).is_none() || atom(key).is_none() {
            return Ok(None);
        }
        self.bound_canonical_property(atom(id).unwrap(), atom(key).unwrap())
            .map(Some)
    }

    pub(crate) fn bound_property_goal(
        &self,
        name: &str,
        args: &[Term],
    ) -> WamResult<Option<Vec<Term>>> {
        let [id, value] = args else {
            return Ok(None);
        };
        if !name.starts_with("prop_") || (atom(id).is_none() && storage_value(value).is_none()) {
            return Ok(None);
        }
        let key = name.trim_start_matches("prop_");
        match atom(id) {
            Some(id) => self.bound_property(key, id).map(Some),
            None => self
                .bound_property_value(name, key, storage_value(value).unwrap())
                .map(Some),
        }
    }

    fn bound_canonical_property(&self, id: &str, key: &str) -> WamResult<Vec<Term>> {
        let mut facts = Vec::new();
        if let Some(node) = self.storage.get_node(id).map_err(provider_error)? {
            if let Some(value) = node.properties.get(key).cloned() {
                facts.push(compound(
                    "property",
                    vec![
                        Term::Atom(node.id),
                        Term::Atom(key.into()),
                        value_term(value),
                    ],
                ));
            }
        }
        if let Some(edge) = self.storage.get_edge(id).map_err(provider_error)? {
            if let Some(value) = edge.properties.get(key).cloned() {
                facts.push(compound(
                    "property",
                    vec![
                        Term::Atom(edge.id),
                        Term::Atom(key.into()),
                        value_term(value),
                    ],
                ));
            }
        }
        Ok(facts)
    }

    fn bound_property_value(
        &self,
        predicate: &str,
        key: &str,
        value: zlf_core::Value,
    ) -> WamResult<Vec<Term>> {
        let mut facts = self
            .storage
            .get_nodes_by_property(key, &value)
            .map_err(provider_error)?
            .into_iter()
            .map(|node| {
                compound(
                    predicate,
                    vec![Term::Atom(node.id), value_term(value.clone())],
                )
            })
            .collect::<Vec<_>>();
        facts.extend(
            self.storage
                .get_all_edges()
                .map_err(provider_error)?
                .into_iter()
                .filter(|edge| edge.properties.get(key) == Some(&value))
                .map(|edge| {
                    compound(
                        predicate,
                        vec![Term::Atom(edge.id), value_term(value.clone())],
                    )
                }),
        );
        Ok(facts)
    }

    fn bound_property(&self, key: &str, id: &str) -> WamResult<Vec<Term>> {
        let mut facts = Vec::new();
        if let Some(node) = self.storage.get_node(id).map_err(provider_error)? {
            if let Some(value) = node.properties.get(key).cloned() {
                facts.push(compound(
                    format!("prop_{key}"),
                    vec![Term::Atom(node.id), value_term(value)],
                ));
            }
        }
        if let Some(edge) = self.storage.get_edge(id).map_err(provider_error)? {
            if let Some(value) = edge.properties.get(key).cloned() {
                facts.push(compound(
                    format!("prop_{key}"),
                    vec![Term::Atom(edge.id), value_term(value)],
                ));
            }
        }
        Ok(facts)
    }
}

fn storage_value(term: &Term) -> Option<zlf_core::Value> {
    match term {
        Term::Atom(value) | Term::String(value) => Some(zlf_core::Value::String(value.clone())),
        Term::Integer(value) => Some(zlf_core::Value::Number(*value as f64)),
        Term::Float(value) => Some(zlf_core::Value::Number(*value)),
        _ => None,
    }
}

fn atom(term: &Term) -> Option<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Some(value),
        _ => None,
    }
}

fn compound(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}

fn provider_error(error: zlf_core::ZlfError) -> WamError {
    WamError::Provider(error.to_string())
}
