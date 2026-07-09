use std::collections::HashSet;

use zlf_core::{Result, ZlfError};
use zlf_prolog::wam::{FactKey, StorageFactWriter};

use crate::ZlfDatabase;

impl ZlfDatabase {
    /// Delete a fact from storage.  Returns the canonical FactKey if
    /// the term was recognized and deletion was attempted.
    /// Delete a fact from storage.  `source` is Prolog syntax like
    /// `retract(person(alice)).` or `person(alice).`.
    /// When wrapped in `retract(...)`, the wrapper is stripped.
    /// Returns the canonical FactKey if deletion was attempted.
    pub fn retract_fact(&self, source: &str) -> Result<Option<FactKey>> {
        let source = source.trim().trim_end_matches('.');
        let inner = if let Some(stripped) = source.strip_prefix("retract(") {
            stripped.strip_suffix(')').unwrap_or(stripped)
        } else {
            source
        };
        let head = if inner.contains('(') || inner.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            zlf_prolog::PrologParser::parse_term(inner)
                .map_err(|e| ZlfError::SyntaxError(0, e.to_string()))?
        } else {
            let fact = zlf_prolog::PrologParser::parse_fact(source)
                .map_err(|e| ZlfError::SyntaxError(0, e.to_string()))?;
            fact.head
        };
        let writer = StorageFactWriter::new(self.storage.as_ref());
        writer
            .retract_fact(&head)
            .map_err(|e| ZlfError::Internal(e.to_string()))
    }

    /// Deduplicate query results by removing rows where all variable
    /// bindings produce the same canonical representation.
    pub fn dedupe_results(&self, results: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
        let mut seen = HashSet::new();
        let mut deduped = Vec::new();
        for row in results {
            let canonical = serde_json::to_string(&row).unwrap_or_default();
            if seen.insert(canonical) {
                deduped.push(row);
            }
        }
        deduped
    }
}
