use std::collections::{HashMap, HashSet, VecDeque};

use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;
use super::view_helpers::compound_term;
use zlf_storage::Storage;

const DEFAULT_MAX_DEPTH: usize = 32;
const MAX_VISITED: usize = 100_000;
const MAX_RESULTS: usize = 10_000;

/// A FactProvider that materializes graph algorithm predicates
/// (reachable/2-3, shortest_path/3, degree/2, in_degree/2, out_degree/2)
/// using storage-backed BFS and edge indexes.
pub struct GraphAlgorithmProvider<'a> {
    storage: &'a Storage,
}

impl<'a> GraphAlgorithmProvider<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }
}

impl FactProvider for GraphAlgorithmProvider<'_> {
    fn facts_for(&self, key: &PredicateKey) -> WamResult<Vec<Term>> {
        match (key.name.as_str(), key.arity) {
            ("degree", 2) | ("in_degree", 2) | ("out_degree", 2) => self.degree_facts(&key.name),
            _ => Ok(Vec::new()),
        }
    }

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        if let Term::Compound { name, args } = goal {
            match (name.as_str(), args.len()) {
                ("reachable", 2) => self.reachable_facts(args, DEFAULT_MAX_DEPTH),
                ("reachable", 3) => {
                    let depth = bound_usize(&args[2]).unwrap_or(DEFAULT_MAX_DEPTH);
                    self.reachable_facts(args, depth)
                }
                ("shortest_path", 3) => self.shortest_path_facts(args),
                ("degree", 2) | ("in_degree", 2) | ("out_degree", 2) => {
                    self.degree_facts_for(name, args)
                }
                _ => Ok(Vec::new()),
            }
        } else {
            Ok(Vec::new())
        }
    }
}

impl GraphAlgorithmProvider<'_> {
    #[allow(clippy::too_many_lines)]
    fn reachable_facts(&self, args: &[Term], max_depth: usize) -> WamResult<Vec<Term>> {
        let source = match &args[0] {
            Term::Atom(s) | Term::String(s) => s.clone(),
            _ => return Ok(Vec::new()),
        };
        let target = &args[1];
        if let Term::Atom(t) | Term::String(t) = target {
            // reachable(+Source, +Target, +MaxDepth)
            let found = self.bfs_reachable_bool(&source, t, max_depth)?;
            if found {
                return Ok(vec![compound_term(
                    "reachable",
                    vec![
                        Term::Atom(source.clone()),
                        Term::Atom(t.clone()),
                        number(max_depth as f64),
                    ],
                )]);
            }
            return Ok(Vec::new());
        }
        // reachable(+Source, -Target, +MaxDepth)
        let targets = self.bfs_reachable_targets(&source, max_depth)?;
        Ok(targets
            .into_iter()
            .map(|t| {
                compound_term(
                    "reachable",
                    vec![
                        Term::Atom(source.clone()),
                        Term::Atom(t),
                        number(max_depth as f64),
                    ],
                )
            })
            .collect())
    }

    fn shortest_path_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let source = match &args[0] {
            Term::Atom(s) | Term::String(s) => s.clone(),
            _ => return Ok(Vec::new()),
        };
        let target = match &args[1] {
            Term::Atom(t) | Term::String(t) => t.clone(),
            _ => return Ok(Vec::new()),
        };
        let path = self.bfs_shortest_path(&source, &target)?;
        match path {
            Some(nodes) => Ok(vec![compound_term(
                "shortest_path",
                vec![
                    Term::Atom(source.clone()),
                    Term::Atom(target.clone()),
                    Term::List(nodes.into_iter().map(Term::Atom).collect()),
                ],
            )]),
            None => Ok(Vec::new()),
        }
    }

    fn degree_facts(&self, _name: &str) -> WamResult<Vec<Term>> {
        // Enumerate all nodes with their degrees
        let nodes = self.storage.get_all_nodes().map_err(provider_error)?;
        let mut facts = Vec::new();
        for node in &nodes {
            let out = self
                .storage
                .count_outgoing_edges(&node.id)
                .map_err(provider_error)?;
            let inc = self
                .storage
                .count_incoming_edges(&node.id)
                .map_err(provider_error)?;
            let degree = out + inc;
            facts.push(compound_term(
                "degree",
                vec![Term::Atom(node.id.clone()), number(degree as f64)],
            ));
            facts.push(compound_term(
                "out_degree",
                vec![Term::Atom(node.id.clone()), number(out as f64)],
            ));
            facts.push(compound_term(
                "in_degree",
                vec![Term::Atom(node.id.clone()), number(inc as f64)],
            ));
        }
        Ok(facts)
    }

    fn degree_facts_for(&self, name: &str, args: &[Term]) -> WamResult<Vec<Term>> {
        let node = match &args[0] {
            Term::Atom(s) | Term::String(s) => s.clone(),
            _ => return Ok(Vec::new()),
        };
        let (out, inc) = (
            self.storage
                .count_outgoing_edges(&node)
                .map_err(provider_error)?,
            self.storage
                .count_incoming_edges(&node)
                .map_err(provider_error)?,
        );
        let degree = out + inc;
        let value = match name {
            "degree" => degree,
            "out_degree" => out,
            "in_degree" => inc,
            _ => degree,
        };
        Ok(vec![compound_term(
            name,
            vec![Term::Atom(node), number(value as f64)],
        )])
    }

    // --- BFS helpers ---

    fn bfs_reachable_bool(&self, source: &str, target: &str, max_depth: usize) -> WamResult<bool> {
        if source == target {
            return Ok(true);
        }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(source.to_string());
        queue.push_back((source.to_string(), 0usize));
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            let neighbors = self
                .storage
                .get_outgoing_neighbors(&node)
                .map_err(provider_error)?;
            for next in neighbors {
                if next == target {
                    return Ok(true);
                }
                if visited.len() >= MAX_VISITED {
                    return Ok(false);
                }
                if visited.insert(next.clone()) {
                    queue.push_back((next, depth + 1));
                }
            }
        }
        Ok(false)
    }

    fn bfs_reachable_targets(&self, source: &str, max_depth: usize) -> WamResult<Vec<String>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();
        visited.insert(source.to_string());
        queue.push_back((source.to_string(), 0usize));
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            let neighbors = self
                .storage
                .get_outgoing_neighbors(&node)
                .map_err(provider_error)?;
            for next in neighbors {
                if results.len() >= MAX_RESULTS {
                    return Ok(results);
                }
                if visited.len() >= MAX_VISITED {
                    return Ok(results);
                }
                if visited.insert(next.clone()) {
                    results.push(next.clone());
                    queue.push_back((next, depth + 1));
                }
            }
        }
        Ok(results)
    }

    fn bfs_shortest_path(&self, source: &str, target: &str) -> WamResult<Option<Vec<String>>> {
        if source == target {
            return Ok(Some(vec![source.to_string()]));
        }
        let mut visited = HashSet::new();
        let mut parent: HashMap<String, String> = HashMap::new();
        let mut queue = VecDeque::new();
        visited.insert(source.to_string());
        queue.push_back(source.to_string());
        while let Some(node) = queue.pop_front() {
            if visited.len() >= MAX_VISITED {
                return Ok(None);
            }
            let neighbors = self
                .storage
                .get_outgoing_neighbors(&node)
                .map_err(provider_error)?;
            for next in neighbors {
                if visited.insert(next.clone()) {
                    parent.insert(next.clone(), node.clone());
                    if next == target {
                        return Ok(Some(reconstruct_path(&parent, source, target)));
                    }
                    queue.push_back(next);
                }
            }
        }
        Ok(None)
    }
}

fn reconstruct_path(parent: &HashMap<String, String>, source: &str, target: &str) -> Vec<String> {
    let mut path = vec![target.to_string()];
    let mut current = target;
    while current != source {
        if let Some(prev) = parent.get(current) {
            path.push(prev.clone());
            current = prev;
        } else {
            break;
        }
    }
    path.reverse();
    path
}

fn bound_usize(term: &Term) -> Option<usize> {
    match term {
        Term::Number(n) if *n >= 0.0 => Some(*n as usize),
        _ => None,
    }
}

fn number(value: f64) -> Term {
    Term::Number(value)
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
