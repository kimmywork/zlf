use std::collections::VecDeque;

use crate::parser::Term;

use super::error::{WamError, WamResult};
use super::storage_index_writer::Embedder;
use zlf_index::{VectorEntry, VectorIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingJob {
    pub node_id: String,
    pub text: String,
}

#[derive(Debug, Default)]
pub struct EmbeddingQueue {
    jobs: VecDeque<EmbeddingJob>,
}

impl EmbeddingQueue {
    pub fn new() -> Self {
        Self {
            jobs: VecDeque::new(),
        }
    }

    pub fn push(&mut self, node_id: impl Into<String>, text: impl Into<String>) {
        self.jobs.push_back(EmbeddingJob {
            node_id: node_id.into(),
            text: text.into(),
        });
    }

    pub fn enqueue_fact(&mut self, fact: &Term) -> WamResult<()> {
        let Some((name, args)) = compound(fact) else {
            return Ok(());
        };
        match (name, args) {
            ("node", [id, props]) => self.enqueue_object(atom(id)?, props),
            ("node", [id, _, props]) => self.enqueue_object(atom(id)?, props),
            ("property", [id, _, value]) => self.enqueue_value(atom(id)?, value),
            (name, [id, value]) if name.starts_with("prop_") => {
                self.enqueue_value(atom(id)?, value)
            }
            _ => Ok(()),
        }
    }

    pub fn process_all(
        &mut self,
        embedder: &dyn Embedder,
        index: &VectorIndex,
    ) -> WamResult<usize> {
        let mut processed = 0;
        while let Some(job) = self.jobs.pop_front() {
            index
                .add_entry(VectorEntry {
                    node_id: job.node_id,
                    embedding: embedder.embed(&job.text)?,
                    model: embedder.model().to_string(),
                })
                .map_err(provider_error)?;
            processed += 1;
        }
        Ok(processed)
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    fn enqueue_object(&mut self, node_id: &str, term: &Term) -> WamResult<()> {
        if let Term::Object(entries) = term {
            for (_, value) in entries {
                self.enqueue_value(node_id, value)?;
            }
        }
        Ok(())
    }

    fn enqueue_value(&mut self, node_id: &str, term: &Term) -> WamResult<()> {
        match term {
            Term::Atom(value) | Term::String(value) => self.push(node_id, value),
            Term::List(items) => {
                for item in items {
                    self.enqueue_value(node_id, item)?;
                }
            }
            Term::Object(entries) => {
                for (_, value) in entries {
                    self.enqueue_value(node_id, value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn compound(term: &Term) -> Option<(&str, &[Term])> {
    match term {
        Term::Compound { name, args } => Some((name, args)),
        _ => None,
    }
}

fn atom(term: &Term) -> WamResult<&str> {
    match term {
        Term::Atom(value) | Term::String(value) => Ok(value),
        _ => Err(WamError::Provider("expected atom".to_string())),
    }
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
