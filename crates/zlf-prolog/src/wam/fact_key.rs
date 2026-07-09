use crate::parser::Term;

use super::predicate::PredicateKey;

/// Canonical identity for every durable fact-like write.
/// Maps one-to-one to RocksDB records and index entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FactKey {
    Node {
        id: String,
    },
    Label {
        node: String,
        label: String,
    },
    Property {
        entity: String,
        key: String,
    },
    Edge {
        source: String,
        edge_type: String,
        target: String,
    },
    Rule {
        predicate: String,
        arity: usize,
        source_hash: String,
    },
}

/// Internal mutation event for future incremental tabling / dependency tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationEvent {
    FactInserted(FactKey),
    FactDeleted(FactKey),
    FactUpdated(FactKey),
    RuleInserted(PredicateKey, String),
    RuleDeleted(PredicateKey, String),
}

/// Parse a Prolog fact term into its canonical FactKey.
/// Returns None if the term is not a recognized storage-backed fact form.
#[allow(clippy::too_many_lines)]
pub fn term_to_fact_key(term: &Term) -> Option<FactKey> {
    let (name, args) = compound(term)?;
    let a = |t: &Term| atom(t).map(str::to_string);
    match (name, args) {
        ("node", [id]) => Some(FactKey::Node { id: a(id)? }),
        ("node", [id, _]) => Some(FactKey::Node { id: a(id)? }),
        ("node", [id, _, _]) => Some(FactKey::Node { id: a(id)? }),
        ("label", [id, label]) => Some(FactKey::Label {
            node: a(id)?,
            label: a(label)?,
        }),
        ("property", [id, key, _]) => Some(FactKey::Property {
            entity: a(id)?,
            key: a(key)?,
        }),
        ("edge", [source, edge_type, target]) => Some(FactKey::Edge {
            source: a(source)?,
            edge_type: a(edge_type)?,
            target: a(target)?,
        }),
        ("edge", [source, edge_type, target, _]) => Some(FactKey::Edge {
            source: a(source)?,
            edge_type: a(edge_type)?,
            target: a(target)?,
        }),
        (name, [id]) => {
            // Label shortcut: Label(Id)
            Some(FactKey::Label {
                node: a(id)?,
                label: name.to_string(),
            })
        }
        (name, [id, _]) if name.starts_with("prop_") => {
            // Property shortcut: prop_Key(Id, Value)
            Some(FactKey::Property {
                entity: a(id)?,
                key: name.trim_start_matches("prop_").to_string(),
            })
        }
        (name, [source, target]) => {
            // Edge type shortcut: EdgeType(Source, Target)
            Some(FactKey::Edge {
                source: a(source)?,
                edge_type: name.to_string(),
                target: a(target)?,
            })
        }
        (name, [source, target, _]) => {
            // Edge type shortcut with props: EdgeType(Source, Target, Props)
            Some(FactKey::Edge {
                source: a(source)?,
                edge_type: name.to_string(),
                target: a(target)?,
            })
        }
        _ => None,
    }
}

/// Parse a retract argument term into a DeletePattern that can match
/// multiple facts.  e.g. `retract(prop_name(alice, _))` should delete
/// any property value for (alice, name).
///
/// Returns None if the term cannot be interpreted as a deletion pattern.
#[allow(clippy::too_many_lines)]
pub fn term_to_delete_pattern(term: &Term) -> Option<DeletePattern> {
    let (name, args) = compound(term)?;
    let is_var_or_wildcard =
        |t: &Term| matches!(t, Term::Variable(n) if n == "_" || n.starts_with('_'));
    match (name, args) {
        ("node", [id]) => {
            let id = atom(id)?;
            Some(DeletePattern::Node { id: id.to_string() })
        }
        ("label", [id, label]) => {
            let id = atom(id)?;
            let label = atom(label)?;
            Some(DeletePattern::Label {
                node: id.to_string(),
                label: label.to_string(),
            })
        }
        ("property", [id, key, _]) => {
            let id = atom(id)?;
            let key = atom(key)?;
            Some(DeletePattern::Property {
                entity: id.to_string(),
                key: key.to_string(),
            })
        }
        ("edge", [source, edge_type, target]) => {
            let source = atom(source)?;
            let edge_type = atom(edge_type)?;
            let target = atom(target)?;
            Some(DeletePattern::Edge {
                source: source.to_string(),
                edge_type: edge_type.to_string(),
                target: target.to_string(),
            })
        }
        (name, [id]) => {
            // Label shortcut: Label(Id) -> delete label
            let id = atom(id)?;
            Some(DeletePattern::Label {
                node: id.to_string(),
                label: name.to_string(),
            })
        }
        (name, [id, _]) if name.starts_with("prop_") => {
            // Property shortcut: prop_Key(Id, _) -> delete property
            let id = atom(id)?;
            Some(DeletePattern::Property {
                entity: id.to_string(),
                key: name.trim_start_matches("prop_").to_string(),
            })
        }
        (name, [source, target]) if is_var_or_wildcard(target) => {
            // Edge type shortcut with wildcard target: EdgeType(Source, _) -> delete all edges of type from source
            let source = atom(source)?;
            Some(DeletePattern::EdgeTypeFromSource {
                edge_type: name.to_string(),
                source: source.to_string(),
            })
        }
        (name, [source, target]) => {
            // Edge type shortcut: EdgeType(Source, Target) -> delete edge
            let source = atom(source)?;
            let target = atom(target)?;
            Some(DeletePattern::Edge {
                source: source.to_string(),
                edge_type: name.to_string(),
                target: target.to_string(),
            })
        }
        _ => None,
    }
}

/// Describes which facts to delete.  Supports wildcards via variable arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeletePattern {
    Node {
        id: String,
    },
    Label {
        node: String,
        label: String,
    },
    Property {
        entity: String,
        key: String,
    },
    Edge {
        source: String,
        edge_type: String,
        target: String,
    },
    EdgeTypeFromSource {
        edge_type: String,
        source: String,
    },
}

fn compound(term: &Term) -> Option<(&str, &[Term])> {
    match term {
        Term::Compound { name, args } => Some((name, args)),
        Term::Atom(name) => Some((name, &[])),
        _ => None,
    }
}

fn atom(term: &Term) -> Option<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Some(value),
        _ => None,
    }
}
