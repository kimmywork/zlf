use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;
use super::view_helpers::{compound_term, edge_in_term, edge_out_term, properties_to_object};
use zlf_core::{Edge, Node};
use zlf_storage::Storage;

/// Storage-backed graph view predicates.
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
        self.facts_for_goal(&unbound_goal(&key.name, key.arity))
    }

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        let Term::Compound { name, args } = goal else {
            return Ok(Vec::new());
        };
        match (name.as_str(), args.len()) {
            ("labels", 2) => self.labels(args),
            ("properties", 2) => self.properties(args),
            ("out_edges", 2) => self.edge_lists(args, Direction::Out, None),
            ("out_edges", 3) => self.edge_lists(args, Direction::Out, args.get(1)),
            ("in_edges", 2) => self.edge_lists(args, Direction::In, None),
            ("in_edges", 3) => self.edge_lists(args, Direction::In, args.get(1)),
            ("neighbors", 2) => self.neighbors(args, None),
            ("neighbors", 3) => self.neighbors(args, args.get(1)),
            ("node_view", 2) => self.node_view(args),
            _ => Ok(Vec::new()),
        }
    }
}

impl GraphViewProvider<'_> {
    fn labels(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        if let Some(id) = bound_atom(args.first()) {
            return Ok(self
                .storage
                .get_node(id)
                .map_err(provider_error)?
                .map(|node| vec![labels_term(&node.id, node.labels)])
                .unwrap_or_default());
        }
        self.storage
            .get_all_nodes()
            .map_err(provider_error)
            .map(|nodes| {
                nodes
                    .into_iter()
                    .map(|node| labels_term(&node.id, node.labels))
                    .collect()
            })
    }

    fn properties(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        if let Some(id) = bound_atom(args.first()) {
            return Ok(self
                .storage
                .get_node(id)
                .map_err(provider_error)?
                .map(|node| vec![properties_term(&node)])
                .unwrap_or_default());
        }
        self.storage
            .get_all_nodes()
            .map_err(provider_error)
            .map(|nodes| nodes.iter().map(properties_term).collect())
    }

    fn edge_lists(
        &self,
        args: &[Term],
        direction: Direction,
        type_arg: Option<&Term>,
    ) -> WamResult<Vec<Term>> {
        let type_name = type_arg.and_then(|term| bound_atom(Some(term)));
        let arity = args.len();
        if let Some(node) = bound_atom(args.first()) {
            let edges = match direction {
                Direction::Out => self.storage.get_outgoing_edges(node, type_name),
                Direction::In => self.storage.get_incoming_edges(node, type_name),
            }
            .map_err(provider_error)?;
            return Ok(vec![edge_list_term(
                direction, node, type_name, edges, arity,
            )]);
        }
        let mut rows = Vec::new();
        for edge in self.storage.get_all_edges().map_err(provider_error)? {
            if type_name.is_some_and(|wanted| wanted != edge.edge_type) {
                continue;
            }
            let node = endpoint(direction, &edge).to_string();
            let edge_type = type_name
                .map(str::to_string)
                .unwrap_or_else(|| edge.edge_type.clone());
            rows.push(edge_list_term(
                direction,
                &node,
                Some(&edge_type),
                vec![edge],
                arity,
            ));
        }
        Ok(rows)
    }

    fn neighbors(&self, args: &[Term], type_arg: Option<&Term>) -> WamResult<Vec<Term>> {
        let type_name = type_arg.and_then(|term| bound_atom(Some(term)));
        let edges = if let Some(node) = bound_atom(args.first()) {
            self.storage.get_outgoing_edges(node, type_name)
        } else {
            self.storage.get_all_edges()
        }
        .map_err(provider_error)?;
        Ok(edges
            .into_iter()
            .filter(|edge| type_name.is_none_or(|wanted| wanted == edge.edge_type))
            .map(|edge| {
                if args.len() == 3 {
                    compound_term(
                        "neighbors",
                        vec![atom(edge.source), atom(edge.edge_type), atom(edge.target)],
                    )
                } else {
                    compound_term("neighbors", vec![atom(edge.source), atom(edge.target)])
                }
            })
            .collect())
    }

    fn node_view(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let edges = self.storage.get_all_edges().map_err(provider_error)?;
        if let Some(id) = bound_atom(args.first()) {
            return Ok(self
                .storage
                .get_node(id)
                .map_err(provider_error)?
                .map(|node| vec![node_view_term(&node, &edges)])
                .unwrap_or_default());
        }
        self.storage
            .get_all_nodes()
            .map_err(provider_error)
            .map(|nodes| {
                nodes
                    .iter()
                    .map(|node| node_view_term(node, &edges))
                    .collect()
            })
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Out,
    In,
}

fn edge_list_term(
    direction: Direction,
    node: &str,
    edge_type: Option<&str>,
    edges: Vec<Edge>,
    arity: usize,
) -> Term {
    let list = edges
        .iter()
        .map(|edge| match direction {
            Direction::Out => edge_out_term(edge),
            Direction::In => edge_in_term(edge),
        })
        .collect();
    let name = match direction {
        Direction::Out => "out_edges",
        Direction::In => "in_edges",
    };
    let mut args = vec![atom(node)];
    if arity == 3 {
        args.push(atom(edge_type.unwrap_or("")));
    }
    args.push(Term::List(list));
    compound_term(name, args)
}

fn node_view_term(node: &Node, all_edges: &[Edge]) -> Term {
    let labels = node.labels.iter().cloned().map(atom).collect();
    let out_edges = all_edges
        .iter()
        .filter(|edge| edge.source == node.id)
        .map(edge_out_term)
        .collect();
    let in_edges = all_edges
        .iter()
        .filter(|edge| edge.target == node.id)
        .map(edge_in_term)
        .collect();
    compound_term(
        "node_view",
        vec![
            atom(&node.id),
            Term::Object(vec![
                ("id".to_string(), atom(&node.id)),
                ("labels".to_string(), Term::List(labels)),
                (
                    "properties".to_string(),
                    properties_to_object(&node.properties),
                ),
                ("out_edges".to_string(), Term::List(out_edges)),
                ("in_edges".to_string(), Term::List(in_edges)),
            ]),
        ],
    )
}

fn labels_term(id: &str, labels: Vec<String>) -> Term {
    compound_term(
        "labels",
        vec![atom(id), Term::List(labels.into_iter().map(atom).collect())],
    )
}

fn properties_term(node: &Node) -> Term {
    compound_term(
        "properties",
        vec![atom(&node.id), properties_to_object(&node.properties)],
    )
}

fn endpoint(direction: Direction, edge: &Edge) -> &str {
    match direction {
        Direction::Out => &edge.source,
        Direction::In => &edge.target,
    }
}

fn unbound_goal(name: &str, arity: usize) -> Term {
    compound_term(
        name,
        (0..arity)
            .map(|idx| Term::Variable(format!("_V{idx}")))
            .collect(),
    )
}

fn bound_atom(term: Option<&Term>) -> Option<&str> {
    match term {
        Some(Term::Atom(value) | Term::String(value)) => Some(value),
        _ => None,
    }
}

fn atom(value: impl Into<String>) -> Term {
    Term::Atom(value.into())
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
