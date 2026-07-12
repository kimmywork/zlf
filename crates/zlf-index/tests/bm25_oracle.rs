use zlf_index::{bm25_term_score, Bm25Config, UnicodeJiebaAnalyzer};

#[test]
fn hand_calculated_bm25_fixture_matches_formula() {
    let score = bm25_term_score(2, 2, 3, 4, 3.0, 1.2, 0.75);
    assert!((score - 0.590_861_7).abs() < 1e-6, "score={score}");
}

#[test]
fn term_frequency_and_length_normalization_change_rank_as_expected() {
    let repeated = bm25_term_score(3, 2, 3, 3, 5.0, 1.2, 0.75);
    let verbose = bm25_term_score(1, 2, 3, 10, 5.0, 1.2, 0.75);
    assert!(repeated > verbose);
}

#[test]
fn configuration_rejects_invalid_and_unbounded_shapes() {
    assert!(Bm25Config::default().validate().is_ok());
    assert!(Bm25Config {
        b: 1.1,
        ..Default::default()
    }
    .validate()
    .is_err());
    assert!(Bm25Config {
        top_k: 100,
        candidate_limit: 10,
        ..Default::default()
    }
    .validate()
    .is_err());
}

#[test]
fn analyzer_has_deterministic_english_chinese_and_mixed_goldens() {
    let analyzer = UnicodeJiebaAnalyzer::default();
    assert_eq!(
        analyzer.analyze("Alice DATABASE"),
        vec!["alice", "database"]
    );
    let chinese = analyzer.analyze("软件工程师");
    assert!(chinese.contains(&"软件".to_string()));
    assert!(chinese.contains(&"工程师".to_string()));
    assert_eq!(
        analyzer.analyze("Alice 是软件工程师"),
        analyzer.analyze("Alice 是软件工程师")
    );
}

#[test]
fn empty_corpus_statistics_score_zero() {
    assert_eq!(bm25_term_score(1, 0, 0, 1, 0.0, 1.2, 0.75), 0.0);
}
