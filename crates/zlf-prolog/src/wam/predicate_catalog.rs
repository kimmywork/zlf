use super::predicate::PredicateKey;

/// Builtin/core predicates that zlf ships.
#[allow(clippy::too_many_lines)]
pub fn builtin_predicates() -> Vec<(PredicateKey, &'static str)> {
    vec![
        (
            PredicateKey {
                name: "true".into(),
                arity: 0,
            },
            "always succeeds",
        ),
        (
            PredicateKey {
                name: "fail".into(),
                arity: 0,
            },
            "always fails",
        ),
        (
            PredicateKey {
                name: "!".into(),
                arity: 0,
            },
            "cut",
        ),
        (
            PredicateKey {
                name: "node".into(),
                arity: 1,
            },
            "storage node existence",
        ),
        (
            PredicateKey {
                name: "label".into(),
                arity: 2,
            },
            "storage label enumeration",
        ),
        (
            PredicateKey {
                name: "property".into(),
                arity: 3,
            },
            "storage property enumeration",
        ),
        (
            PredicateKey {
                name: "edge".into(),
                arity: 3,
            },
            "storage edge enumeration",
        ),
        (
            PredicateKey {
                name: "bm25".into(),
                arity: 3,
            },
            "BM25 full-text search",
        ),
        (
            PredicateKey {
                name: "vector_similar".into(),
                arity: 3,
            },
            "vector similarity search",
        ),
        (
            PredicateKey {
                name: "temporal_on".into(),
                arity: 2,
            },
            "temporal exact-date query",
        ),
        (
            PredicateKey {
                name: "temporal_between".into(),
                arity: 3,
            },
            "temporal date-range query",
        ),
        (
            PredicateKey {
                name: "predicate".into(),
                arity: 3,
            },
            "list all known predicates",
        ),
        (
            PredicateKey {
                name: "builtin_predicate".into(),
                arity: 3,
            },
            "list builtin predicates",
        ),
        (
            PredicateKey {
                name: "rule".into(),
                arity: 3,
            },
            "list user-defined rules",
        ),
        (
            PredicateKey {
                name: "rule_depends_on".into(),
                arity: 2,
            },
            "query rule dependencies",
        ),
    ]
}

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
    ]
}
