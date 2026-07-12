use zlf_core::{Result, ZlfError};
use zlf_index::{GenerationId, GenerationState};

use crate::generation_manager::{active_key, generation_key, serialized_record, NAMESPACE};
use crate::GenerationManager;

impl GenerationManager<'_> {
    pub fn rollback(&self, target: &str, id: &GenerationId) -> Result<u64> {
        let mut selected = self
            .get(target, id)?
            .ok_or_else(|| ZlfError::Internal("generation not found".into()))?;
        if selected.state != GenerationState::Retired
            || selected.checksum.is_none()
            || selected.validated_at.is_none()
        {
            return Err(ZlfError::Internal(
                "rollback requires a validated retired generation".into(),
            ));
        }
        let mut records = Vec::new();
        if let Some(mut current) = self.active(target)? {
            current.state = GenerationState::Retired;
            records.push(serialized_record(
                generation_key(target, &current.id),
                &current,
            )?);
        }
        selected.state = GenerationState::Active;
        records.push(serialized_record(
            generation_key(target, &selected.id),
            &selected,
        )?);
        records.push((
            active_key(target).into_bytes(),
            selected.id.0.as_bytes().to_vec(),
        ));
        self.storage
            .commit_projection_config(NAMESPACE, &selected.id.0, &records)
    }
}
