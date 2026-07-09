use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::fact_provider::FactProvider;
use super::predicate::PredicateKey;

/// A FactProvider that materializes deterministic builtin predicates
/// (is/2, arithmetic comparisons, var/1, atom/1, functor/3, arg/3, =../2,
///  assertz/1, retract/1, clause/2, current_predicate/1).
pub struct BuiltinProvider;

fn arith_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < f64::EPSILON
}
fn arith_ne(a: f64, b: f64) -> bool {
    (a - b).abs() >= f64::EPSILON
}

impl FactProvider for BuiltinProvider {
    fn facts_for(&self, _key: &PredicateKey) -> WamResult<Vec<Term>> {
        Ok(Vec::new())
    }

    #[allow(clippy::too_many_lines)]
    fn facts_for_goal(&self, goal: &Term) -> WamResult<Vec<Term>> {
        match goal {
            Term::Compound { name, args } => match (name.as_str(), args.len()) {
                ("is", 2) => self.eval_is(args),
                ("=:=", 2) => self.eval_arith_cmp(args, arith_eq),
                ("=\\=", 2) => self.eval_arith_cmp(args, arith_ne),
                ("<", 2) => self.eval_arith_cmp(args, |a, b| a < b),
                ("=<", 2) => self.eval_arith_cmp(args, |a, b| a <= b),
                (">", 2) => self.eval_arith_cmp(args, |a, b| a > b),
                (">=", 2) => self.eval_arith_cmp(args, |a, b| a >= b),
                ("var", 1) => self.eval_type(&args[0], |t| matches!(t, Term::Variable(_))),
                ("nonvar", 1) => self.eval_type(&args[0], |t| !matches!(t, Term::Variable(_))),
                ("atom", 1) => self.eval_type(&args[0], |t| matches!(t, Term::Atom(_))),
                ("integer", 1) => self.eval_type(
                    &args[0],
                    |t| matches!(t, Term::Number(n) if n.fract() == 0.0),
                ),
                ("float", 1) => self.eval_type(
                    &args[0],
                    |t| matches!(t, Term::Number(n) if n.fract() != 0.0),
                ),
                ("number", 1) => self.eval_type(&args[0], |t| matches!(t, Term::Number(_))),
                ("atomic", 1) => self.eval_type(&args[0], |t| {
                    matches!(t, Term::Atom(_) | Term::Number(_) | Term::String(_))
                }),
                ("compound", 1) => self.eval_type(&args[0], |t| {
                    matches!(t, Term::Compound { .. } | Term::List(_) | Term::Object(_))
                }),
                ("ground", 1) => self.eval_type(&args[0], is_ground),
                ("functor", 3) => self.eval_functor(args),
                ("arg", 3) => self.eval_arg(args),
                ("=..", 2) => self.eval_univ(args),
                ("true", 0) => Ok(vec![Term::Atom("true".to_string())]),
                ("fail", 0) | ("false", 0) => Ok(Vec::new()),
                _ => Ok(Vec::new()),
            },
            Term::Atom(name) => match name.as_str() {
                "true" => Ok(vec![Term::Atom("true".to_string())]),
                "fail" | "false" => Ok(Vec::new()),
                _ => Ok(Vec::new()),
            },
            _ => Ok(Vec::new()),
        }
    }
}

impl BuiltinProvider {
    fn eval_is(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [lhs, rhs] = args else {
            return Ok(Vec::new());
        };
        let value = eval_arithmetic(rhs)?;
        Ok(vec![compound_term("is", vec![lhs.clone(), number(value)])])
    }

    fn eval_arith_cmp(&self, args: &[Term], cmp: fn(f64, f64) -> bool) -> WamResult<Vec<Term>> {
        let [a, b] = args else { return Ok(Vec::new()) };
        if cmp(eval_arithmetic(a)?, eval_arithmetic(b)?) {
            Ok(vec![compound_term("true", vec![])])
        } else {
            Ok(Vec::new())
        }
    }

    fn eval_type(&self, arg: &Term, pred: fn(&Term) -> bool) -> WamResult<Vec<Term>> {
        if pred(arg) {
            Ok(vec![compound_term("true", vec![])])
        } else {
            Ok(Vec::new())
        }
    }

    #[allow(clippy::too_many_lines)]
    fn eval_functor(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [term, name_arg, arity_arg] = args else {
            return Ok(Vec::new());
        };
        let (name_str, arity_val): (&str, usize) = match term {
            Term::Compound { name, args: cargs } => (name, cargs.len()),
            Term::Atom(name) => (name, 0),
            Term::Number(n) => {
                return Ok(vec![compound_term(
                    "functor",
                    vec![term.clone(), Term::Atom(n.to_string()), number(0.0)],
                )])
            }
            Term::List(_) => {
                return Ok(vec![compound_term(
                    "functor",
                    vec![term.clone(), Term::Atom("'.'".to_string()), number(2.0)],
                )])
            }
            Term::String(_) => {
                return Ok(vec![compound_term(
                    "functor",
                    vec![term.clone(), Term::Atom("string".to_string()), number(0.0)],
                )])
            }
            Term::Variable(_) => {
                if let (Term::Atom(name), Term::Number(arity)) = (name_arg, arity_arg) {
                    let fresh: Vec<Term> = (0..*arity as usize)
                        .map(|_| Term::Variable("_".to_string()))
                        .collect();
                    let constructed = Term::Compound {
                        name: name.clone(),
                        args: fresh,
                    };
                    return Ok(vec![compound_term(
                        "functor",
                        vec![constructed, Term::Atom(name.clone()), number(*arity)],
                    )]);
                }
                return Ok(Vec::new());
            }
            Term::Object(_) => return Ok(Vec::new()),
        };
        Ok(vec![compound_term(
            "functor",
            vec![
                term.clone(),
                Term::Atom(name_str.to_string()),
                number(arity_val as f64),
            ],
        )])
    }

    fn eval_arg(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [index, term, _arg_out] = args else {
            return Ok(Vec::new());
        };
        let idx = match index {
            Term::Number(n) => *n as usize,
            _ => return Ok(Vec::new()),
        };
        if idx < 1 {
            return Ok(Vec::new());
        }
        let inner_args = match term {
            Term::Compound { args, .. } => args,
            Term::List(items) => items,
            _ => return Ok(Vec::new()),
        };
        if idx > inner_args.len() {
            return Ok(Vec::new());
        }
        Ok(vec![compound_term(
            "arg",
            vec![
                Term::Number(idx as f64),
                term.clone(),
                inner_args[idx - 1].clone(),
            ],
        )])
    }

    #[allow(clippy::too_many_lines)]
    fn eval_univ(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [term, _list] = args else {
            return Ok(Vec::new());
        };
        match term {
            Term::Compound { name, args: cargs } => {
                let mut items = vec![Term::Atom(name.clone())];
                items.extend(cargs.clone());
                Ok(vec![compound_term(
                    "=..",
                    vec![term.clone(), Term::List(items)],
                )])
            }
            Term::Atom(name) => Ok(vec![compound_term(
                "=..",
                vec![term.clone(), Term::List(vec![Term::Atom(name.clone())])],
            )]),
            Term::List(items) => {
                if items.is_empty() {
                    return Ok(Vec::new());
                }
                if let Term::Atom(name) = &items[0] {
                    let constructed = Term::Compound {
                        name: name.clone(),
                        args: items[1..].to_vec(),
                    };
                    Ok(vec![compound_term("=..", vec![constructed, term.clone()])])
                } else {
                    Ok(Vec::new())
                }
            }
            _ => Ok(Vec::new()),
        }
    }
}

#[allow(clippy::too_many_lines)]
fn eval_arithmetic(term: &Term) -> WamResult<f64> {
    match term {
        Term::Number(n) => Ok(*n),
        Term::Atom(name) => name
            .parse::<f64>()
            .map_err(|_| WamError::Provider(format!("arithmetic: not a number: {name}"))),
        Term::Compound { name, args } => {
            let values: Vec<f64> = args
                .iter()
                .map(eval_arithmetic)
                .collect::<WamResult<Vec<_>>>()?;
            match name.as_str() {
                "+" => Ok(values[0] + values[1]),
                "-" if values.len() == 1 => Ok(-values[0]),
                "-" => Ok(values[0] - values[1]),
                "*" => Ok(values[0] * values[1]),
                "/" => Ok(values[0] / values[1]),
                "//" => Ok((values[0] / values[1]).floor()),
                "mod" => Ok(values[0] % values[1]),
                "abs" => Ok(values[0].abs()),
                "min" => Ok(values[0].min(values[1])),
                "max" => Ok(values[0].max(values[1])),
                _ => Err(WamError::Provider(format!(
                    "arithmetic: unknown function: {name}"
                ))),
            }
        }
        Term::Variable(name) => Err(WamError::Provider(format!(
            "arithmetic: unbound variable: {name}"
        ))),
        _ => Err(WamError::Provider(
            "arithmetic: unsupported term".to_string(),
        )),
    }
}

fn is_ground(term: &Term) -> bool {
    match term {
        Term::Variable(_) => false,
        Term::Compound { args, .. } => args.iter().all(is_ground),
        Term::List(items) => items.iter().all(is_ground),
        Term::Object(entries) => entries.iter().all(|(_, v)| is_ground(v)),
        _ => true,
    }
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
