use zlf_core::{Result, ZlfError};
use zlf_prolog::wam::{
    builtin_predicates, graph_algorithm_predicates, graph_view_predicates, index_predicates,
    CompiledRuleArtifact, PredicateKind, PredicateRegistry,
};
use zlf_storage::Storage;

/// Populate a PredicateRegistry by scanning storage for labels, edge types,
/// and property keys, and registering builtin and index predicates.
#[allow(clippy::too_many_lines)]
pub fn populate_registry(
    storage: &Storage,
    rules: &[CompiledRuleArtifact],
    registry: &mut PredicateRegistry,
) -> Result<()> {
    // Register builtin predicates
    for (key, _) in builtin_predicates() {
        registry.register(key, PredicateKind::BuiltinCore);
    }
    // Register graph view predicates
    for key in graph_view_predicates() {
        registry.register(key, PredicateKind::StorageProvider);
    }
    // Register graph algorithm predicates
    for key in graph_algorithm_predicates() {
        registry.register(key, PredicateKind::GraphAlgorithm);
    }
    // Register index predicates
    for key in index_predicates() {
        registry.register(key, PredicateKind::IndexProvider);
    }
    // Register user rules
    for artifact in rules {
        registry.register(artifact.key.clone(), PredicateKind::UserRule);
    }
    // Discover shortcuts from compact metadata when available.
    let labels = metadata_values(storage, "label")?;
    let properties = metadata_values(storage, "property")?;
    let edge_types = metadata_values(storage, "edge_type")?;
    if !labels.is_empty() || !properties.is_empty() || !edge_types.is_empty() {
        registry.sync_label_shortcuts(&labels);
        registry.sync_edge_shortcuts(&edge_types);
        registry.sync_property_shortcuts(&properties);
        return Ok(());
    }
    populate_registry_legacy(storage, registry)
}

fn populate_registry_legacy(storage: &Storage, registry: &mut PredicateRegistry) -> Result<()> {
    let nodes = storage
        .get_all_nodes()
        .map_err(|e| ZlfError::Internal(e.to_string()))?;
    let mut all_labels = Vec::new();
    let mut all_prop_keys = Vec::new();
    for node in &nodes {
        for label in &node.labels {
            if !all_labels.contains(label) {
                all_labels.push(label.clone());
            }
        }
        for key in node.properties.keys() {
            if !all_prop_keys.contains(key) {
                all_prop_keys.push(key.clone());
            }
        }
    }
    let edges = storage
        .get_all_edges()
        .map_err(|e| ZlfError::Internal(e.to_string()))?;
    let mut all_edge_types = Vec::new();
    for edge in &edges {
        if !all_edge_types.contains(&edge.edge_type) {
            all_edge_types.push(edge.edge_type.clone());
        }
    }
    registry.sync_label_shortcuts(&all_labels);
    registry.sync_edge_shortcuts(&all_edge_types);
    registry.sync_property_shortcuts(&all_prop_keys);
    Ok(())
}

fn metadata_values(storage: &Storage, kind: &str) -> Result<Vec<String>> {
    storage
        .scan_prefix(&format!("meta:predicate:{kind}:"))?
        .into_iter()
        .map(|(_, value)| {
            String::from_utf8(value).map_err(|error| ZlfError::Serialization(error.to_string()))
        })
        .collect()
}
