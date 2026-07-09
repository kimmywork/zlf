use crate::parser::Term;

use super::dependency_graph::RuleDependencyGraph;
use super::error::WamResult;
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;
use super::predicate_registry::{builtin_predicates, PredicateKind, PredicateRegistry};

/// A FactProvider that materializes introspection predicates
/// (predicate/3, builtin_predicate/3, rule/3, rule_depends_on/2)
/// from a PredicateRegistry and RuleDependencyGraph.
pub struct IntrospectionProvider {
    registry: PredicateRegistry,
    dependency_graph: RuleDependencyGraph,
}

impl IntrospectionProvider {
    pub fn new(registry: PredicateRegistry, dependency_graph: RuleDependencyGraph) -> Self {
        Self {
            registry,
            dependency_graph,
        }
    }
}

impl FactProvider for IntrospectionProvider {
    fn facts_for(&self, key: &PredicateKey) -> WamResult<Vec<Term>> {
        match (key.name.as_str(), key.arity) {
            ("predicate", 3) => Ok(self.predicate_facts()),
            ("builtin_predicate", 3) => Ok(self.builtin_facts()),
            ("rule", 3) => Ok(self.rule_facts()),
            ("rule_depends_on", 2) => Ok(self.rule_depends_facts()),
            _ => Ok(Vec::new()),
        }
    }

    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        if let Term::Compound { name, args } = goal {
            match (name.as_str(), args.len()) {
                ("predicate", 3)
                | ("builtin_predicate", 3)
                | ("rule", 3)
                | ("rule_depends_on", 2) => {
                    return self.facts_for(&PredicateKey {
                        name: name.clone(),
                        arity: args.len(),
                    });
                }
                _ => {}
            }
        }
        Ok(Vec::new())
    }
}

impl IntrospectionProvider {
    fn predicate_facts(&self) -> Vec<Term> {
        self.registry
            .all()
            .iter()
            .map(|(key, kind)| {
                compound_term(
                    "predicate",
                    vec![
                        atom(&key.name),
                        number(key.arity as f64),
                        atom(kind_name(kind)),
                    ],
                )
            })
            .collect()
    }

    fn builtin_facts(&self) -> Vec<Term> {
        builtin_predicates()
            .into_iter()
            .map(|(key, desc)| {
                compound_term(
                    "builtin_predicate",
                    vec![atom(&key.name), number(key.arity as f64), atom(desc)],
                )
            })
            .collect()
    }

    fn rule_facts(&self) -> Vec<Term> {
        self.dependency_graph
            .all_rules()
            .into_iter()
            .map(|key| {
                let source = format!("{}/{}", key.name, key.arity);
                compound_term(
                    "rule",
                    vec![atom(&key.name), number(key.arity as f64), atom(&source)],
                )
            })
            .collect()
    }

    fn rule_depends_facts(&self) -> Vec<Term> {
        let mut facts = Vec::new();
        for rule_key in self.dependency_graph.all_rules() {
            for dep_key in self.dependency_graph.dependencies(rule_key) {
                let rule_str = format!("{}/{}", rule_key.name, rule_key.arity);
                let dep_str = format!("{}/{}", dep_key.name, dep_key.arity);
                facts.push(compound_term(
                    "rule_depends_on",
                    vec![atom(&rule_str), atom(&dep_str)],
                ));
            }
        }
        facts
    }
}

fn kind_name(kind: &PredicateKind) -> &'static str {
    match kind {
        PredicateKind::BuiltinCore => "builtin",
        PredicateKind::StorageProvider => "storage",
        PredicateKind::IndexProvider => "index",
        PredicateKind::LabelShortcut => "label_shortcut",
        PredicateKind::EdgeShortcut => "edge_shortcut",
        PredicateKind::PropertyShortcut => "property_shortcut",
        PredicateKind::UserRule => "user_rule",
        PredicateKind::GraphAlgorithm => "graph_algorithm",
        PredicateKind::Introspection => "introspection",
    }
}

fn atom(value: impl Into<String>) -> Term {
    Term::Atom(value.into())
}

fn number(value: f64) -> Term {
    Term::Number(value)
}

fn compound_term(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}
