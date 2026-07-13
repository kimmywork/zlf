use chrono::DateTime;
use zlf_core::EntityRef;
use zlf_index::{
    EventRecord, EventTimeStore, GenerationId, IndexDocumentId, TemporalRecordId, ValidityStore,
    TEMPORAL_RECORD_SCHEMA_VERSION,
};
use zlf_prolog::wam::{IndexAnswerLimits, IndexFactProvider, ProofKind, WamRuntime};
use zlf_prolog::{PrologParser, Term};

#[test]
fn provider_bounds_candidates_answers_and_reports_exhaustion() {
    let fixture = Fixture::new(6);
    let provider = fixture.provider(IndexAnswerLimits {
        candidate_limit: 4,
        answer_limit: 2,
    });
    let answers = WamRuntime::new(16)
        .query_all_with_provider(&query(), &provider)
        .unwrap();

    assert_eq!(answers.len(), 2);
    assert_eq!(answers[0].get("Node"), Some(&Term::Atom("node-00".into())));
    assert_eq!(answers[1].get("Node"), Some(&Term::Atom("node-01".into())));
    let metrics = provider.answer_metrics();
    assert_eq!(metrics.calls, 1);
    assert_eq!(metrics.candidates_produced, 4);
    assert_eq!(metrics.answers_produced, 2);
    assert_eq!(metrics.peak_materialized_answers, 2);
    assert_eq!(metrics.candidate_budget_exhaustions, 1);
    assert_eq!(metrics.answer_budget_exhaustions, 1);
}

#[test]
#[allow(clippy::too_many_lines)]
fn bounded_answers_preserve_once_cut_and_external_proof_leaves() {
    let fixture = Fixture::new(6);
    let limits = IndexAnswerLimits {
        candidate_limit: 4,
        answer_limit: 2,
    };
    let once_provider = fixture.provider(limits);
    let once = WamRuntime::new(16)
        .query_all_with_provider(
            &term("once(temporal_on(\"2026-01-01\", Node))"),
            &once_provider,
        )
        .unwrap();
    assert_eq!(once.len(), 1);
    assert_eq!(once_provider.answer_metrics().peak_materialized_answers, 2);

    let cut_provider = fixture.provider(limits);
    let mut cut_runtime = WamRuntime::new(16);
    cut_runtime.add_rule(
        PrologParser::parse_rule("first_event(Node) :- temporal_on(\"2026-01-01\", Node), !.")
            .unwrap(),
    );
    let cut = cut_runtime
        .query_all_with_provider(&term("first_event(Node)"), &cut_provider)
        .unwrap();
    assert_eq!(cut.len(), 1);

    let proof_provider = fixture.provider(limits);
    let proofs = WamRuntime::new(16)
        .query_all_with_provider_with_proof(&query(), &proof_provider)
        .unwrap();
    assert_eq!(proofs.len(), 2);
    assert!(proofs.iter().all(|answer| answer
        .proof
        .nodes
        .iter()
        .any(|node| node.clause.kind == ProofKind::Fact)));
}

#[test]
fn invalid_limits_fail_before_provider_execution() {
    let fixture = Fixture::new(1);
    assert!(IndexFactProvider::new()
        .with_temporal(&fixture.events, &fixture.validities, &fixture.generation)
        .with_limits(IndexAnswerLimits {
            candidate_limit: 1,
            answer_limit: 2,
        })
        .is_err());
    assert!(IndexFactProvider::new()
        .with_limits(IndexAnswerLimits {
            candidate_limit: 0,
            answer_limit: 0,
        })
        .is_err());
}

struct Fixture {
    _directory: tempfile::TempDir,
    events: EventTimeStore,
    validities: ValidityStore,
    generation: GenerationId,
}

impl Fixture {
    fn new(count: usize) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let events = EventTimeStore::open(directory.path().join("events")).unwrap();
        let validities = ValidityStore::open(directory.path().join("validities")).unwrap();
        let generation = GenerationId("g1".into());
        let records = (0..count)
            .map(|index| EventRecord {
                schema_version: TEMPORAL_RECORD_SCHEMA_VERSION,
                generation: generation.clone(),
                id: TemporalRecordId(format!("event-{index:02}")),
                document_id: IndexDocumentId::new(
                    EntityRef::Node(format!("node-{index:02}")),
                    "event",
                    "0",
                ),
                source_version: 1,
                at_micros: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                    .unwrap()
                    .timestamp_micros(),
            })
            .collect::<Vec<_>>();
        events.apply(&records, &[]).unwrap();
        Self {
            _directory: directory,
            events,
            validities,
            generation,
        }
    }

    fn provider(&self, limits: IndexAnswerLimits) -> IndexFactProvider<'_> {
        IndexFactProvider::new()
            .with_temporal(&self.events, &self.validities, &self.generation)
            .with_limits(limits)
            .unwrap()
    }
}

fn query() -> Term {
    term("temporal_on(\"2026-01-01\", Node)")
}

fn term(source: &str) -> Term {
    PrologParser::parse_term(source).unwrap()
}
