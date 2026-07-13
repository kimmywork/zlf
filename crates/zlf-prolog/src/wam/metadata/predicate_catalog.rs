use super::predicate::PredicateKey;

/// Graph view provider predicates.
#[allow(clippy::too_many_lines)]
pub fn graph_view_predicates() -> Vec<PredicateKey> {
    vec![
        PredicateKey {
            name: "labels".into(),
            arity: 2,
        },
        PredicateKey {
            name: "properties".into(),
            arity: 2,
        },
        PredicateKey {
            name: "out_edges".into(),
            arity: 2,
        },
        PredicateKey {
            name: "out_edges".into(),
            arity: 3,
        },
        PredicateKey {
            name: "in_edges".into(),
            arity: 2,
        },
        PredicateKey {
            name: "in_edges".into(),
            arity: 3,
        },
        PredicateKey {
            name: "neighbors".into(),
            arity: 2,
        },
        PredicateKey {
            name: "neighbors".into(),
            arity: 3,
        },
        PredicateKey {
            name: "node_view".into(),
            arity: 2,
        },
    ]
}

/// Graph algorithm provider predicates.
pub fn graph_algorithm_predicates() -> Vec<PredicateKey> {
    vec![
        PredicateKey {
            name: "reachable".into(),
            arity: 2,
        },
        PredicateKey {
            name: "reachable".into(),
            arity: 3,
        },
        PredicateKey {
            name: "shortest_path".into(),
            arity: 3,
        },
        PredicateKey {
            name: "degree".into(),
            arity: 2,
        },
        PredicateKey {
            name: "in_degree".into(),
            arity: 2,
        },
        PredicateKey {
            name: "out_degree".into(),
            arity: 2,
        },
    ]
}

/// Index provider predicates.
pub fn index_predicates() -> Vec<PredicateKey> {
    vec![
        PredicateKey {
            name: "bm25".into(),
            arity: 3,
        },
        PredicateKey {
            name: "vector_similar".into(),
            arity: 3,
        },
        PredicateKey {
            name: "temporal_on".into(),
            arity: 2,
        },
        PredicateKey {
            name: "temporal_between".into(),
            arity: 3,
        },
        PredicateKey {
            name: "valid_at".into(),
            arity: 2,
        },
        PredicateKey {
            name: "valid_overlaps".into(),
            arity: 3,
        },
    ]
}
