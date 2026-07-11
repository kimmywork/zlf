use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use zlf_core::{Result, ZlfError};
use zlf_index::IndexJobMetrics;
use zlf_storage::{MutationEvent, Storage};

use crate::coordinator_store::{list_progress, load_jobs, load_progress, save_job, save_progress};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexJobState {
    Pending,
    Claimed,
    Retryable,
    Completed,
    Stale,
    Dead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableIndexJob {
    pub target: String,
    pub event_sequence: u64,
    pub state: IndexJobState,
    pub attempts: u32,
    pub lease_until: Option<DateTime<Utc>>,
    pub next_attempt: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetProgress {
    pub target: String,
    pub scanned_watermark: u64,
    pub published_watermark: u64,
}

impl TargetProgress {
    pub(crate) fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            scanned_watermark: 0,
            published_watermark: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CoordinatorConfig {
    pub event_batch: usize,
    pub max_attempts: u32,
    pub lease: Duration,
    pub retry_delay: Duration,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            event_batch: 256,
            max_attempts: 8,
            lease: Duration::seconds(30),
            retry_delay: Duration::seconds(1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TargetApplyError {
    pub message: String,
    pub retryable: bool,
}

pub trait IndexTarget {
    fn apply(
        &self,
        storage: &Storage,
        event: &MutationEvent,
    ) -> std::result::Result<(), TargetApplyError>;
}

pub struct IndexCoordinator<'a> {
    storage: &'a Storage,
    config: CoordinatorConfig,
}

impl<'a> IndexCoordinator<'a> {
    pub fn new(storage: &'a Storage, config: CoordinatorConfig) -> Self {
        Self { storage, config }
    }

    pub fn register_target(&self, target: &str) -> Result<TargetProgress> {
        let progress = load_progress(self.storage, target)?;
        save_progress(self.storage, &progress)?;
        Ok(progress)
    }

    pub fn enqueue_available(&self, target: &str) -> Result<usize> {
        let mut progress = load_progress(self.storage, target)?;
        let events = self
            .storage
            .mutation_events_after(progress.scanned_watermark, self.config.event_batch)?;
        for event in &events {
            save_job(
                self.storage,
                &DurableIndexJob {
                    target: target.to_string(),
                    event_sequence: event.sequence,
                    state: IndexJobState::Pending,
                    attempts: 0,
                    lease_until: None,
                    next_attempt: None,
                    last_error: None,
                },
            )?;
            progress.scanned_watermark = event.sequence;
            save_progress(self.storage, &progress)?;
        }
        Ok(events.len())
    }

    pub fn process_next(&self, target_name: &str, target: &dyn IndexTarget) -> Result<bool> {
        let now = Utc::now();
        let Some(mut job) = next_job(load_jobs(self.storage, target_name)?, now) else {
            return Ok(false);
        };
        let event = self
            .storage
            .mutation_events_after(job.event_sequence.saturating_sub(1), 1)?
            .into_iter()
            .find(|event| event.sequence == job.event_sequence)
            .ok_or_else(|| ZlfError::Internal("outbox event missing for durable job".into()))?;
        job.state = IndexJobState::Claimed;
        job.attempts += 1;
        job.lease_until = Some(now + self.config.lease);
        save_job(self.storage, &job)?;
        if self.is_stale(&event)? {
            job.state = IndexJobState::Stale;
            self.finish_job(&mut job)?;
            return Ok(true);
        }
        match target.apply(self.storage, &event) {
            Ok(()) => {
                job.state = IndexJobState::Completed;
                self.finish_job(&mut job)?;
            }
            Err(error) => self.fail_job(&mut job, error, now)?,
        }
        Ok(true)
    }

    pub fn progress(&self, target: &str) -> Result<TargetProgress> {
        load_progress(self.storage, target)
    }

    pub fn jobs(&self, target: &str) -> Result<Vec<DurableIndexJob>> {
        load_jobs(self.storage, target)
    }

    pub fn metrics(&self, target: &str) -> Result<IndexJobMetrics> {
        let jobs = self.jobs(target)?;
        let progress = self.progress(target)?;
        Ok(IndexJobMetrics {
            pending: jobs
                .iter()
                .filter(|job| {
                    matches!(job.state, IndexJobState::Pending | IndexJobState::Retryable)
                })
                .count() as u64,
            claimed: jobs
                .iter()
                .filter(|job| job.state == IndexJobState::Claimed)
                .count() as u64,
            retried: jobs
                .iter()
                .map(|job| job.attempts.saturating_sub(1) as u64)
                .sum(),
            dead: jobs
                .iter()
                .filter(|job| job.state == IndexJobState::Dead)
                .count() as u64,
            stale: jobs
                .iter()
                .filter(|job| job.state == IndexJobState::Stale)
                .count() as u64,
            lag: progress
                .scanned_watermark
                .saturating_sub(progress.published_watermark),
        })
    }

    pub fn compact_outbox(&self) -> Result<u64> {
        let progresses = list_progress(self.storage)?;
        let floor = progresses
            .iter()
            .map(|progress| progress.published_watermark)
            .min()
            .unwrap_or(0);
        self.storage.compact_outbox_through(floor)?;
        Ok(floor)
    }

    fn is_stale(&self, event: &MutationEvent) -> Result<bool> {
        let Some(entity) = &event.entity else {
            return Ok(false);
        };
        Ok(self
            .storage
            .get_entity_state(entity)?
            .is_some_and(|state| state.source_version != event.source_version))
    }

    fn finish_job(&self, job: &mut DurableIndexJob) -> Result<()> {
        job.lease_until = None;
        job.next_attempt = None;
        save_job(self.storage, job)?;
        let mut progress = load_progress(self.storage, &job.target)?;
        if progress.published_watermark + 1 == job.event_sequence {
            progress.published_watermark = job.event_sequence;
            save_progress(self.storage, &progress)?;
        }
        Ok(())
    }

    fn fail_job(
        &self,
        job: &mut DurableIndexJob,
        error: TargetApplyError,
        now: DateTime<Utc>,
    ) -> Result<()> {
        job.lease_until = None;
        job.last_error = Some(redact_error(&error.message));
        if error.retryable && job.attempts < self.config.max_attempts {
            job.state = IndexJobState::Retryable;
            job.next_attempt = Some(now + self.config.retry_delay);
        } else {
            job.state = IndexJobState::Dead;
            job.next_attempt = None;
        }
        save_job(self.storage, job)
    }
}

fn next_job(mut jobs: Vec<DurableIndexJob>, now: DateTime<Utc>) -> Option<DurableIndexJob> {
    jobs.sort_by_key(|job| job.event_sequence);
    for job in jobs {
        match job.state {
            IndexJobState::Completed | IndexJobState::Stale => continue,
            IndexJobState::Pending => return Some(job),
            IndexJobState::Retryable if job.next_attempt.is_none_or(|next| next <= now) => {
                return Some(job);
            }
            IndexJobState::Claimed if job.lease_until.is_none_or(|lease| lease <= now) => {
                return Some(job);
            }
            IndexJobState::Retryable | IndexJobState::Claimed | IndexJobState::Dead => return None,
        }
    }
    None
}

fn redact_error(message: &str) -> String {
    let mut redacted = message.chars().take(256).collect::<String>();
    for marker in ["api_key", "authorization", "bearer"] {
        if redacted.to_ascii_lowercase().contains(marker) {
            redacted = "redacted provider error".into();
            break;
        }
    }
    redacted
}
