use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::parser::{PrologRule, Term};

pub(crate) fn permanent_slots(rule: &PrologRule) -> HashMap<String, usize> {
    let mut goal_counts: BTreeMap<String, usize> = BTreeMap::new();
    for goal in std::iter::once(&rule.head).chain(rule.body.iter()) {
        for name in variables_in_goal(goal) {
            *goal_counts.entry(name).or_default() += 1;
        }
    }
    goal_counts
        .into_iter()
        .filter(|(name, count)| name != "_" && *count > 1 && appears_in_body(rule, name))
        .enumerate()
        .map(|(slot, (name, _))| (name, slot))
        .collect()
}

fn appears_in_body(rule: &PrologRule, name: &str) -> bool {
    rule.body
        .iter()
        .any(|goal| variables_in_goal(goal).contains(name))
}

fn variables_in_goal(term: &Term) -> BTreeSet<String> {
    let mut vars = BTreeSet::new();
    collect_variables(term, &mut vars);
    vars
}

fn collect_variables(term: &Term, vars: &mut BTreeSet<String>) {
    match term {
        Term::Variable(name) => {
            vars.insert(name.clone());
        }
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
        Term::Atom(_) | Term::Integer(_) | Term::Float(_) | Term::String(_) => {}
    }
}
