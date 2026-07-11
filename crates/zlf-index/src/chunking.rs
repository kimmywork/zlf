use sha2::{Digest, Sha256};

use crate::{ChunkingProfile, ContentFingerprint, SourceRange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexChunk {
    pub chunk_id: String,
    pub text: String,
    pub source_range: SourceRange,
    pub ordinal: u32,
    pub content_fingerprint: ContentFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplicitChunk {
    pub chunk_id: String,
    pub text: String,
    pub source_range: SourceRange,
}

pub fn chunk_text(profile: &ChunkingProfile, text: &str) -> Result<Vec<IndexChunk>, String> {
    match profile {
        ChunkingProfile::Explicit { .. } => {
            Err("explicit chunk profile requires adapter chunks".into())
        }
        ChunkingProfile::WholeField { version } => {
            validate_version(*version)?;
            Ok((!text.is_empty())
                .then(|| make_chunk(text, 0..text.len(), 0, None))
                .into_iter()
                .collect())
        }
        ChunkingProfile::ParagraphHeading { version } => {
            validate_version(*version)?;
            Ok(chunks_from_ranges(text, paragraph_ranges(text)))
        }
        ChunkingProfile::FixedTokenWindow {
            version,
            size,
            overlap,
        } => {
            validate_window(*version, *size, *overlap)?;
            Ok(chunks_from_ranges(
                text,
                window_ranges(text, *size as usize, *overlap as usize),
            ))
        }
    }
}

pub fn accept_explicit_chunks(
    version: u32,
    chunks: &[ExplicitChunk],
) -> Result<Vec<IndexChunk>, String> {
    validate_version(version)?;
    chunks
        .iter()
        .enumerate()
        .map(|(ordinal, chunk)| {
            if chunk.chunk_id.is_empty() || chunk.text.is_empty() || !chunk.source_range.is_valid()
            {
                return Err("explicit chunks require id, text, and a valid range".into());
            }
            Ok(make_chunk(
                &chunk.text,
                0..chunk.text.len(),
                ordinal as u32,
                Some((chunk.chunk_id.clone(), chunk.source_range)),
            ))
        })
        .collect()
}

pub fn content_fingerprint(text: &str) -> ContentFingerprint {
    let bytes = Sha256::digest(text.as_bytes());
    ContentFingerprint(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn chunks_from_ranges(text: &str, ranges: Vec<std::ops::Range<usize>>) -> Vec<IndexChunk> {
    ranges
        .into_iter()
        .enumerate()
        .map(|(ordinal, range)| make_chunk(text, range, ordinal as u32, None))
        .collect()
}

fn make_chunk(
    source: &str,
    range: std::ops::Range<usize>,
    ordinal: u32,
    explicit: Option<(String, SourceRange)>,
) -> IndexChunk {
    let text = source[range.clone()].to_string();
    let fingerprint = content_fingerprint(&text);
    let source_range = explicit.as_ref().map_or(
        SourceRange {
            start: range.start as u64,
            end: range.end as u64,
        },
        |item| item.1,
    );
    let chunk_id = explicit.map_or_else(
        || format!("{ordinal}-{}", &fingerprint.0[..16]),
        |item| item.0,
    );
    IndexChunk {
        chunk_id,
        text,
        source_range,
        ordinal,
        content_fingerprint: fingerprint,
    }
}

fn paragraph_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut paragraph_start = None;
    for line in line_ranges(text) {
        let content = text[line.clone()].trim();
        let heading = content.starts_with('#');
        if content.is_empty() || heading {
            if let Some(start) = paragraph_start.take() {
                ranges.push(trim_range(text, start..line.start));
            }
            if heading {
                ranges.push(trim_range(text, line));
            }
        } else if paragraph_start.is_none() {
            paragraph_start = Some(line.start);
        }
    }
    if let Some(start) = paragraph_start {
        ranges.push(trim_range(text, start..text.len()));
    }
    ranges
        .into_iter()
        .filter(|range| range.start < range.end)
        .collect()
}

fn line_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut start = 0;
    let mut ranges = Vec::new();
    for (index, character) in text.char_indices() {
        if character == '\n' {
            ranges.push(start..index + 1);
            start = index + 1;
        }
    }
    if start < text.len() {
        ranges.push(start..text.len());
    }
    ranges
}

fn trim_range(text: &str, mut range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    while range.start < range.end {
        let character = text[range.clone()].chars().next().unwrap();
        if !character.is_whitespace() {
            break;
        }
        range.start += character.len_utf8();
    }
    while range.start < range.end {
        let character = text[range.clone()].chars().next_back().unwrap();
        if !character.is_whitespace() {
            break;
        }
        range.end -= character.len_utf8();
    }
    range
}

fn window_ranges(text: &str, size: usize, overlap: usize) -> Vec<std::ops::Range<usize>> {
    let tokens = token_ranges(text);
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < tokens.len() {
        let end = (start + size).min(tokens.len());
        ranges.push(tokens[start].start..tokens[end - 1].end);
        if end == tokens.len() {
            break;
        }
        start = end - overlap;
    }
    ranges
}

fn token_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut ascii_start = None;
    for (index, character) in text.char_indices() {
        let end = index + character.len_utf8();
        if character.is_ascii_alphanumeric() || character == '_' {
            ascii_start.get_or_insert(index);
        } else {
            if let Some(start) = ascii_start.take() {
                ranges.push(start..index);
            }
            if !character.is_whitespace() && !character.is_ascii_punctuation() {
                ranges.push(index..end);
            }
        }
    }
    if let Some(start) = ascii_start {
        ranges.push(start..text.len());
    }
    ranges
}

fn validate_version(version: u32) -> Result<(), String> {
    (version > 0)
        .then_some(())
        .ok_or_else(|| "chunk profile version must be positive".into())
}

fn validate_window(version: u32, size: u32, overlap: u32) -> Result<(), String> {
    validate_version(version)?;
    (size > 0 && overlap < size)
        .then_some(())
        .ok_or_else(|| "window size must be positive and overlap smaller than size".into())
}
