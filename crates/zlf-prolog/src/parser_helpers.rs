use zlf_core::Result;

use crate::parser::PrologParser;
use crate::parser_ast::Term;

pub(crate) fn atom_text(text: &str) -> String {
    text.strip_prefix('\'')
        .and_then(|text| text.strip_suffix('\''))
        .map_or_else(|| text.to_string(), |text| text.replace("\\'", "'"))
}

pub(crate) fn string_text(text: &str) -> String {
    let content = &text[1..text.len() - 1];
    let mut value = String::new();
    let mut chars = content.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            value.push(character);
            continue;
        }
        match chars.next() {
            Some('n') => value.push('\n'),
            Some('r') => value.push('\r'),
            Some('t') => value.push('\t'),
            Some('"') => value.push('"'),
            Some('\\') => value.push('\\'),
            Some(other) => {
                value.push('\\');
                value.push(other);
            }
            None => value.push('\\'),
        }
    }
    value
}

pub(crate) fn parse_directive(body: &str) -> Result<Term> {
    for name in ["dynamic", "table"] {
        if let Some(argument) = body.strip_prefix(name) {
            return Ok(Term::Compound {
                name: name.to_string(),
                args: vec![PrologParser::parse_term(argument.trim())?],
            });
        }
    }
    PrologParser::parse_term(body)
}

pub(crate) fn canonical_list(items: Vec<Term>, tail: Term) -> Term {
    items
        .into_iter()
        .rev()
        .fold(tail, |tail, head| Term::Compound {
            name: ".".to_string(),
            args: vec![head, tail],
        })
}
