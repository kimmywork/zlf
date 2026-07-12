use std::collections::{HashMap, HashSet};

use zlf_prolog::Term;

use crate::PrologRule;

/// Build a query plan from a list of body terms.
/// A single term is the query directly. Multiple terms get a wrapper rule.
pub fn query_plan(terms: &[Term]) -> (Term, Option<PrologRule>) {
    if terms.len() == 1 {
        return (terms[0].clone(), None);
    }
    let vars = query_variables(terms);
    let head = Term::Compound {
        name: "__query".to_string(),
        args: vars
            .iter()
            .map(|name| Term::Variable(name.clone()))
            .collect(),
    };
    (
        head.clone(),
        Some(PrologRule {
            head,
            body: terms.to_vec(),
        }),
    )
}

fn query_variables(terms: &[Term]) -> Vec<String> {
    let mut vars = Vec::new();
    for term in terms {
        collect_variables(term, &mut vars);
    }
    vars
}

fn collect_variables(term: &Term, vars: &mut Vec<String>) {
    match term {
        Term::Variable(name) if name != "_" && !vars.contains(name) => vars.push(name.clone()),
        Term::Compound { args, .. } | Term::List(args) => {
            for arg in args {
                collect_variables(arg, vars);
            }
        }
        Term::Object(entries) => {
            for (_, value) in entries {
                collect_variables(value, vars);
            }
        }
        _ => {}
    }
}

/// Convert a binding map to a JSON object value.
pub fn solution_to_json(solution: HashMap<String, Term>) -> serde_json::Value {
    serde_json::Value::Object(
        solution
            .into_iter()
            .filter(|(name, _)| name != "_")
            .map(|(name, term)| (name, term_to_json(&term)))
            .collect(),
    )
}

fn term_to_json(term: &Term) -> serde_json::Value {
    match term {
        Term::Variable(name) => serde_json::json!({ "variable": name }),
        Term::Atom(name) | Term::String(name) => serde_json::json!(name),
        Term::Integer(number) => serde_json::json!(number),
        Term::Float(number) => serde_json::json!(number),
        Term::Compound { name, args } => serde_json::json!({
            "name": name,
            "args": args.iter().map(term_to_json).collect::<Vec<_>>()
        }),
        Term::List(items) => serde_json::json!(items.iter().map(term_to_json).collect::<Vec<_>>()),
        Term::Object(entries) => serde_json::Value::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), term_to_json(value)))
                .collect(),
        ),
    }
}

pub fn dedupe_results(results: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    let mut seen = HashSet::new();
    results
        .into_iter()
        .filter(|row| seen.insert(serde_json::to_string(row).unwrap_or_default()))
        .collect()
}

pub fn lock_error(error: impl std::fmt::Display) -> zlf_core::ZlfError {
    zlf_core::ZlfError::Internal(error.to_string())
}
