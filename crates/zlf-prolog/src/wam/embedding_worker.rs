use std::thread;
use std::time::Duration;

use super::error::WamResult;
use super::persistent_embedding_queue::PersistentEmbeddingQueue;
use super::storage_index_writer::Embedder;
use zlf_index::VectorIndex;

pub struct EmbeddingWorker<'a> {
    queue: &'a PersistentEmbeddingQueue<'a>,
    embedder: &'a dyn Embedder,
    index: &'a VectorIndex,
    poll_interval: Duration,
}

impl<'a> EmbeddingWorker<'a> {
    pub fn new(
        queue: &'a PersistentEmbeddingQueue<'a>,
        embedder: &'a dyn Embedder,
        index: &'a VectorIndex,
    ) -> Self {
        Self {
            queue,
            embedder,
            index,
            poll_interval: Duration::from_millis(250),
        }
    }

    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    pub fn run_once(&self) -> WamResult<usize> {
        self.queue.process_all(self.embedder, self.index)
    }

    pub fn run_until_idle(&self, max_idle_ticks: usize) -> WamResult<usize> {
        let mut processed = 0;
        let mut idle_ticks = 0;
        while idle_ticks < max_idle_ticks {
            let count = self.run_once()?;
            processed += count;
            if count == 0 {
                idle_ticks += 1;
                thread::sleep(self.poll_interval);
            } else {
                idle_ticks = 0;
            }
        }
        Ok(processed)
    }

    pub fn run_forever(&self) -> WamResult<()> {
        loop {
            self.run_once()?;
            thread::sleep(self.poll_interval);
        }
    }
}
