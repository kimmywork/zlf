use std::collections::{HashMap, HashSet};

use crate::wam::predicate::{predicate_key, PredicateKey};
use crate::wam::runtime::WamRuntime;

pub(crate) fn component(runtime: &WamRuntime, target: &PredicateKey) -> HashSet<PredicateKey> {
    let graph = dependency_graph(runtime);
    let forward = reachable(&graph, target);
    let reverse = reverse_graph(&graph);
    let backward = reachable(&reverse, target);
    forward.intersection(&backward).cloned().collect()
}

fn dependency_graph(runtime: &WamRuntime) -> HashMap<PredicateKey, HashSet<PredicateKey>> {
    let mut graph: HashMap<PredicateKey, HashSet<PredicateKey>> = HashMap::new();
    for rule in runtime.rules.iter().chain(
        runtime
            .compiled_rules
            .iter()
            .map(|artifact| &artifact.source),
    ) {
        let Some(head) = predicate_key(&rule.head) else {
            continue;
        };
        let dependencies = graph.entry(head).or_default();
        dependencies.extend(rule.body.iter().filter_map(predicate_key));
    }
    graph
}

fn reverse_graph(
    graph: &HashMap<PredicateKey, HashSet<PredicateKey>>,
) -> HashMap<PredicateKey, HashSet<PredicateKey>> {
    let mut reverse = HashMap::new();
    for (source, targets) in graph {
        reverse.entry(source.clone()).or_insert_with(HashSet::new);
        for target in targets {
            reverse
                .entry(target.clone())
                .or_insert_with(HashSet::new)
                .insert(source.clone());
        }
    }
    reverse
}

fn reachable(
    graph: &HashMap<PredicateKey, HashSet<PredicateKey>>,
    start: &PredicateKey,
) -> HashSet<PredicateKey> {
    let mut found = HashSet::from([start.clone()]);
    let mut stack = vec![start.clone()];
    while let Some(key) = stack.pop() {
        if let Some(next) = graph.get(&key) {
            for dependency in next {
                if found.insert(dependency.clone()) {
                    stack.push(dependency.clone());
                }
            }
        }
    }
    found
}
