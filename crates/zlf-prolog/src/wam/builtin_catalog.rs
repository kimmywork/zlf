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
        (
            PredicateKey {
                name: "is".into(),
                arity: 2,
            },
            "arithmetic evaluation",
        ),
        (
            PredicateKey {
                name: "=:=".into(),
                arity: 2,
            },
            "arithmetic equality",
        ),
        (
            PredicateKey {
                name: "=\\=".into(),
                arity: 2,
            },
            "arithmetic inequality",
        ),
        (
            PredicateKey {
                name: "<".into(),
                arity: 2,
            },
            "arithmetic less than",
        ),
        (
            PredicateKey {
                name: "=<".into(),
                arity: 2,
            },
            "arithmetic less or equal",
        ),
        (
            PredicateKey {
                name: ">".into(),
                arity: 2,
            },
            "arithmetic greater than",
        ),
        (
            PredicateKey {
                name: ">=".into(),
                arity: 2,
            },
            "arithmetic greater or equal",
        ),
        (
            PredicateKey {
                name: "var".into(),
                arity: 1,
            },
            "variable test",
        ),
        (
            PredicateKey {
                name: "nonvar".into(),
                arity: 1,
            },
            "non-variable test",
        ),
        (
            PredicateKey {
                name: "atom".into(),
                arity: 1,
            },
            "atom test",
        ),
        (
            PredicateKey {
                name: "integer".into(),
                arity: 1,
            },
            "integer test",
        ),
        (
            PredicateKey {
                name: "float".into(),
                arity: 1,
            },
            "float test",
        ),
        (
            PredicateKey {
                name: "number".into(),
                arity: 1,
            },
            "number test",
        ),
        (
            PredicateKey {
                name: "atomic".into(),
                arity: 1,
            },
            "atomic test",
        ),
        (
            PredicateKey {
                name: "compound".into(),
                arity: 1,
            },
            "compound test",
        ),
        (
            PredicateKey {
                name: "ground".into(),
                arity: 1,
            },
            "ground test",
        ),
        (
            PredicateKey {
                name: "functor".into(),
                arity: 3,
            },
            "term functor",
        ),
        (
            PredicateKey {
                name: "arg".into(),
                arity: 3,
            },
            "term argument",
        ),
        (
            PredicateKey {
                name: "=..".into(),
                arity: 2,
            },
            "univ",
        ),
        (
            PredicateKey {
                name: "assertz".into(),
                arity: 1,
            },
            "assert fact/rule",
        ),
        (
            PredicateKey {
                name: "retract".into(),
                arity: 1,
            },
            "retract fact",
        ),
        (
            PredicateKey {
                name: "clause".into(),
                arity: 2,
            },
            "clause inspection",
        ),
        (
            PredicateKey {
                name: "current_predicate".into(),
                arity: 1,
            },
            "current predicate",
        ),
    ]
}
