use zlf_core::EntityRef;

use crate::{encode_ordered_micros, EventRecord, GenerationId, IndexDocumentId};

const TIME_PREFIX: &[u8] = b"temporal:v1:event:time:";
const ENTITY_PREFIX: &[u8] = b"temporal:v1:event:entity:";
const GRAPH_ENTITY_PREFIX: &[u8] = b"temporal:v1:event:graph-entity:";

pub(crate) fn time_key(record: &EventRecord) -> Vec<u8> {
    let prefix = time_generation_prefix(&record.generation);
    let mut key = time_seek_key(&prefix, record.at_micros);
    push_part(&mut key, record.id.0.as_bytes());
    key
}

pub(crate) fn time_generation_prefix(generation: &GenerationId) -> Vec<u8> {
    let mut key = TIME_PREFIX.to_vec();
    push_part(&mut key, generation.0.as_bytes());
    key
}

pub(crate) fn time_seek_key(prefix: &[u8], instant: i64) -> Vec<u8> {
    let mut key = prefix.to_vec();
    key.extend_from_slice(&encode_ordered_micros(instant));
    key
}

pub(crate) fn entity_key(record: &EventRecord) -> Vec<u8> {
    let mut key = entity_document_prefix(&record.generation, &record.document_id);
    push_part(&mut key, record.id.0.as_bytes());
    key
}

pub(crate) fn entity_document_prefix(
    generation: &GenerationId,
    document_id: &IndexDocumentId,
) -> Vec<u8> {
    let mut key = ENTITY_PREFIX.to_vec();
    push_part(&mut key, generation.0.as_bytes());
    push_part(&mut key, &document_id.canonical_bytes());
    key
}

pub(crate) fn graph_entity_key(record: &EventRecord) -> Vec<u8> {
    let mut key = graph_entity_prefix(&record.generation, &record.document_id.entity);
    push_part(&mut key, record.id.0.as_bytes());
    key
}

pub(crate) fn graph_entity_prefix(generation: &GenerationId, entity: &EntityRef) -> Vec<u8> {
    let mut key = GRAPH_ENTITY_PREFIX.to_vec();
    push_part(&mut key, generation.0.as_bytes());
    key.push(u8::from(matches!(entity, EntityRef::Edge(_))));
    push_part(&mut key, entity.id().as_bytes());
    key
}

fn push_part(target: &mut Vec<u8>, part: &[u8]) {
    target.extend_from_slice(&(part.len() as u32).to_be_bytes());
    target.extend_from_slice(part);
}
