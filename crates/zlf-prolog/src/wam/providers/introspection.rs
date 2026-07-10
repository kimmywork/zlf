use crate::parser::{PrologRule, Term};

use super::builtin_catalog::builtin_predicates;
use super::dependency_graph::RuleDependencyGraph;
use super::error::WamResult;
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;
use super::predicate_registry::{PredicateKind, PredicateRegistry};
use super::rule_store::CompiledRuleArtifact;

/// A FactProvider that materializes introspection predicates
/// (predicate/3, builtin_predicate/3, rule/3, rule_depends_on/2)
/// from a PredicateRegistry and RuleDependencyGraph.
pub struct IntrospectionProvider {
    registry: PredicateRegistry,
    dependency_graph: RuleDependencyGraph,
    rule_sources: Vec<(PredicateKey, String)>,
}

impl IntrospectionProvider {
    pub fn new(registry: PredicateRegistry, rules: &[CompiledRuleArtifact]) -> Self {
        Self {
            registry,
            dependency_graph: RuleDependencyGraph::from_rules(rules),
            rule_sources: rules
                .iter()
                .map(|rule| (rule.key.clone(), rule_to_source(&rule.source)))
                .collect(),
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
                    vec![
                        atom(&key.name),
                        number(key.arity as f64),
                        Term::String(desc.to_string()),
                    ],
                )
            })
            .collect()
    }

    fn rule_facts(&self) -> Vec<Term> {
        self.rule_sources
            .iter()
            .map(|(key, source)| {
                compound_term(
                    "rule",
                    vec![
                        atom(&key.name),
                        number(key.arity as f64),
                        Term::String(source.clone()),
                    ],
                )
            })
            .collect()
    }

    fn rule_depends_facts(&self) -> Vec<Term> {
        let mut facts = Vec::new();
        for rule_key in self.dependency_graph.all_rules() {
            for dep_key in self.dependency_graph.dependencies(rule_key) {
                facts.push(compound_term(
                    "rule_depends_on",
                    vec![
                        Term::String(indicator(rule_key)),
                        Term::String(indicator(dep_key)),
                    ],
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

fn indicator(key: &PredicateKey) -> String {
    format!("{}/{}", key.name, key.arity)
}

fn rule_to_source(rule: &PrologRule) -> String {
    let body = rule
        .body
        .iter()
        .map(term_to_source)
        .collect::<Vec<_>>()
        .join(", ");
    if body.is_empty() {
        format!("{}.", term_to_source(&rule.head))
    } else {
        format!("{} :- {}.", term_to_source(&rule.head), body)
    }
}

#[allow(clippy::too_many_lines)]
fn term_to_source(term: &Term) -> String {
    match term {
        Term::Variable(name) | Term::Atom(name) => name.clone(),
        Term::String(value) => format!("\"{}\"", value.replace('"', "\\\"")),
        Term::Integer(value) => value.to_string(),
        Term::Float(value) => value.to_string(),
        Term::Compound { name, args } => format!(
            "{}({})",
            name,
            args.iter()
                .map(term_to_source)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Term::List(items) => format!(
            "[{}]",
            items
                .iter()
                .map(term_to_source)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Term::Object(entries) => format!(
            "{{ {} }}",
            entries
                .iter()
                .map(|(key, value)| format!("{}: {}", key, term_to_source(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn atom(value: impl Into<String>) -> Term {
    Term::Atom(value.into())
}

fn number(value: f64) -> Term {
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        Term::Integer(value as i64)
    } else {
        Term::Float(value)
    }
}

fn compound_term(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}
