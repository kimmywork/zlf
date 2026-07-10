use std::collections::{HashMap, HashSet};

use crate::parser::PrologRule;

use super::predicate::PredicateKey;

/// Compute dependencies between rules in the same rule set.
/// A rule depends on another if any body goal matches the other's predicate key.
#[derive(Debug, Clone, Default)]
pub struct RuleDependencyGraph {
    edges: HashMap<PredicateKey, HashSet<PredicateKey>>,
}

impl RuleDependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build dependency graph from a set of compiled rules.
    pub fn from_rules(rules: &[super::rule_store::CompiledRuleArtifact]) -> Self {
        let mut graph = Self::new();
        for artifact in rules {
            let deps: HashSet<PredicateKey> = dependency_keys(&artifact.source);
            graph.edges.insert(artifact.key.clone(), deps);
        }
        graph
    }

    /// Dependencies of a given rule (direct dependencies only).
    pub fn dependencies(&self, key: &PredicateKey) -> Vec<&PredicateKey> {
        self.edges
            .get(key)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Whether `rule` depends directly on `dep`.
    pub fn depends_on(&self, rule: &PredicateKey, dep: &PredicateKey) -> bool {
        self.edges
            .get(rule)
            .map(|deps| deps.contains(dep))
            .unwrap_or(false)
    }

    /// All rule keys in the graph.
    pub fn all_rules(&self) -> Vec<&PredicateKey> {
        self.edges.keys().collect()
    }

    /// All transitive dependencies of a rule (including indirect).
    pub fn transitive_dependencies(&self, key: &PredicateKey) -> HashSet<&PredicateKey> {
        let mut visited = HashSet::new();
        let mut stack = self.dependencies(key);
        while let Some(dep) = stack.pop() {
            if visited.insert(dep) {
                stack.extend(self.dependencies(dep));
            }
        }
        visited
    }
}

/// Extract all predicate keys referenced in a rule's body.
fn dependency_keys(rule: &PrologRule) -> HashSet<PredicateKey> {
    let mut deps = HashSet::new();
    for goal in &rule.body {
        if let Some(key) = super::predicate_key(goal) {
            deps.insert(key);
        }
    }
    deps
}
