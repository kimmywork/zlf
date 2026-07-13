use zlf_core::{Result, ZlfError};
use zlf_prolog::Term;

use super::{lock_error, ZlfDatabase};

impl ZlfDatabase {
    pub fn table_metrics(&self) -> zlf_prolog::wam::TableMetricsSnapshot {
        self.table_manager.metrics()
    }

    pub(super) fn refresh_after_mutation(&self, terms: &[Term]) -> Result<()> {
        self.reload_rules()?;
        self.refresh_registry()?;
        let facts = mutation_fact_keys(terms);
        let predicates = mutation_predicates(terms);
        if !facts.is_empty() {
            self.table_manager
                .invalidate_facts(&facts)
                .map_err(|error| ZlfError::Internal(error.to_string()))?;
        }
        if !predicates.is_empty() {
            self.invalidate_predicates(&predicates)?;
        }
        if facts.is_empty() && predicates.is_empty() {
            self.clear_tables()?;
        }
        Ok(())
    }

    pub(super) fn invalidate_fact(&self, fact: &Term) -> Result<()> {
        if let Some(key) = zlf_prolog::wam::term_to_fact_key(fact) {
            self.table_manager
                .invalidate_facts(&[key])
                .map_err(|error| ZlfError::Internal(error.to_string()))?;
        }
        self.invalidate_predicates(&fact_predicates(fact))
    }

    pub(super) fn invalidate_node(&self, node: &zlf_core::Node) -> Result<()> {
        let mut predicates = vec![key("node", 1), key("label", 2), key("property", 3)];
        predicates.extend(node.labels.iter().map(|label| key(label, 1)));
        predicates.extend(
            node.properties
                .keys()
                .map(|property| key(&format!("prop_{property}"), 2)),
        );
        self.invalidate_predicates(&predicates)
    }

    pub(super) fn invalidate_edge(&self, edge_type: &str) -> Result<()> {
        self.invalidate_predicates(&[key("edge", 3), key(edge_type, 2)])
    }

    pub(super) fn invalidate_predicates(
        &self,
        predicates: &[zlf_prolog::wam::PredicateKey],
    ) -> Result<()> {
        self.table_manager
            .invalidate_predicates(predicates)
            .map(|_| ())
            .map_err(|error| ZlfError::Internal(error.to_string()))
    }

    pub(super) fn clear_tables(&self) -> Result<()> {
        self.table_manager
            .invalidate_all()
            .map_err(|error| ZlfError::Internal(error.to_string()))
    }

    pub(super) fn apply_directive(&self, directive: &Term) -> Result<()> {
        let Term::Compound { name, args } = directive else {
            return Ok(());
        };
        match (name.as_str(), args.as_slice()) {
            ("table", [indicator]) => self.apply_table_directive(indicator),
            ("index_profile", [profile_name, version, config]) => {
                let profile =
                    crate::profile_store::lower_profile_directive(profile_name, version, config)?;
                self.put_index_profile(&profile).map(|_| ())
            }
            ("activate_index_profile", [profile_name, Term::Integer(version)]) => {
                let name = directive_text(profile_name)?;
                let version = u32::try_from(*version)
                    .map_err(|_| ZlfError::Internal("invalid profile version".into()))?;
                self.activate_index_profile(name, version).map(|_| ())
            }
            _ => Ok(()),
        }
    }

    fn apply_table_directive(&self, indicator: &Term) -> Result<()> {
        let Some(key) = predicate_indicator(indicator) else {
            return Err(ZlfError::Internal(
                "invalid table predicate indicator".to_string(),
            ));
        };
        self.storage.put_raw(
            &format!("table:declaration:{}/{}", key.name, key.arity),
            &bincode::serialize(&key)
                .map_err(|error| ZlfError::Serialization(error.to_string()))?,
        )?;
        self.tabled.write().map_err(lock_error)?.insert(key);
        Ok(())
    }
}

pub(super) fn load_declarations(
    storage: &zlf_storage::Storage,
) -> Result<std::collections::HashSet<zlf_prolog::wam::PredicateKey>> {
    storage
        .scan_prefix("table:declaration:")?
        .into_iter()
        .map(|(_, bytes)| {
            bincode::deserialize(&bytes).map_err(|error| ZlfError::Serialization(error.to_string()))
        })
        .collect()
}

pub(super) fn mutation_fact_keys(terms: &[Term]) -> Vec<zlf_prolog::wam::FactKey> {
    let mut facts = Vec::new();
    for term in terms {
        collect_mutation_facts(term, &mut facts);
    }
    facts
}

fn collect_mutation_facts(term: &Term, facts: &mut Vec<zlf_prolog::wam::FactKey>) {
    let Term::Compound { name, args } = term else {
        return;
    };
    if name == "retract" {
        if let Some(fact) = args.first().and_then(selective_retract_fact) {
            facts.push(fact);
        }
    } else if is_property_mutation(name) {
        if let (Some(entity), Some(key)) = (
            args.first().and_then(term_text),
            args.get(1).and_then(term_text),
        ) {
            facts.push(zlf_prolog::wam::FactKey::Property {
                entity: entity.to_string(),
                key: key.to_string(),
            });
        }
    }
    args.iter()
        .for_each(|argument| collect_mutation_facts(argument, facts));
}

fn selective_retract_fact(term: &Term) -> Option<zlf_prolog::wam::FactKey> {
    let fact = zlf_prolog::wam::term_to_fact_key(term)?;
    (!matches!(fact, zlf_prolog::wam::FactKey::Node { .. })).then_some(fact)
}

pub(super) fn mutation_predicates(terms: &[Term]) -> Vec<zlf_prolog::wam::PredicateKey> {
    let mut predicates = Vec::new();
    for term in terms {
        collect_mutation_predicates(term, &mut predicates);
    }
    predicates.sort_by(|left, right| (&left.name, left.arity).cmp(&(&right.name, right.arity)));
    predicates.dedup();
    predicates
}

fn collect_mutation_predicates(term: &Term, predicates: &mut Vec<zlf_prolog::wam::PredicateKey>) {
    let Term::Compound { name, args } = term else {
        return;
    };
    if matches!(
        name.as_str(),
        "asserta" | "assertz" | "retract" | "retractall"
    ) {
        if let Some(mutated) = args.first() {
            if name == "retract" && zlf_prolog::wam::term_to_fact_key(mutated).is_some() {
                return;
            }
            let head = match mutated {
                Term::Compound { name, args } if name == ":-" => args.first().unwrap_or(mutated),
                _ => mutated,
            };
            predicates.extend(fact_predicates(head));
        }
    }
    if is_property_mutation(name) {
        predicates.push(key("property", 3));
        if let Some(property) = args.get(1).and_then(term_text) {
            predicates.push(key(&format!("prop_{property}"), 2));
        }
    }
    args.iter()
        .for_each(|argument| collect_mutation_predicates(argument, predicates));
}

pub(super) fn contains_mutation(term: &Term) -> bool {
    match term {
        Term::Compound { name, args } => {
            matches!(
                name.as_str(),
                "asserta" | "assertz" | "retract" | "retractall"
            ) || is_property_mutation(name)
                || args.iter().any(contains_mutation)
        }
        Term::List(items) => items.iter().any(contains_mutation),
        Term::Object(entries) => entries.iter().any(|(_, value)| contains_mutation(value)),
        _ => false,
    }
}

fn is_property_mutation(name: &str) -> bool {
    matches!(
        name,
        "set_node_property" | "remove_node_property" | "set_edge_property" | "remove_edge_property"
    )
}

fn term_text(term: &Term) -> Option<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Some(value),
        _ => None,
    }
}

fn fact_predicates(term: &Term) -> Vec<zlf_prolog::wam::PredicateKey> {
    let Term::Compound { name, args } = term else {
        return Vec::new();
    };
    let mut predicates = vec![key(name, args.len())];
    match (name.as_str(), args.as_slice()) {
        ("node", _) => {
            predicates.extend([key("node", 1), key("label", 2), key("property", 3)]);
            if let Some(Term::List(labels)) = args.get(args.len().saturating_sub(2)) {
                predicates.extend(labels.iter().filter_map(|label| match label {
                    Term::Atom(label) => Some(key(label, 1)),
                    _ => None,
                }));
            }
            if let Some(Term::Object(properties)) = args.last() {
                predicates.extend(
                    properties
                        .iter()
                        .map(|(property, _)| key(&format!("prop_{property}"), 2)),
                );
            }
        }
        ("edge", [_, Term::Atom(edge_type), _, ..]) => predicates.push(key(edge_type, 2)),
        ("property", [_, Term::Atom(property), _]) => {
            predicates.push(key(&format!("prop_{property}"), 2));
        }
        _ => {}
    }
    predicates
}

fn key(name: &str, arity: usize) -> zlf_prolog::wam::PredicateKey {
    zlf_prolog::wam::PredicateKey {
        name: name.to_string(),
        arity,
    }
}

fn directive_text(term: &Term) -> Result<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value),
        _ => Err(ZlfError::Internal("profile name must be text".into())),
    }
}

fn predicate_indicator(term: &Term) -> Option<zlf_prolog::wam::PredicateKey> {
    let Term::Compound { name, args } = term else {
        return None;
    };
    if name != "/" || args.len() != 2 {
        return None;
    }
    let (Term::Atom(name), Term::Integer(arity)) = (&args[0], &args[1]) else {
        return None;
    };
    Some(zlf_prolog::wam::PredicateKey {
        name: name.clone(),
        arity: *arity as usize,
    })
}
