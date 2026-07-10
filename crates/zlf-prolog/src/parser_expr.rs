use zlf_core::Result;

use crate::parser::PrologParser;
use crate::parser_ast::Term;
use crate::parser_expr_scan::{
    find_top_level_operator, has_top_level_char, split_operator, split_operator_right_assoc,
    split_top_level, split_word_operator,
};

pub fn parse_query_terms(input: &str) -> Result<Option<Vec<Term>>> {
    let trimmed = input.trim();
    if !trimmed.starts_with('?') || !trimmed.ends_with('.') {
        return Ok(None);
    }
    let body = &trimmed[1..trimmed.len() - 1];
    let parts = split_top_level(body, ',')?;
    let mut terms = Vec::new();
    for part in parts {
        terms.push(parse_term_expr(&part)?);
    }
    Ok(Some(terms))
}

#[allow(clippy::too_many_lines)]
pub fn parse_term_expr(input: &str) -> Result<Term> {
    let text = strip_outer_parens(input.trim());
    if let Some(rest) = text.strip_prefix("\\+") {
        return Ok(compound("\\+", vec![parse_term_expr(rest)?]));
    }
    if let Some((left, right)) = split_operator(text, ":-") {
        return Ok(compound(
            ":-",
            vec![parse_term_expr(left)?, parse_term_expr(right)?],
        ));
    }
    if let Some((left, right)) = split_operator(text, "->") {
        return Ok(compound(
            "->",
            vec![parse_term_expr(left)?, parse_term_expr(right)?],
        ));
    }
    if let Some((left, right)) = split_operator(text, ";") {
        return Ok(compound(
            ";",
            vec![parse_term_expr(left)?, parse_term_expr(right)?],
        ));
    }
    if let Some((left, right)) = split_operator(text, ",") {
        return Ok(compound(
            ",",
            vec![parse_term_expr(left)?, parse_term_expr(right)?],
        ));
    }
    if let Some((left, right)) = split_word_operator(text, "is") {
        return Ok(compound(
            "is",
            vec![parse_term_expr(left)?, parse_arith(right)?],
        ));
    }
    for op in [
        "=:=", "=\\=", "\\==", "\\=", "==", "@=<", "@>=", "@<", "@>", "=..", "=<", ">=", "<", ">",
        "=",
    ] {
        if let Some((left, right)) = split_operator(text, op) {
            let term_operator = matches!(
                op,
                "=" | "\\=" | "=.." | "==" | "\\==" | "@<" | "@=<" | "@>" | "@>="
            );
            let right_term = if term_operator {
                parse_term_expr(right)?
            } else {
                parse_arith(right)?
            };
            let left_term = if term_operator {
                parse_term_expr(left)?
            } else {
                parse_arith(left)?
            };
            return Ok(compound(op, vec![left_term, right_term]));
        }
    }
    if text.starts_with('[')
        && text.ends_with(']')
        && has_top_level_char(&text[1..text.len() - 1], '|')
    {
        return parse_list_tail(text);
    }
    if let Some(term) = parse_compound_expr(text)? {
        return Ok(term);
    }
    if looks_arithmetic(text) {
        return parse_arith(text);
    }
    PrologParser::parse_term_pest(text)
}

fn parse_arith(input: &str) -> Result<Term> {
    let text = strip_outer_parens(input.trim());
    for op in ["+", "-"] {
        if let Some((left, right)) = split_operator_right_assoc(text, op) {
            if !left.trim().is_empty() {
                return Ok(compound(op, vec![parse_arith(left)?, parse_arith(right)?]));
            }
        }
    }
    for op in ["mod", "rem", "//", "*", "/"] {
        let found = if matches!(op, "mod" | "rem") {
            split_word_operator(text, op)
        } else {
            split_operator_right_assoc(text, op)
        };
        if let Some((left, right)) = found {
            return Ok(compound(op, vec![parse_arith(left)?, parse_arith(right)?]));
        }
    }
    if let Some(rest) = text.strip_prefix('+') {
        if !rest.trim().is_empty() {
            return Ok(compound("+", vec![parse_arith(rest)?]));
        }
    }
    if let Some(rest) = text.strip_prefix('-') {
        if !rest.trim().is_empty() && rest.trim().parse::<f64>().is_err() {
            return Ok(compound("-", vec![parse_arith(rest)?]));
        }
    }
    PrologParser::parse_term_pest(text)
}

fn parse_compound_expr(text: &str) -> Result<Option<Term>> {
    let Some(open) = text.find('(') else {
        return Ok(None);
    };
    if !text.ends_with(')') || open == 0 {
        return Ok(None);
    }
    let name = text[..open].trim();
    let Some(name) = callable_name(name) else {
        return Ok(None);
    };
    let inner = &text[open + 1..text.len() - 1];
    let args = split_top_level(inner, ',')?
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .map(|part| parse_term_expr(&part))
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(compound(name, args)))
}

fn parse_list_tail(text: &str) -> Result<Term> {
    let inner = &text[1..text.len() - 1];
    let Some((head, tail)) = split_operator(inner, "|") else {
        return PrologParser::parse_term_pest(text);
    };
    let items = split_top_level(head, ',')?
        .into_iter()
        .filter(|item| !item.trim().is_empty())
        .map(|item| parse_term_expr(&item))
        .collect::<Result<Vec<_>>>()?;
    Ok(items
        .into_iter()
        .rev()
        .fold(parse_term_expr(tail)?, |tail, head| {
            compound(".", vec![head, tail])
        }))
}

fn strip_outer_parens(input: &str) -> &str {
    let mut text = input;
    loop {
        if !(text.starts_with('(') && text.ends_with(')')) {
            return text;
        }
        let inner = &text[1..text.len() - 1];
        if split_top_level(inner, ',').is_ok_and(|parts| parts.len() == 1) {
            text = inner.trim();
        } else {
            return text;
        }
    }
}

fn looks_arithmetic(input: &str) -> bool {
    ["+", "*", "/", "//"]
        .iter()
        .any(|op| find_top_level_operator(input, op).is_some())
        || split_word_operator(input, "mod").is_some()
        || input.starts_with('-') && input[1..].contains(char::is_alphabetic)
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn callable_name(name: &str) -> Option<String> {
    if name.starts_with('\'') && name.ends_with('\'') {
        return Some(crate::parser_helpers::atom_text(name));
    }
    let mut chars = name.chars();
    if matches!(chars.next(), Some(ch) if ch.is_ascii_lowercase() || ch == '_')
        && chars.all(is_word_char)
    {
        Some(name.to_string())
    } else {
        None
    }
}

fn compound(name: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        name: name.into(),
        args,
    }
}
