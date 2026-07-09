use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;
use super::view_helpers::{compound_term, edge_in_term, edge_out_term, properties_to_object};
use zlf_storage::Storage;

/// A FactProvider that materializes graph view predicates
/// (labels/2, properties/2, out_edges/2-3, in_edges/2-3, neighbors/2-3, node_view/2)
/// from RocksDB storage.
pub struct GraphViewProvider<'a> {
    storage: &'a Storage,
}

impl<'a> GraphViewProvider<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }
}

impl FactProvider for GraphViewProvider<'_> {
    fn facts_for(&self, key: &PredicateKey) -> WamResult<Vec<Term>> {
        match (key.name.as_str(), key.arity) {
            ("labels", 2) => self.labels_facts(),
            ("properties", 2) => self.properties_facts(),
            ("out_edges", 2) | ("out_edges", 3) => self.out_edges_facts(),
            ("in_edges", 2) | ("in_edges", 3) => self.in_edges_facts(),
            ("neighbors", 2) | ("neighbors", 3) => self.neighbors_facts(),
            ("node_view", 2) => self.node_view_facts(),
            _ => Ok(Vec::new()),
        }
    }

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        if let Term::Compound { name, args } = goal {
            match (name.as_str(), args.len()) {
                ("labels", 2) => self.labels_facts_for(args),
                ("properties", 2) => self.properties_facts_for(args),
                ("out_edges", 2) | ("out_edges", 3) => self.out_edges_facts_for(args),
                ("in_edges", 2) | ("in_edges", 3) => self.in_edges_facts_for(args),
                ("neighbors", 2) | ("neighbors", 3) => self.neighbors_facts_for(args),
                ("node_view", 2) => self.node_view_facts_for(args),
                _ => Ok(Vec::new()),
            }
        } else {
            Ok(Vec::new())
        }
    }
}

impl GraphViewProvider<'_> {
    fn labels_facts(&self) -> WamResult<Vec<Term>> {
        self.storage
            .get_all_nodes()
            .map_err(provider_error)?
            .into_iter()
            .map(|node| {
                let labels: Vec<Term> = node.labels.iter().map(|l| Term::Atom(l.clone())).collect();
                Ok(compound_term(
                    "labels",
                    vec![Term::Atom(node.id), Term::List(labels)],
                ))
            })
            .collect()
    }

    fn labels_facts_for(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let id = match args.first() {
            Some(Term::Atom(id) | Term::String(id)) => id,
            _ => return self.labels_facts(),
        };
        let Some(n) = self.storage.get_node(id).map_err(provider_error)? else {
            return Ok(Vec::new());
        };
        let labels: Vec<Term> = n.labels.iter().map(|l| Term::Atom(l.clone())).collect();
        Ok(vec![compound_term(
            "labels",
            vec![Term::Atom(id.clone()), Term::List(labels)],
        )])
    }

    fn properties_facts(&self) -> WamResult<Vec<Term>> {
        self.storage
            .get_all_nodes()
            .map_err(provider_error)?
            .into_iter()
            .map(|node| {
                let props = properties_to_object(&node.properties);
                Ok(compound_term(
                    "properties",
                    vec![Term::Atom(node.id), props],
                ))
            })
            .collect()
    }

    fn properties_facts_for(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let id = match args.first() {
            Some(Term::Atom(id) | Term::String(id)) => id,
            _ => return self.properties_facts(),
        };
        let Some(n) = self.storage.get_node(id).map_err(provider_error)? else {
            return Ok(Vec::new());
        };
        let props = properties_to_object(&n.properties);
        Ok(vec![compound_term(
            "properties",
            vec![Term::Atom(id.clone()), props],
        )])
    }

    fn out_edges_facts(&self) -> WamResult<Vec<Term>> {
        let edges = self.storage.get_all_edges().map_err(provider_error)?;
        let mut by_source: std::collections::HashMap<String, Vec<Term>> =
            std::collections::HashMap::new();
        for edge in &edges {
            let out = edge_out_term(edge);
            by_source.entry(edge.source.clone()).or_default().push(out);
        }
        Ok(by_source
            .into_iter()
            .map(|(source, list)| {
                compound_term("out_edges", vec![Term::Atom(source), Term::List(list)])
            })
            .collect())
    }

    fn out_edges_facts_for(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let id = match args.first() {
            Some(Term::Atom(id) | Term::String(id)) => id,
            _ => return self.out_edges_facts(),
        };
        let edges = self.storage.get_all_edges().map_err(provider_error)?;
        let out: Vec<Term> = edges
            .into_iter()
            .filter(|e| e.source == *id)
            .map(|e| edge_out_term(&e))
            .collect();
        if out.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![compound_term(
            "out_edges",
            vec![Term::Atom(id.to_string()), Term::List(out)],
        )])
    }

    fn in_edges_facts(&self) -> WamResult<Vec<Term>> {
        let edges = self.storage.get_all_edges().map_err(provider_error)?;
        let mut by_target: std::collections::HashMap<String, Vec<Term>> =
            std::collections::HashMap::new();
        for edge in &edges {
            let inc = edge_in_term(edge);
            by_target.entry(edge.target.clone()).or_default().push(inc);
        }
        Ok(by_target
            .into_iter()
            .map(|(target, list)| {
                compound_term("in_edges", vec![Term::Atom(target), Term::List(list)])
            })
            .collect())
    }

    fn in_edges_facts_for(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let id = match args.first() {
            Some(Term::Atom(id) | Term::String(id)) => id,
            _ => return self.in_edges_facts(),
        };
        let edges = self.storage.get_all_edges().map_err(provider_error)?;
        let inc: Vec<Term> = edges
            .into_iter()
            .filter(|e| e.target == *id)
            .map(|e| edge_in_term(&e))
            .collect();
        if inc.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![compound_term(
            "in_edges",
            vec![Term::Atom(id.to_string()), Term::List(inc)],
        )])
    }

    fn neighbors_facts(&self) -> WamResult<Vec<Term>> {
        let edges = self.storage.get_all_edges().map_err(provider_error)?;
        Ok(edges
            .into_iter()
            .map(|e| {
                compound_term(
                    "neighbors",
                    vec![Term::Atom(e.source), Term::Atom(e.target)],
                )
            })
            .collect())
    }

    fn neighbors_facts_for(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let id = match args.first() {
            Some(Term::Atom(id) | Term::String(id)) => id,
            _ => return self.neighbors_facts(),
        };
        let edges = self.storage.get_all_edges().map_err(provider_error)?;
        Ok(edges
            .into_iter()
            .filter(|e| e.source == *id)
            .map(|e| {
                compound_term(
                    "neighbors",
                    vec![Term::Atom(id.to_string()), Term::Atom(e.target)],
                )
            })
            .collect())
    }

    fn node_view_facts(&self) -> WamResult<Vec<Term>> {
        let edges = self.storage.get_all_edges().map_err(provider_error)?;
        self.storage
            .get_all_nodes()
            .map_err(provider_error)?
            .into_iter()
            .map(|node| Ok(self.build_node_view(&node, &edges)))
            .collect()
    }

    fn node_view_facts_for(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let id = match args.first() {
            Some(Term::Atom(id) | Term::String(id)) => id,
            _ => return self.node_view_facts(),
        };
        let Some(n) = self.storage.get_node(id).map_err(provider_error)? else {
            return Ok(Vec::new());
        };
        let edges = self.storage.get_all_edges().map_err(provider_error)?;
        Ok(vec![self.build_node_view(&n, &edges)])
    }

    fn build_node_view(&self, node: &zlf_core::Node, all_edges: &[zlf_core::Edge]) -> Term {
        let out_edges: Vec<Term> = all_edges
            .iter()
            .filter(|e| e.source == node.id)
            .map(edge_out_term)
            .collect();
        let in_edges: Vec<Term> = all_edges
            .iter()
            .filter(|e| e.target == node.id)
            .map(edge_in_term)
            .collect();
        let labels: Vec<Term> = node.labels.iter().map(|l| Term::Atom(l.clone())).collect();
        let props = properties_to_object(&node.properties);
        compound_term(
            "node_view",
            vec![
                Term::Atom(node.id.clone()),
                Term::Object(vec![
                    ("labels".to_string(), Term::List(labels)),
                    ("properties".to_string(), props),
                    ("out_edges".to_string(), Term::List(out_edges)),
                    ("in_edges".to_string(), Term::List(in_edges)),
                ]),
            ],
        )
    }
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
