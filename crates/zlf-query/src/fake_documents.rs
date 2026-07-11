use std::collections::{BTreeSet, HashMap};

use zlf_core::{EntityRef, Result, Value};
use zlf_index::{
    chunk_text, ChunkingProfile, DocumentManifest, EntityMatcher, IndexDocument,
    IndexProfileArtifact, INDEX_DOCUMENT_SCHEMA_VERSION,
};
use zlf_storage::{MutationEvent, MutationKind, Storage};

use crate::{IndexManifestStore, IndexProfileStore};

pub(crate) fn apply_fake_documents(
    storage: &Storage,
    target: &str,
    event: &MutationEvent,
) -> Result<()> {
    let Some(entity) = &event.entity else {
        return Ok(());
    };
    if matches!(event.kind, MutationKind::Delete) {
        return delete_entity_documents(storage, target, entity, event.source_version);
    }
    let profiles = active_profiles(storage)?;
    for profile in profiles {
        if let Some(fields) = matching_fields(storage, entity, &profile)? {
            reconcile_profile(
                storage,
                target,
                entity,
                event.source_version,
                &profile,
                fields,
            )?;
        }
    }
    Ok(())
}

pub(crate) fn fake_documents(storage: &Storage, target: &str) -> Result<Vec<IndexDocument>> {
    storage
        .scan_prefix(&document_prefix(target))?
        .into_iter()
        .map(|(_, bytes)| {
            bincode::deserialize(&bytes)
                .map_err(|error| zlf_core::ZlfError::Serialization(error.to_string()))
        })
        .collect()
}

fn active_profiles(storage: &Storage) -> Result<Vec<IndexProfileArtifact>> {
    let store = IndexProfileStore::new(storage);
    let names = store
        .list()?
        .into_iter()
        .map(|profile| profile.name)
        .collect::<BTreeSet<_>>();
    Ok(names
        .into_iter()
        .map(|name| store.active(&name))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect())
}

fn matching_fields(
    storage: &Storage,
    entity: &EntityRef,
    profile: &IndexProfileArtifact,
) -> Result<Option<HashMap<String, Value>>> {
    match (entity, &profile.matcher) {
        (EntityRef::Node(id), EntityMatcher::NodeLabels { labels }) => Ok(storage
            .get_node(id)?
            .filter(|node| labels.iter().any(|label| node.labels.contains(label)))
            .map(|node| node.properties)),
        (EntityRef::Edge(id), EntityMatcher::EdgeTypes { edge_types }) => Ok(storage
            .get_edge(id)?
            .filter(|edge| edge_types.contains(&edge.edge_type))
            .map(|edge| edge.properties)),
        _ => Ok(None),
    }
}

fn reconcile_profile(
    storage: &Storage,
    target: &str,
    entity: &EntityRef,
    source_version: u64,
    profile: &IndexProfileArtifact,
    fields: HashMap<String, Value>,
) -> Result<()> {
    let manifest = DocumentManifest {
        entity: entity.clone(),
        profile_name: profile.name.clone(),
        profile_version: profile.version,
        source_version,
        documents: profile_documents(entity, source_version, profile, &fields)?,
    };
    let changes = IndexManifestStore::new(storage).reconcile_and_save(&manifest)?;
    apply_changes(storage, target, changes)
}

fn profile_documents(
    entity: &EntityRef,
    source_version: u64,
    profile: &IndexProfileArtifact,
    fields: &HashMap<String, Value>,
) -> Result<Vec<IndexDocument>> {
    let mut documents = Vec::new();
    for (field, options) in &profile.fields {
        let Some(Value::String(text)) = fields.get(field) else {
            continue;
        };
        let chunking = options
            .vector
            .as_ref()
            .map_or(ChunkingProfile::WholeField { version: 1 }, |vector| {
                vector.chunking.clone()
            });
        for chunk in chunk_text(&chunking, text).map_err(zlf_core::ZlfError::Internal)? {
            documents.push(IndexDocument {
                schema_version: INDEX_DOCUMENT_SCHEMA_VERSION,
                id: zlf_index::IndexDocumentId::new(entity.clone(), field, chunk.chunk_id),
                source_version,
                content_fingerprint: chunk.content_fingerprint,
                source_range: Some(chunk.source_range),
                chunk_ordinal: chunk.ordinal,
                chunk_profile: format!("{chunking:?}"),
                content: chunk.text,
            });
        }
    }
    Ok(documents)
}

fn delete_entity_documents(
    storage: &Storage,
    target: &str,
    entity: &EntityRef,
    source_version: u64,
) -> Result<()> {
    let store = IndexManifestStore::new(storage);
    for old in store.list_for_entity(entity)? {
        let empty = DocumentManifest {
            entity: entity.clone(),
            profile_name: old.profile_name,
            profile_version: old.profile_version,
            source_version,
            documents: Vec::new(),
        };
        let changes = store.reconcile_and_save(&empty)?;
        apply_changes(storage, target, changes)?;
    }
    Ok(())
}

fn apply_changes(
    storage: &Storage,
    target: &str,
    changes: zlf_index::DocumentChanges,
) -> Result<()> {
    for id in changes.deletes {
        storage.delete_raw(&document_key(target, &id))?;
    }
    for document in changes.upserts {
        storage.put_raw(
            &document_key(target, &document.id),
            &bincode::serialize(&document)
                .map_err(|error| zlf_core::ZlfError::Serialization(error.to_string()))?,
        )?;
    }
    Ok(())
}

fn document_prefix(target: &str) -> String {
    format!("projection:fake-document:{}:", hex(target.as_bytes()))
}

fn document_key(target: &str, id: &zlf_index::IndexDocumentId) -> String {
    format!("{}{}", document_prefix(target), hex(&id.canonical_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
