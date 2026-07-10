use zlf_core::{Result, ZlfError};
use zlf_prolog::Term;

use super::{lock_error, ZlfDatabase};

impl ZlfDatabase {
    pub(super) fn clear_tables(&self) -> Result<()> {
        self.table_manager
            .invalidate_all()
            .map_err(|error| ZlfError::Internal(error.to_string()))
    }

    pub(super) fn apply_directive(&self, directive: &Term) -> Result<()> {
        let Term::Compound { name, args } = directive else {
            return Ok(());
        };
        if name != "table" || args.len() != 1 {
            return Ok(());
        }
        let Some(key) = predicate_indicator(&args[0]) else {
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

pub(super) fn contains_mutation(term: &Term) -> bool {
    match term {
        Term::Compound { name, args } => {
            matches!(
                name.as_str(),
                "asserta" | "assertz" | "retract" | "retractall"
            ) || args.iter().any(contains_mutation)
        }
        Term::List(items) => items.iter().any(contains_mutation),
        Term::Object(entries) => entries.iter().any(|(_, value)| contains_mutation(value)),
        _ => false,
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
