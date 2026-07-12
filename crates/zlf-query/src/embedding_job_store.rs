use chrono::{DateTime, Duration, Utc};
use zlf_core::{Result, ZlfError};
use zlf_index::{EmbeddingJob, EmbeddingJobState};
use zlf_storage::Storage;

const PREFIX: &str = "projection:embedding-job:v1:";

pub struct EmbeddingJobStore<'a> {
    storage: &'a Storage,
}

impl<'a> EmbeddingJobStore<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }

    pub fn enqueue(&self, mut job: EmbeddingJob) -> Result<bool> {
        job.validate().map_err(ZlfError::Internal)?;
        let key = job_key(&job);
        if self
            .storage
            .get_raw(&key)?
            .map(|bytes| deserialize(&bytes))
            .transpose()?
            .is_some_and(|old| same_work(&old, &job))
        {
            return Ok(false);
        }
        job.state = EmbeddingJobState::Pending;
        job.attempts = 0;
        job.lease_until = None;
        job.retry_at = None;
        job.completed_at = None;
        job.last_error_class = None;
        self.save(&job)?;
        Ok(true)
    }

    pub fn claim_ready(
        &self,
        now: DateTime<Utc>,
        limit: usize,
        lease: Duration,
    ) -> Result<Vec<EmbeddingJob>> {
        let mut jobs = self
            .list()?
            .into_iter()
            .filter(|job| ready(job, now))
            .collect::<Vec<_>>();
        jobs.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.document_id.cmp(&right.document_id))
        });
        jobs.truncate(limit);
        for job in &mut jobs {
            job.state = EmbeddingJobState::Leased;
            job.attempts += 1;
            job.lease_until = Some(now + lease);
            self.save(job)?;
        }
        Ok(jobs)
    }

    pub fn complete(&self, job: &EmbeddingJob, now: DateTime<Utc>) -> Result<()> {
        let mut current = self.required(job)?;
        require_leased(&current)?;
        current.state = EmbeddingJobState::Completed;
        current.completed_at = Some(now);
        current.lease_until = None;
        self.save(&current)
    }

    pub fn stale(&self, job: &EmbeddingJob, now: DateTime<Utc>) -> Result<()> {
        let mut current = self.required(job)?;
        require_leased(&current)?;
        current.state = EmbeddingJobState::Stale;
        current.completed_at = Some(now);
        current.lease_until = None;
        self.save(&current)
    }

    pub fn fail(
        &self,
        job: &EmbeddingJob,
        error_class: &str,
        retry_at: DateTime<Utc>,
        max_attempts: u32,
        retryable: bool,
    ) -> Result<()> {
        let mut current = self.required(job)?;
        require_leased(&current)?;
        current.last_error_class = Some(error_class.chars().take(128).collect());
        current.lease_until = None;
        if retryable && current.attempts < max_attempts {
            current.state = EmbeddingJobState::Retry;
            current.retry_at = Some(retry_at);
        } else {
            current.state = EmbeddingJobState::Dead;
            current.retry_at = None;
        }
        self.save(&current)
    }

    pub fn get(&self, job: &EmbeddingJob) -> Result<Option<EmbeddingJob>> {
        self.storage
            .get_raw(&job_key(job))?
            .map(|bytes| deserialize(&bytes))
            .transpose()
    }

    pub fn list(&self) -> Result<Vec<EmbeddingJob>> {
        self.storage
            .scan_prefix(PREFIX)?
            .into_iter()
            .map(|(_, bytes)| deserialize(&bytes))
            .collect()
    }

    fn required(&self, job: &EmbeddingJob) -> Result<EmbeddingJob> {
        self.get(job)?
            .ok_or_else(|| ZlfError::Internal("embedding job not found".into()))
    }

    fn save(&self, job: &EmbeddingJob) -> Result<()> {
        job.validate().map_err(ZlfError::Internal)?;
        self.storage.put_raw(
            &job_key(job),
            &bincode::serialize(job).map_err(serialization)?,
        )
    }
}

fn ready(job: &EmbeddingJob, now: DateTime<Utc>) -> bool {
    match job.state {
        EmbeddingJobState::Pending => true,
        EmbeddingJobState::Retry => job.retry_at.is_none_or(|retry| retry <= now),
        EmbeddingJobState::Leased => job.lease_until.is_some_and(|lease| lease <= now),
        _ => false,
    }
}

fn same_work(left: &EmbeddingJob, right: &EmbeddingJob) -> bool {
    left.source_version == right.source_version
        && left.content_fingerprint == right.content_fingerprint
        && left.expected_dimension == right.expected_dimension
}

fn require_leased(job: &EmbeddingJob) -> Result<()> {
    if job.state != EmbeddingJobState::Leased {
        return Err(ZlfError::Internal("embedding job is not leased".into()));
    }
    Ok(())
}

fn job_key(job: &EmbeddingJob) -> String {
    format!(
        "{PREFIX}{}:{}:{:010}:{}",
        hex(job.generation.0.as_bytes()),
        hex(job.model_profile.as_bytes()),
        job.model_version,
        hex(&job.document_id.canonical_bytes())
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn deserialize(bytes: &[u8]) -> Result<EmbeddingJob> {
    bincode::deserialize(bytes).map_err(serialization)
}

fn serialization(error: impl std::fmt::Display) -> ZlfError {
    ZlfError::Serialization(error.to_string())
}
