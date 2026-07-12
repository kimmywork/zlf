use std::sync::Arc;

use zlf_core::{Result, ZlfError};
use zlf_prolog::wam::{
    CompiledRuleArtifact, CompositeFactProvider, GraphAlgorithmProvider, GraphViewProvider,
    IndexFactProvider, IntrospectionProvider, ProofAnswer, StorageFactProvider, WamRuntime,
};
use zlf_prolog::{PrologParser, Query, Term};

use super::{helpers, lock_error, ZlfDatabase};

impl ZlfDatabase {
    pub fn query_prolog_with_proof(&self, source: &str) -> Result<Vec<ProofAnswer>> {
        match PrologParser::parse_query(source)? {
            Query::Goal(term) => self.execute_terms_with_proof(&[term]),
            Query::Goals(terms) => self.execute_terms_with_proof(&terms),
            Query::RuleDef(rule) => {
                self.store_rule(rule)?;
                Ok(Vec::new())
            }
            Query::Directive(directive) => {
                self.apply_directive(&directive)?;
                Ok(Vec::new())
            }
        }
    }

    fn execute_terms_with_proof(&self, terms: &[Term]) -> Result<Vec<ProofAnswer>> {
        let storage_provider = StorageFactProvider::new(self.storage.as_ref());
        let bm25 = self.bm25.read().map_err(lock_error)?.clone();
        let index_provider = IndexFactProvider::new()
            .with_bm25(bm25.as_ref())
            .with_vector(self.vector.as_ref())
            .with_temporal(self.temporal.as_ref());
        let registry = self.registry.read().map_err(lock_error)?.clone();
        let rules = self.rules.read().map_err(lock_error)?.clone();
        let introspection = IntrospectionProvider::new(registry, &rules);
        let graph_view = GraphViewProvider::new(self.storage.as_ref());
        let graph_algo = GraphAlgorithmProvider::new(self.storage.as_ref());
        let provider = CompositeFactProvider::new()
            .with(&storage_provider)
            .with(&index_provider)
            .with(&introspection)
            .with(&graph_view)
            .with(&graph_algo);
        let (runtime, query) = self.proof_runtime(terms, rules)?;
        runtime
            .query_all_with_provider_and_storage_with_proof(
                &query,
                &provider,
                self.storage.as_ref(),
            )
            .map_err(|error| ZlfError::Internal(error.to_string()))
    }

    fn proof_runtime(
        &self,
        terms: &[Term],
        rules: Vec<CompiledRuleArtifact>,
    ) -> Result<(WamRuntime, Term)> {
        let mut runtime = WamRuntime::new(64);
        runtime.set_table_manager(Arc::clone(&self.table_manager));
        for key in self.tabled.read().map_err(lock_error)?.iter().cloned() {
            runtime.declare_tabled(key);
        }
        for artifact in rules {
            runtime.add_compiled_rule(artifact);
        }
        let (query, wrapper) = helpers::query_plan(terms);
        if let Some(rule) = wrapper {
            runtime.add_rule(rule);
        }
        Ok((runtime, query))
    }
}
