use std::collections::BTreeMap;

use zlf_core::EntityRef;
use zlf_index::{EmbeddingModelProfile, ExactVectorStore, GenerationId, VectorQuery};

use crate::parser::Term;

use super::error::{WamError, WamResult};

#[derive(Clone, Copy)]
pub(super) struct ExactVectorProvider<'a> {
    store: &'a ExactVectorStore,
    profile: &'a EmbeddingModelProfile,
    generation: &'a GenerationId,
}

impl<'a> ExactVectorProvider<'a> {
    pub(super) fn new(
        store: &'a ExactVectorStore,
        profile: &'a EmbeddingModelProfile,
        generation: &'a GenerationId,
    ) -> Self {
        Self {
            store,
            profile,
            generation,
        }
    }

    pub(super) fn source_facts(
        &self,
        source: &str,
        target: Option<&str>,
        candidate_limit: usize,
    ) -> WamResult<(Vec<Term>, bool)> {
        let scores = self.source_scores(source, target, candidate_limit)?;
        let exhausted = scores.len() == candidate_limit;
        Ok((
            scores
                .into_iter()
                .map(|(node, score)| {
                    compound_term(
                        "vector_similar",
                        vec![atom(source), atom(node), number(score)],
                    )
                })
                .collect(),
            exhausted,
        ))
    }

    fn source_scores(
        &self,
        source: &str,
        target: Option<&str>,
        candidate_limit: usize,
    ) -> WamResult<Vec<(String, f32)>> {
        let records = self
            .store
            .records_for_entity(
                &self.generation.0,
                &self.profile.id,
                self.profile.version,
                &EntityRef::Node(source.into()),
            )
            .map_err(provider_error)?;
        let mut scores = BTreeMap::<String, f32>::new();
        for record in records {
            self.merge_record_scores(source, target, record.values, candidate_limit, &mut scores)?;
            trim_scores(&mut scores, candidate_limit);
        }
        let mut scores = scores.into_iter().collect::<Vec<_>>();
        scores.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        Ok(scores)
    }

    fn merge_record_scores(
        &self,
        source: &str,
        target: Option<&str>,
        values: Vec<f32>,
        candidate_limit: usize,
        scores: &mut BTreeMap<String, f32>,
    ) -> WamResult<()> {
        let query = VectorQuery {
            generation: self.generation.clone(),
            model_profile: self.profile.id.clone(),
            model_version: self.profile.version,
            values,
            top_k: candidate_limit,
            threshold: Some(0.0),
            include_sources: Vec::new(),
            exclude_sources: Vec::new(),
            include_entities: target
                .map(|target| vec![EntityRef::Node(target.into())])
                .unwrap_or_default(),
            exclude_entities: Vec::new(),
            fields: Vec::new(),
            metadata: BTreeMap::new(),
        };
        for hit in self
            .store
            .search(&query, self.profile)
            .map_err(provider_error)?
        {
            let target = hit.key.document_id.entity.id();
            if target != source {
                scores
                    .entry(target.to_string())
                    .and_modify(|score| *score = score.max(hit.score))
                    .or_insert(hit.score);
            }
        }
        Ok(())
    }
}

fn trim_scores(scores: &mut BTreeMap<String, f32>, limit: usize) {
    if scores.len() <= limit {
        return;
    }
    let mut ranked = scores
        .iter()
        .map(|(node, score)| (node.clone(), *score))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let retained = ranked
        .into_iter()
        .take(limit)
        .map(|(node, _)| node)
        .collect::<std::collections::BTreeSet<_>>();
    scores.retain(|node, _| retained.contains(node));
}

fn atom(value: impl Into<String>) -> Term {
    Term::Atom(value.into())
}

fn number(value: f32) -> Term {
    Term::Float(value as f64)
}

fn compound_term(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}

fn provider_error(error: impl std::fmt::Display) -> WamError {
    WamError::Provider(error.to_string())
}
