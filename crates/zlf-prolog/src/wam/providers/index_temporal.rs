use zlf_index::parse_utc_micros;

use crate::parser::Term;

use super::error::WamResult;
use super::index_provider::{
    atom, bound_entity, compound_term, constant, provider_error, string, IndexFactProvider,
};

impl IndexFactProvider<'_> {
    pub(super) fn temporal_on_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [date, target] = args else {
            return Ok(Vec::new());
        };
        let date = constant(date)?;
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        let candidates = event_day(
            index,
            date,
            bound_entity(target),
            self.limits.candidate_limit,
        )?
        .records
        .into_iter()
        .map(|record| {
            compound_term(
                "temporal_on",
                vec![string(date), atom(record.document_id.entity.id())],
            )
        })
        .collect::<Vec<_>>();
        Ok(self.finish(candidates))
    }

    pub(super) fn temporal_between_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let Some((start, end, start_micros, end_micros)) = temporal_range(args)? else {
            return Ok(Vec::new());
        };
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        let candidates = event_range(
            index,
            args.get(2).and_then(bound_entity),
            start_micros,
            end_micros,
            self.limits.candidate_limit,
        )?
        .records
        .into_iter()
        .map(|record| {
            range_term(
                "temporal_between",
                start,
                end,
                record.document_id.entity.id(),
            )
        })
        .collect::<Vec<_>>();
        Ok(self.finish(candidates))
    }

    pub(super) fn valid_at_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let [instant, target] = args else {
            return Ok(Vec::new());
        };
        let instant = constant(instant)?;
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        let micros = parse_utc_micros(instant).map_err(provider_error)?;
        let candidates = valid_at(
            index,
            bound_entity(target),
            micros,
            self.limits.candidate_limit,
        )?
        .records
        .into_iter()
        .map(|record| {
            compound_term(
                "valid_at",
                vec![string(instant), atom(record.document_id.entity.id())],
            )
        })
        .collect::<Vec<_>>();
        Ok(self.finish(candidates))
    }

    pub(super) fn valid_overlaps_facts(&self, args: &[Term]) -> WamResult<Vec<Term>> {
        let Some((start, end, start_micros, end_micros)) = temporal_range(args)? else {
            return Ok(Vec::new());
        };
        let Some(index) = self.temporal else {
            return Ok(Vec::new());
        };
        let candidates = valid_overlaps(
            index,
            args.get(2).and_then(bound_entity),
            start_micros,
            end_micros,
            self.limits.candidate_limit,
        )?
        .records
        .into_iter()
        .map(|record| range_term("valid_overlaps", start, end, record.document_id.entity.id()))
        .collect::<Vec<_>>();
        Ok(self.finish(candidates))
    }
}

fn event_day(
    index: super::index_provider::TemporalProvider<'_>,
    date: &str,
    entity: Option<zlf_core::EntityRef>,
    limit: usize,
) -> WamResult<zlf_index::EventQueryResult> {
    match entity {
        Some(entity) => {
            let (start, end) = zlf_index::utc_day_range(date).map_err(provider_error)?;
            index
                .events
                .range_for_entity(index.generation, &entity, start, end, limit)
        }
        None => index.events.day(index.generation, date, limit),
    }
    .map_err(provider_error)
}

fn event_range(
    index: super::index_provider::TemporalProvider<'_>,
    entity: Option<zlf_core::EntityRef>,
    start: i64,
    end: i64,
    limit: usize,
) -> WamResult<zlf_index::EventQueryResult> {
    match entity {
        Some(entity) => index
            .events
            .range_for_entity(index.generation, &entity, start, end, limit),
        None => index.events.range(index.generation, start, end, limit),
    }
    .map_err(provider_error)
}

fn valid_at(
    index: super::index_provider::TemporalProvider<'_>,
    entity: Option<zlf_core::EntityRef>,
    instant: i64,
    limit: usize,
) -> WamResult<zlf_index::ValidityQueryResult> {
    match entity {
        Some(entity) => {
            index
                .validities
                .valid_at_for_entity(index.generation, &entity, instant, limit)
        }
        None => index.validities.valid_at(index.generation, instant, limit),
    }
    .map_err(provider_error)
}

fn valid_overlaps(
    index: super::index_provider::TemporalProvider<'_>,
    entity: Option<zlf_core::EntityRef>,
    start: i64,
    end: i64,
    limit: usize,
) -> WamResult<zlf_index::ValidityQueryResult> {
    match entity {
        Some(entity) => {
            index
                .validities
                .overlaps_for_entity(index.generation, &entity, start, end, limit)
        }
        None => index
            .validities
            .overlaps(index.generation, start, end, limit),
    }
    .map_err(provider_error)
}

fn temporal_range(args: &[Term]) -> WamResult<Option<(&str, &str, i64, i64)>> {
    let [start, end, _] = args else {
        return Ok(None);
    };
    let start = constant(start)?;
    let end = constant(end)?;
    Ok(Some((
        start,
        end,
        parse_utc_micros(start).map_err(provider_error)?,
        parse_utc_micros(end).map_err(provider_error)?,
    )))
}

fn range_term(predicate: &str, start: &str, end: &str, entity: &str) -> Term {
    compound_term(predicate, vec![string(start), string(end), atom(entity)])
}
