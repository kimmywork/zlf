use std::collections::HashSet;

use serde::Serialize;
use zlf_core::Result;
use zlf_prolog::wam::{predicate_key, PredicateKey, PredicateKind};
use zlf_prolog::{PrologParser, Query, Term};

use super::{lock_error, ZlfDatabase};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ArgumentMode {
    Bound,
    Free,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum AccessPath {
    WamProgram,
    Builtin,
    ExactNode,
    NodeScan,
    LabelIndex,
    ExactProperty,
    EntityProperty,
    PropertyScan,
    ExactEdge,
    OutgoingEdges,
    IncomingEdges,
    EdgeTypeScan,
    TemporalEventRange,
    ValidityInterval,
    ExternalIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedGoal {
    pub predicate: PredicateKey,
    pub modes: Vec<ArgumentMode>,
    pub access: AccessPath,
    pub pushed_down: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct QueryPlan {
    pub goals: Vec<PlannedGoal>,
}

impl ZlfDatabase {
    pub fn explain_prolog(&self, source: &str) -> Result<QueryPlan> {
        let terms = match PrologParser::parse_query(source)? {
            Query::Goal(term) => vec![term],
            Query::Goals(terms) => terms,
            Query::RuleDef(_) | Query::Directive(_) => Vec::new(),
        };
        let registry = self.registry.read().map_err(lock_error)?;
        let mut bound = HashSet::new();
        let mut goals = Vec::new();
        for term in terms {
            let Some(predicate) = predicate_key(&term) else {
                continue;
            };
            let args = compound_args(&term);
            let modes = args
                .iter()
                .map(|argument| argument_mode(argument, &bound))
                .collect::<Vec<_>>();
            let kind = registry.kind_for(&predicate);
            let access = access_path(&predicate, &modes, kind);
            collect_variables(&term, &mut bound);
            goals.push(PlannedGoal {
                predicate,
                modes,
                pushed_down: !matches!(access, AccessPath::WamProgram | AccessPath::Builtin),
                access,
            });
        }
        Ok(QueryPlan { goals })
    }
}

fn access_path(
    predicate: &PredicateKey,
    modes: &[ArgumentMode],
    kind: Option<&PredicateKind>,
) -> AccessPath {
    match (predicate.name.as_str(), modes) {
        ("node", [ArgumentMode::Bound]) => AccessPath::ExactNode,
        ("node", [ArgumentMode::Free]) => AccessPath::NodeScan,
        ("label", [_, ArgumentMode::Bound]) => AccessPath::LabelIndex,
        ("property", [ArgumentMode::Bound, _, _]) => AccessPath::EntityProperty,
        ("edge", [ArgumentMode::Bound, ArgumentMode::Bound, ArgumentMode::Bound]) => {
            AccessPath::ExactEdge
        }
        ("edge", [ArgumentMode::Bound, ArgumentMode::Bound, _]) => AccessPath::OutgoingEdges,
        ("edge", [_, ArgumentMode::Bound, ArgumentMode::Bound]) => AccessPath::IncomingEdges,
        ("temporal_on" | "temporal_between", _) => AccessPath::TemporalEventRange,
        ("valid_at" | "valid_overlaps", _) => AccessPath::ValidityInterval,
        _ => shortcut_access(modes, kind),
    }
}

fn shortcut_access(modes: &[ArgumentMode], kind: Option<&PredicateKind>) -> AccessPath {
    match (kind, modes) {
        (Some(PredicateKind::BuiltinCore), _) => AccessPath::Builtin,
        (Some(PredicateKind::IndexProvider | PredicateKind::GraphAlgorithm), _) => {
            AccessPath::ExternalIndex
        }
        (Some(PredicateKind::LabelShortcut), _) => AccessPath::LabelIndex,
        (Some(PredicateKind::PropertyShortcut), [_, ArgumentMode::Bound]) => {
            AccessPath::ExactProperty
        }
        (Some(PredicateKind::PropertyShortcut), [ArgumentMode::Bound, _]) => {
            AccessPath::EntityProperty
        }
        (Some(PredicateKind::PropertyShortcut), _) => AccessPath::PropertyScan,
        (Some(PredicateKind::EdgeShortcut), [ArgumentMode::Bound, ArgumentMode::Bound]) => {
            AccessPath::ExactEdge
        }
        (Some(PredicateKind::EdgeShortcut), [ArgumentMode::Bound, _]) => AccessPath::OutgoingEdges,
        (Some(PredicateKind::EdgeShortcut), [_, ArgumentMode::Bound]) => AccessPath::IncomingEdges,
        (Some(PredicateKind::EdgeShortcut), _) => AccessPath::EdgeTypeScan,
        _ => AccessPath::WamProgram,
    }
}

fn compound_args(term: &Term) -> &[Term] {
    match term {
        Term::Compound { args, .. } => args,
        _ => &[],
    }
}

fn argument_mode(term: &Term, bound: &HashSet<String>) -> ArgumentMode {
    match term {
        Term::Variable(name) if !bound.contains(name) => ArgumentMode::Free,
        _ => ArgumentMode::Bound,
    }
}

fn collect_variables(term: &Term, bound: &mut HashSet<String>) {
    match term {
        Term::Variable(name) if name != "_" => {
            bound.insert(name.clone());
        }
        Term::Compound { args, .. } | Term::List(args) => {
            args.iter()
                .for_each(|argument| collect_variables(argument, bound));
        }
        Term::Object(entries) => entries
            .iter()
            .for_each(|(_, value)| collect_variables(value, bound)),
        _ => {}
    }
}
