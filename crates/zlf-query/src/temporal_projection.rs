use std::collections::HashMap;

use zlf_core::{EntityRef, Result, Value, ZlfError};
use zlf_index::{
    parse_utc_micros, EventRecord, GenerationId, IndexDocumentId, IndexProfileArtifact,
    TemporalRecordId, TemporalRole, ValidityRecord, TEMPORAL_RECORD_SCHEMA_VERSION,
};

use crate::temporal_target::TemporalManifest;

pub(crate) fn project_manifest(
    generation: &GenerationId,
    entity: &EntityRef,
    source_version: u64,
    profile: &IndexProfileArtifact,
    fields: &HashMap<String, Value>,
) -> Result<TemporalManifest> {
    let events = event_records(generation, entity, source_version, profile, fields)?;
    let validities = validity_records(generation, entity, source_version, profile, fields)?;
    Ok(TemporalManifest {
        entity: entity.clone(),
        profile_name: profile.name.clone(),
        profile_version: profile.version,
        source_version,
        events,
        validities,
    })
}

fn event_records(
    generation: &GenerationId,
    entity: &EntityRef,
    source_version: u64,
    profile: &IndexProfileArtifact,
    fields: &HashMap<String, Value>,
) -> Result<Vec<EventRecord>> {
    let mut records = Vec::new();
    for (field, options) in &profile.fields {
        if options.temporal != Some(TemporalRole::Event) {
            continue;
        }
        for (ordinal, value) in temporal_values(fields.get(field))?.into_iter().enumerate() {
            records.push(EventRecord {
                schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
                generation: generation.clone(),
                id: record_id(profile, "event", field, ordinal),
                document_id: document_id(entity, field, ordinal),
                source_version,
                at_micros: parse_utc_micros(&value).map_err(ZlfError::Internal)?,
            });
        }
    }
    Ok(records)
}

fn validity_records(
    generation: &GenerationId,
    entity: &EntityRef,
    source_version: u64,
    profile: &IndexProfileArtifact,
    fields: &HashMap<String, Value>,
) -> Result<Vec<ValidityRecord>> {
    let Some((field, starts)) = validity_values(profile, fields, TemporalRole::ValidFrom)? else {
        return Ok(Vec::new());
    };
    let ends = validity_values(profile, fields, TemporalRole::ValidTo)?
        .map_or_else(Vec::new, |(_, values)| values);
    if !ends.is_empty() && ends.len() != starts.len() {
        return Err(ZlfError::Internal(
            "valid_from and valid_to arrays must have equal lengths".into(),
        ));
    }
    starts
        .into_iter()
        .enumerate()
        .map(|(ordinal, start)| {
            let record = ValidityRecord {
                schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
                generation: generation.clone(),
                id: record_id(profile, "valid", field, ordinal),
                document_id: document_id(entity, field, ordinal),
                source_version,
                valid_from_micros: parse_utc_micros(&start).map_err(ZlfError::Internal)?,
                valid_to_micros: ends
                    .get(ordinal)
                    .map(|end| parse_utc_micros(end).map_err(ZlfError::Internal))
                    .transpose()?,
            };
            record.validate().map_err(ZlfError::Internal)?;
            Ok(record)
        })
        .collect()
}

fn validity_values<'a>(
    profile: &'a IndexProfileArtifact,
    fields: &HashMap<String, Value>,
    role: TemporalRole,
) -> Result<Option<(&'a str, Vec<String>)>> {
    profile
        .fields
        .iter()
        .find(|(_, options)| options.temporal == Some(role))
        .map(|(field, _)| temporal_values(fields.get(field)).map(|values| (field.as_str(), values)))
        .transpose()
}

fn temporal_values(value: Option<&Value>) -> Result<Vec<String>> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(value)) => Ok(vec![value.clone()]),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| match value {
                Value::String(value) => Ok(value.clone()),
                _ => Err(ZlfError::Internal(
                    "temporal arrays must contain strings".into(),
                )),
            })
            .collect(),
        _ => Err(ZlfError::Internal(
            "temporal field must be text or text array".into(),
        )),
    }
}

fn record_id(
    profile: &IndexProfileArtifact,
    kind: &str,
    field: &str,
    ordinal: usize,
) -> TemporalRecordId {
    TemporalRecordId(format!(
        "{}:{}:{kind}:{field}:{ordinal}",
        profile.name, profile.version
    ))
}

fn document_id(entity: &EntityRef, field: &str, ordinal: usize) -> IndexDocumentId {
    IndexDocumentId::new(entity.clone(), field, ordinal.to_string())
}
