use zlf_core::{Result, ZlfError};

pub(crate) fn split_top_level(input: &str, separator: char) -> Result<Vec<String>> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in input.char_indices() {
        if update_quote(ch, &mut quote, &mut escaped) {
            continue;
        }
        match ch {
            '(' | '[' | '{' if quote.is_none() => depth += 1,
            ')' | ']' | '}' if quote.is_none() => depth -= 1,
            _ if ch == separator && depth == 0 && quote.is_none() => {
                parts.push(input[start..index].trim().to_string());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
        if depth < 0 {
            return Err(ZlfError::SyntaxError(
                0,
                "unbalanced delimiters".to_string(),
            ));
        }
    }
    parts.push(input[start..].trim().to_string());
    Ok(parts)
}

pub(crate) fn split_word_operator<'a>(input: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    find_top_level_operator(input, op).and_then(|index| {
        let before = input[..index].chars().last();
        let after = input[index + op.len()..].chars().next();
        if before.is_some_and(is_word_char) || after.is_some_and(is_word_char) {
            None
        } else {
            Some((&input[..index], &input[index + op.len()..]))
        }
    })
}

pub(crate) fn split_operator<'a>(input: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    find_top_level_operator(input, op).map(|index| (&input[..index], &input[index + op.len()..]))
}

pub(crate) fn split_operator_right_assoc<'a>(
    input: &'a str,
    op: &str,
) -> Option<(&'a str, &'a str)> {
    scan_top_level(input)
        .into_iter()
        .rev()
        .find(|index| input[*index..].starts_with(op))
        .map(|index| (&input[..index], &input[index + op.len()..]))
}

pub(crate) fn find_top_level_operator(input: &str, op: &str) -> Option<usize> {
    scan_top_level(input)
        .into_iter()
        .find(|index| input[*index..].starts_with(op))
}

pub(crate) fn has_top_level_char(input: &str, needle: char) -> bool {
    scan_top_level(input)
        .into_iter()
        .any(|index| input[index..].starts_with(needle))
}

fn scan_top_level(input: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in input.char_indices() {
        if update_quote(ch, &mut quote, &mut escaped) {
            continue;
        }
        match ch {
            '(' | '[' | '{' if quote.is_none() => depth += 1,
            ')' | ']' | '}' if quote.is_none() => depth -= 1,
            _ if depth == 0 && quote.is_none() => positions.push(index),
            _ => {}
        }
    }
    positions
}

fn update_quote(ch: char, quote: &mut Option<char>, escaped: &mut bool) -> bool {
    if *escaped {
        *escaped = false;
        return true;
    }
    if quote.is_some() && ch == '\\' {
        *escaped = true;
        return true;
    }
    if matches!(ch, '\'' | '"') {
        if *quote == Some(ch) {
            *quote = None;
        } else if quote.is_none() {
            *quote = Some(ch);
        }
        return true;
    }
    false
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
