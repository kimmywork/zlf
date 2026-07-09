use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::predicate::PredicateKey;

/// Categorizes a predicate by its source of truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredicateKind {
    /// Core builtin (arithmetic, type tests, control, meta-call).
    BuiltinCore,
    /// Backed by RocksDB storage (node/1, label/2, property/3, edge/3).
    StorageProvider,
    /// Backed by BM25/vector/temporal indexes.
    IndexProvider,
    /// Dynamically discovered label shortcut: Label(Id) -> label(Id, Label).
    LabelShortcut,
    /// Dynamically discovered edge shortcut: EdgeType(S, T) -> edge(S, EdgeType, T).
    EdgeShortcut,
    /// Dynamically discovered property shortcut: prop_Key(E, V) -> property(E, Key, V).
    PropertyShortcut,
    /// User-defined compiled rule.
    UserRule,
    /// Graph algorithm builtin (reachable, shortest_path, degree, etc.).
    GraphAlgorithm,
    /// Introspection predicate (predicate/3, rule/3, etc.).
    Introspection,
}

/// Central registry of all known predicates and their kinds.
/// Populated from storage labels, edge types, property keys,
/// compiled rules, index providers, and builtin declarations.
#[derive(Debug, Clone, Default)]
pub struct PredicateRegistry {
    entries: HashMap<PredicateKey, PredicateKind>,
    /// Label shortcuts discovered dynamically.
    label_shortcuts: Vec<String>,
    /// Edge shortcuts discovered dynamically.
    edge_shortcuts: Vec<String>,
    /// Property shortcuts discovered dynamically.
    property_shortcuts: Vec<String>,
}

impl PredicateRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a predicate with its kind.
    pub fn register(&mut self, key: PredicateKey, kind: PredicateKind) {
        self.entries.insert(key, kind);
    }

    /// Register multiple predicates.
    pub fn register_all(&mut self, items: Vec<(PredicateKey, PredicateKind)>) {
        for (key, kind) in items {
            self.entries.insert(key, kind);
        }
    }

    /// Lookup a predicate's kind.
    pub fn kind_for(&self, key: &PredicateKey) -> Option<&PredicateKind> {
        self.entries.get(key)
    }

    /// Check if a predicate is registered.
    pub fn contains(&self, key: &PredicateKey) -> bool {
        self.entries.contains_key(key)
    }

    /// All registered predicates.
    pub fn all(&self) -> &HashMap<PredicateKey, PredicateKind> {
        &self.entries
    }

    /// Predicates matching a given kind.
    pub fn by_kind(&self, kind: &PredicateKind) -> Vec<&PredicateKey> {
        self.entries
            .iter()
            .filter(|(_, k)| *k == kind)
            .map(|(key, _)| key)
            .collect()
    }

    // --- Dynamic shortcut management ---

    pub fn add_label_shortcut(&mut self, label: String) {
        let key = PredicateKey {
            name: label.clone(),
            arity: 1,
        };
        if let std::collections::hash_map::Entry::Vacant(e) = self.entries.entry(key) {
            e.insert(PredicateKind::LabelShortcut);
            self.label_shortcuts.push(label);
        }
    }

    pub fn add_edge_shortcut(&mut self, edge_type: String) {
        let key = PredicateKey {
            name: edge_type.clone(),
            arity: 2,
        };
        if let std::collections::hash_map::Entry::Vacant(e) = self.entries.entry(key) {
            e.insert(PredicateKind::EdgeShortcut);
            self.edge_shortcuts.push(edge_type);
        }
    }

    pub fn add_property_shortcut(&mut self, key_name: String) {
        let key = PredicateKey {
            name: format!("prop_{key_name}"),
            arity: 2,
        };
        if let std::collections::hash_map::Entry::Vacant(e) = self.entries.entry(key) {
            e.insert(PredicateKind::PropertyShortcut);
            self.property_shortcuts.push(key_name);
        }
    }

    /// Synchronize shortcuts from storage state.
    pub fn sync_label_shortcuts(&mut self, active_labels: &[String]) {
        let existing: HashSet<_> = self.label_shortcuts.iter().cloned().collect();
        let active: HashSet<_> = active_labels.iter().cloned().collect();
        for label in active.difference(&existing) {
            self.add_label_shortcut(label.clone());
        }
    }

    pub fn sync_edge_shortcuts(&mut self, active_types: &[String]) {
        let existing: HashSet<_> = self.edge_shortcuts.iter().cloned().collect();
        let active: HashSet<_> = active_types.iter().cloned().collect();
        for et in active.difference(&existing) {
            self.add_edge_shortcut(et.clone());
        }
    }

    pub fn sync_property_shortcuts(&mut self, active_keys: &[String]) {
        let existing: HashSet<_> = self.property_shortcuts.iter().cloned().collect();
        let active: HashSet<_> = active_keys.iter().cloned().collect();
        for key in active.difference(&existing) {
            self.add_property_shortcut(key.clone());
        }
    }
}
