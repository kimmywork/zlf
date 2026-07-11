use std::time::{Duration, Instant};

use zlf_core::Result;
use zlf_index::IndexWaitResult;
use zlf_storage::Storage;

use crate::{CoordinatorConfig, IndexCoordinator};

pub fn wait_for_indexes(
    storage: &Storage,
    targets: &[String],
    minimum_sequence: u64,
    timeout: Duration,
) -> Result<IndexWaitResult> {
    let deadline = Instant::now() + timeout;
    loop {
        let coordinator = IndexCoordinator::new(storage, CoordinatorConfig::default());
        let pending = targets
            .iter()
            .filter(|target| {
                coordinator.progress(target).map_or(true, |progress| {
                    progress.published_watermark < minimum_sequence
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if pending.is_empty() || Instant::now() >= deadline {
            return Ok(IndexWaitResult {
                reached: pending.is_empty(),
                minimum_sequence,
                pending_targets: pending,
            });
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
