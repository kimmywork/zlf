use std::collections::{HashMap, HashSet};

use crate::parser::{PrologRule, Term};

pub fn permanent_variables(rule: &PrologRule) -> Vec<String> {
    let mut appearances: HashMap<String, HashSet<usize>> = HashMap::new();
    for (goal_index, goal) in rule.body.iter().enumerate() {
        for variable in variables_in(goal) {
            appearances.entry(variable).or_default().insert(goal_index);
        }
    }
    let mut variables: Vec<_> = appearances
        .into_iter()
        .filter_map(|(name, goals)| (goals.len() > 1).then_some(name))
        .collect();
    variables.sort();
    variables
}

pub fn variables_in(term: &Term) -> Vec<String> {
    let mut variables = Vec::new();
    collect_variables(term, &mut variables);
    variables.sort();
    variables.dedup();
    variables
}

fn collect_variables(term: &Term, variables: &mut Vec<String>) {
    match term {
        Term::Variable(name) if name != "_" => variables.push(name.clone()),
        Term::Compound { args, .. } | Term::List(args) => {
            for arg in args {
                collect_variables(arg, variables);
            }
        }
        _ => {}
    }
}
