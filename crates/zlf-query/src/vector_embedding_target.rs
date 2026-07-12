use chrono::Utc;
use zlf_core::{EntityRef, Result};
use zlf_index::{
    DocumentChanges, DocumentManifest, EmbeddingJob, EmbeddingJobState, EmbeddingModelProfile,
    ExactVectorStore, GenerationId, VectorKey, EMBEDDING_JOB_SCHEMA_VERSION,
};
use zlf_storage::{MutationEvent, MutationKind, Storage};

use crate::coordinator::{IndexTarget, TargetApplyError};
use crate::fake_documents::{active_profiles, matching_fields, profile_documents};
use crate::{EmbeddingJobStore, IndexManifestStore};

pub struct VectorEmbeddingTarget<'a> {
    exact: &'a ExactVectorStore,
    generation: GenerationId,
    model: EmbeddingModelProfile,
    manifest_scope: String,
}

impl<'a> VectorEmbeddingTarget<'a> {
    pub fn new(
        exact: &'a ExactVectorStore,
        generation: GenerationId,
        model: EmbeddingModelProfile,
    ) -> Result<Self> {
        model
            .validate_dense_v1()
            .map_err(zlf_core::ZlfError::Internal)?;
        let manifest_scope = format!("vector:{}:{}:{}", generation.0, model.id, model.version);
        Ok(Self {
            exact,
            generation,
            model,
            manifest_scope,
        })
    }

    pub fn manifest_scope(&self) -> &str {
        &self.manifest_scope
    }

    pub fn rebuild(&self, storage: &Storage) -> Result<()> {
        for node in storage.get_all_nodes()? {
            let entity = EntityRef::Node(node.id);
            let version = source_version(storage, &entity)?;
            self.apply_entity(storage, &entity, version, false)?;
        }
        for edge in storage.get_all_edges()? {
            let entity = EntityRef::Edge(edge.id);
            let version = source_version(storage, &entity)?;
            self.apply_entity(storage, &entity, version, false)?;
        }
        Ok(())
    }

    fn apply_entity(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
        delete: bool,
    ) -> Result<()> {
        if delete {
            return self.delete_entity(storage, entity, source_version);
        }
        for profile in active_profiles(storage)? {
            self.apply_profile(storage, entity, source_version, profile)?;
        }
        Ok(())
    }

    fn apply_profile(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
        profile: zlf_index::IndexProfileArtifact,
    ) -> Result<()> {
        let Some(fields) = matching_fields(storage, entity, &profile)? else {
            return Ok(());
        };
        let mut documents = profile_documents(entity, source_version, &profile, &fields)?;
        documents.retain(|document| {
            profile
                .fields
                .get(&document.id.field)
                .and_then(|options| options.vector.as_ref())
                .is_some_and(|vector| vector.model_profile == self.model.id)
        });
        let store = IndexManifestStore::new(storage, &self.manifest_scope);
        self.retire_versions(
            &store,
            storage,
            entity,
            source_version,
            &profile.name,
            profile.version,
        )?;
        let manifest = DocumentManifest {
            entity: entity.clone(),
            profile_name: profile.name,
            profile_version: profile.version,
            source_version,
            documents,
        };
        self.apply_changes(storage, store.changes(&manifest)?)?;
        store.save(&manifest)
    }

    fn retire_versions(
        &self,
        store: &IndexManifestStore<'_>,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
        name: &str,
        version: u32,
    ) -> Result<()> {
        for previous in store
            .list_for_entity(entity)?
            .into_iter()
            .filter(|manifest| manifest.profile_name == name && manifest.profile_version != version)
        {
            let empty = DocumentManifest {
                entity: entity.clone(),
                profile_name: previous.profile_name,
                profile_version: previous.profile_version,
                source_version,
                documents: Vec::new(),
            };
            self.apply_changes(storage, store.changes(&empty)?)?;
            store.save(&empty)?;
        }
        Ok(())
    }

    fn delete_entity(
        &self,
        storage: &Storage,
        entity: &EntityRef,
        source_version: u64,
    ) -> Result<()> {
        let store = IndexManifestStore::new(storage, &self.manifest_scope);
        for previous in store.list_for_entity(entity)? {
            let empty = DocumentManifest {
                entity: entity.clone(),
                profile_name: previous.profile_name,
                profile_version: previous.profile_version,
                source_version,
                documents: Vec::new(),
            };
            self.apply_changes(storage, store.changes(&empty)?)?;
            store.save(&empty)?;
        }
        Ok(())
    }

    fn apply_changes(&self, storage: &Storage, changes: DocumentChanges) -> Result<()> {
        let mut deletes = changes
            .deletes
            .into_iter()
            .map(|document_id| self.vector_key(document_id))
            .collect::<Vec<_>>();
        deletes.extend(
            changes
                .upserts
                .iter()
                .map(|document| self.vector_key(document.id.clone())),
        );
        self.exact.apply(&[], &deletes, &self.model)?;
        let jobs = EmbeddingJobStore::new(storage);
        for document in changes.upserts {
            jobs.enqueue(self.embedding_job(document))?;
        }
        Ok(())
    }

    fn embedding_job(&self, document: zlf_index::IndexDocument) -> EmbeddingJob {
        EmbeddingJob {
            schema_version: EMBEDDING_JOB_SCHEMA_VERSION,
            generation: self.generation.clone(),
            document_id: document.id,
            source_version: document.source_version,
            content_fingerprint: document.content_fingerprint,
            model_profile: self.model.id.clone(),
            model_version: self.model.version,
            expected_dimension: self.model.dimension,
            attempts: 0,
            state: EmbeddingJobState::Pending,
            created_at: Utc::now(),
            lease_until: None,
            retry_at: None,
            completed_at: None,
            last_error_class: None,
        }
    }

    fn vector_key(&self, document_id: zlf_index::IndexDocumentId) -> VectorKey {
        VectorKey {
            generation: self.generation.clone(),
            model_profile: self.model.id.clone(),
            model_version: self.model.version,
            document_id,
        }
    }
}

impl IndexTarget for VectorEmbeddingTarget<'_> {
    fn apply(
        &self,
        storage: &Storage,
        event: &MutationEvent,
    ) -> std::result::Result<(), TargetApplyError> {
        let result = match &event.entity {
            Some(entity) => self.apply_entity(
                storage,
                entity,
                event.source_version,
                matches!(event.kind, MutationKind::Delete),
            ),
            None => self.rebuild(storage),
        };
        result.map_err(|error| TargetApplyError {
            message: error.to_string(),
            retryable: true,
        })
    }
}

fn source_version(storage: &Storage, entity: &EntityRef) -> Result<u64> {
    Ok(storage
        .get_entity_state(entity)?
        .map_or(0, |state| state.source_version))
}
