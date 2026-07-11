use crate::parser::{PrologRule, Term};

use super::builtin_catalog::builtin_predicates;
use super::error::{WamError, WamResult};
use super::executor::WamExecutor;
use super::fact_lowering::value_to_storage;
use super::predicate::{predicate_key, PredicateKey};
use super::{CompiledRuleArtifact, StorageFactWriter, StorageRuleStore};
use zlf_storage::Storage;

impl WamExecutor {
    pub(crate) fn execute_dynamic_builtin(
        &mut self,
        name: &str,
        arity: usize,
        storage: Option<&Storage>,
    ) -> WamResult<Option<bool>> {
        let Some(storage) = storage else {
            return Ok(None);
        };
        let result = match (name, arity) {
            ("asserta", 1) | ("assertz", 1) => self.assert_term(storage)?,
            ("retract", 1) => self.retract_term(storage, false)?,
            ("retractall", 1) => self.retract_term(storage, true)?,
            ("$current_predicate_bound", 1) | ("current_predicate", 1) => {
                self.current_predicate(storage)?
            }
            ("clause", 2) => self.clause(storage)?,
            ("set_node_property", 3) => self.set_property(storage, true)?,
            ("remove_node_property", 2) => self.remove_property(storage, true)?,
            ("set_edge_property", 3) => self.set_property(storage, false)?,
            ("remove_edge_property", 2) => self.remove_property(storage, false)?,
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    fn assert_term(&self, storage: &Storage) -> WamResult<bool> {
        let term = self.register_term(0)?;
        if let Some(rule) = rule_term(&term) {
            let artifact = CompiledRuleArtifact::compile(&rule)?;
            StorageRuleStore::new(storage).add_compiled_rule(&artifact)?;
        } else {
            StorageFactWriter::new(storage).apply_fact(&term)?;
        }
        Ok(true)
    }

    fn retract_term(&self, storage: &Storage, always_succeed: bool) -> WamResult<bool> {
        let term = self.register_term(0)?;
        let rules = StorageRuleStore::new(storage);
        if let Some(rule) = rule_term(&term) {
            let removed = rules.remove_rule(&rule, always_succeed)? > 0;
            return Ok(always_succeed || removed);
        }
        if always_succeed {
            let removed_fact = StorageFactWriter::new(storage)
                .retract_fact(&term)
                .map(|removed| removed.is_some())
                .unwrap_or(false);
            let removed_rules = predicate_key(&term)
                .map(|key| rules.remove_rules_for(&key))
                .transpose()?
                .unwrap_or_default()
                > 0;
            return Ok(removed_fact || removed_rules || always_succeed);
        }
        Ok(StorageFactWriter::new(storage)
            .retract_fact(&term)?
            .is_some())
    }

    fn set_property(&self, storage: &Storage, node: bool) -> WamResult<bool> {
        let id = term_text(&self.register_term(0)?)?;
        let key = term_text(&self.register_term(1)?)?;
        let value = value_to_storage(&self.register_term(2)?)?;
        let result = if node {
            storage.set_node_property(&id, &key, value)
        } else {
            storage.set_edge_property(&id, &key, value)
        };
        result.map(|_| true).map_err(provider_error)
    }

    fn remove_property(&self, storage: &Storage, node: bool) -> WamResult<bool> {
        let id = term_text(&self.register_term(0)?)?;
        let key = term_text(&self.register_term(1)?)?;
        let result = if node {
            storage.remove_node_property(&id, &key)
        } else {
            storage.remove_edge_property(&id, &key)
        };
        result.map(|_| true).map_err(provider_error)
    }

    fn current_predicate(&self, storage: &Storage) -> WamResult<bool> {
        let indicator = self.register_term(0)?;
        let Some(key) = indicator_key(&indicator) else {
            return Ok(false);
        };
        Ok(predicate_keys(storage)?.contains(&key))
    }

    fn clause(&mut self, storage: &Storage) -> WamResult<bool> {
        let head = self.register_term(0)?;
        let Some(key) = predicate_key(&head) else {
            return Ok(false);
        };
        let rules = StorageRuleStore::new(storage).all_rules()?;
        let Some(rule) = rules.into_iter().find(|rule| rule.key == key) else {
            return Ok(false);
        };
        let body = body_term(&rule.source.body);
        let values = self
            .machine
            .put_terms_shared(&[rule.source.head.clone(), body])?;
        if !self.machine.unify(self.registers.get(0)?, values[0])? {
            return Ok(false);
        }
        self.machine.unify(self.registers.get(1)?, values[1])
    }
}

fn rule_term(term: &Term) -> Option<PrologRule> {
    let Term::Compound { name, args } = term else {
        return None;
    };
    if name != ":-" || args.len() != 2 {
        return None;
    }
    Some(PrologRule {
        head: args[0].clone(),
        body: body_goals(&args[1]),
    })
}

fn body_goals(term: &Term) -> Vec<Term> {
    match term {
        Term::Compound { name, args } if name == "," && args.len() == 2 => {
            let mut goals = body_goals(&args[0]);
            goals.extend(body_goals(&args[1]));
            goals
        }
        goal => vec![goal.clone()],
    }
}

fn indicator_key(term: &Term) -> Option<PredicateKey> {
    match term {
        Term::Atom(text) | Term::String(text) => parse_indicator(text),
        Term::Compound { name, args } if name == "/" && args.len() == 2 => {
            let Term::Atom(name) = &args[0] else {
                return None;
            };
            let Term::Integer(arity) = &args[1] else {
                return None;
            };
            Some(PredicateKey {
                name: name.clone(),
                arity: *arity as usize,
            })
        }
        _ => None,
    }
}

fn parse_indicator(text: &str) -> Option<PredicateKey> {
    let (name, arity) = text.rsplit_once('/')?;
    Some(PredicateKey {
        name: name.to_string(),
        arity: arity.parse().ok()?,
    })
}

fn predicate_keys(storage: &Storage) -> WamResult<Vec<PredicateKey>> {
    let mut keys = builtin_predicates()
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    keys.extend(
        StorageRuleStore::new(storage)
            .all_rules()?
            .into_iter()
            .map(|rule| rule.key),
    );
    keys.extend([
        key("node", 1),
        key("label", 2),
        key("property", 3),
        key("edge", 3),
    ]);
    for node in storage.get_all_nodes().map_err(provider_error)? {
        keys.extend(node.labels.into_iter().map(|label| key(&label, 1)));
        keys.extend(
            node.properties
                .into_keys()
                .map(|property| key(&format!("prop_{property}"), 2)),
        );
    }
    for edge in storage.get_all_edges().map_err(provider_error)? {
        keys.push(key(&edge.edge_type, 2));
    }
    keys.sort_by(|left, right| (&left.name, left.arity).cmp(&(&right.name, right.arity)));
    keys.dedup();
    Ok(keys)
}

fn key(name: &str, arity: usize) -> PredicateKey {
    PredicateKey {
        name: name.to_string(),
        arity,
    }
}

fn body_term(body: &[Term]) -> Term {
    body.iter()
        .rev()
        .cloned()
        .reduce(|right, left| Term::Compound {
            name: ",".to_string(),
            args: vec![left, right],
        })
        .unwrap_or_else(|| Term::Atom("true".to_string()))
}

fn term_text(term: &Term) -> WamResult<String> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value.clone()),
        _ => Err(WamError::Provider("expected atom or string".into())),
    }
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
