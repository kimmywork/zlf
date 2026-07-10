use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::storage_provider::{value_term, StorageFactProvider};
use zlf_core::{Edge, Node};

impl StorageFactProvider<'_> {
    pub(crate) fn facts_for_bound_goal(&self, goal: &Term) -> WamResult<Option<Vec<Term>>> {
        let Term::Compound { name, args } = goal else {
            return Ok(None);
        };
        if let Some(facts) = self.bound_property_goal(name, args)? {
            return Ok(Some(facts));
        }
        match (name.as_str(), args.as_slice()) {
            ("node", [id]) if atom(id).is_some() => Ok(Some(self.bound_node(atom(id).unwrap())?)),
            ("label", [id, label]) if atom(id).is_some() || atom(label).is_some() => {
                Ok(Some(self.bound_labels(atom(id), atom(label))?))
            }
            ("edge", [source, edge_type, target]) if atom(edge_type).is_some() => Ok(Some(
                self.bound_edges(atom(source), atom(edge_type).unwrap(), atom(target), true)?,
            )),
            (name, [source, target])
                if !name.starts_with("prop_")
                    && (atom(source).is_some() || atom(target).is_some()) =>
            {
                Ok(Some(self.bound_edges(
                    atom(source),
                    name,
                    atom(target),
                    false,
                )?))
            }
            (name, [id]) if atom(id).is_some() => {
                Ok(Some(self.bound_label_shortcut(name, atom(id).unwrap())?))
            }
            _ => Ok(None),
        }
    }

    fn bound_property_goal(&self, name: &str, args: &[Term]) -> WamResult<Option<Vec<Term>>> {
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

    fn bound_node(&self, id: &str) -> WamResult<Vec<Term>> {
        self.storage
            .get_node(id)
            .map(|node| {
                node.into_iter()
                    .map(|node| compound("node", vec![Term::Atom(node.id)]))
                    .collect()
            })
            .map_err(provider_error)
    }

    fn bound_labels(&self, id: Option<&str>, label: Option<&str>) -> WamResult<Vec<Term>> {
        let nodes = match (id, label) {
            (Some(id), _) => self
                .storage
                .get_node(id)
                .map(|node| node.into_iter().collect()),
            (None, Some(label)) => self.storage.get_nodes_by_label(label),
            _ => unreachable!(),
        }
        .map_err(provider_error)?;
        Ok(nodes
            .into_iter()
            .flat_map(|node| label_facts(node, label))
            .collect())
    }

    fn bound_edges(
        &self,
        source: Option<&str>,
        edge_type: &str,
        target: Option<&str>,
        canonical: bool,
    ) -> WamResult<Vec<Term>> {
        let edges = match (source, target) {
            (Some(source), _) => self.storage.get_outgoing_edges(source, Some(edge_type)),
            (None, Some(target)) => self.storage.get_incoming_edges(target, Some(edge_type)),
            (None, None) => self.storage.get_edges_by_type(edge_type),
        }
        .map_err(provider_error)?;
        Ok(edges
            .into_iter()
            .filter(|edge| target.is_none_or(|target| edge.target == target))
            .map(|edge| edge_fact(edge, canonical))
            .collect())
    }

    fn bound_label_shortcut(&self, label: &str, id: &str) -> WamResult<Vec<Term>> {
        self.storage
            .get_node(id)
            .map(|node| {
                node.filter(|node| node.labels.iter().any(|item| item == label))
                    .into_iter()
                    .map(|node| compound(label, vec![Term::Atom(node.id)]))
                    .collect()
            })
            .map_err(provider_error)
    }

    fn bound_property_value(
        &self,
        predicate: &str,
        key: &str,
        value: zlf_core::Value,
    ) -> WamResult<Vec<Term>> {
        self.storage
            .get_nodes_by_property(key, &value)
            .map(|nodes| {
                nodes
                    .into_iter()
                    .map(|node| {
                        compound(
                            predicate,
                            vec![Term::Atom(node.id), value_term(value.clone())],
                        )
                    })
                    .collect()
            })
            .map_err(provider_error)
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

fn label_facts(node: Node, filter: Option<&str>) -> Vec<Term> {
    node.labels
        .into_iter()
        .filter(|label| filter.is_none_or(|filter| label == filter))
        .map(|label| {
            compound(
                "label",
                vec![Term::Atom(node.id.clone()), Term::Atom(label)],
            )
        })
        .collect()
}

fn edge_fact(edge: Edge, canonical: bool) -> Term {
    if canonical {
        compound(
            "edge",
            vec![
                Term::Atom(edge.source),
                Term::Atom(edge.edge_type),
                Term::Atom(edge.target),
            ],
        )
    } else {
        compound(
            edge.edge_type,
            vec![Term::Atom(edge.source), Term::Atom(edge.target)],
        )
    }
}

fn compound(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
