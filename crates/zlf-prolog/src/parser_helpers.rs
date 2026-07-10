use zlf_core::Result;

use crate::parser::PrologParser;
use crate::parser_ast::Term;

pub(crate) fn atom_text(text: &str) -> String {
    text.strip_prefix('\'')
        .and_then(|text| text.strip_suffix('\''))
        .map_or_else(|| text.to_string(), |text| text.replace("\\'", "'"))
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
