use zlf_index::{accept_explicit_chunks, chunk_text, ChunkingProfile, ExplicitChunk, SourceRange};

#[test]
fn whole_field_is_deterministic_and_fingerprinted() {
    let profile = ChunkingProfile::WholeField { version: 1 };
    let first = chunk_text(&profile, "hello 世界").unwrap();
    let second = chunk_text(&profile, "hello 世界").unwrap();
    assert_eq!(first, second);
    assert_eq!(first[0].source_range, SourceRange { start: 0, end: 12 });
    assert_eq!(first[0].content_fingerprint.0.len(), 64);
}

#[test]
fn paragraph_heading_split_preserves_utf8_byte_ranges() {
    let text = "# 标题\n\nFirst paragraph.\ncontinued\n\n第二段";
    let chunks = chunk_text(&ChunkingProfile::ParagraphHeading { version: 1 }, text).unwrap();
    assert_eq!(
        chunks
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect::<Vec<_>>(),
        vec!["# 标题", "First paragraph.\ncontinued", "第二段"]
    );
    for chunk in chunks {
        assert_eq!(
            &text[chunk.source_range.start as usize..chunk.source_range.end as usize],
            chunk.text
        );
    }
}

#[test]
fn fixed_window_handles_mixed_chinese_and_english_with_overlap() {
    let chunks = chunk_text(
        &ChunkingProfile::FixedTokenWindow {
            version: 1,
            size: 3,
            overlap: 1,
        },
        "hello 世界 graph",
    )
    .unwrap();
    assert_eq!(
        chunks
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect::<Vec<_>>(),
        vec!["hello 世界", "界 graph"]
    );
}

#[test]
fn explicit_chunks_keep_adapter_identity_and_ranges() {
    let chunks = accept_explicit_chunks(
        1,
        &[ExplicitChunk {
            chunk_id: "adapter-7".into(),
            text: "selected text".into(),
            source_range: SourceRange { start: 10, end: 23 },
        }],
    )
    .unwrap();
    assert_eq!(chunks[0].chunk_id, "adapter-7");
    assert_eq!(chunks[0].source_range.start, 10);
}

#[test]
fn invalid_windows_and_explicit_profile_usage_fail() {
    assert!(chunk_text(
        &ChunkingProfile::FixedTokenWindow {
            version: 1,
            size: 2,
            overlap: 2,
        },
        "text"
    )
    .is_err());
    assert!(chunk_text(&ChunkingProfile::Explicit { version: 1 }, "text").is_err());
}
