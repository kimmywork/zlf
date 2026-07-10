#![allow(
    clippy::never_loop,
    clippy::single_match,
    clippy::while_let_on_iterator
)]

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use zlf_core::{Result, ZlfError};

#[derive(Parser)]
#[grammar = "prolog.pest"]
pub struct PrologParser;

pub use crate::parser_ast::{Fact, PrologRule, Query, Term};

impl PrologParser {
    pub fn parse_term(input: &str) -> Result<Term> {
        crate::parser_expr::parse_term_expr(input)
    }

    pub(crate) fn parse_term_pest(input: &str) -> Result<Term> {
        let pairs = PrologParser::parse(Rule::term, input)
            .map_err(|e| ZlfError::SyntaxError(0, e.to_string()))?;

        Self::build_term(pairs.collect())
    }

    pub fn parse_fact(input: &str) -> Result<Fact> {
        let pairs = PrologParser::parse(Rule::fact, input)
            .map_err(|e| ZlfError::SyntaxError(0, e.to_string()))?;

        let mut terms = Vec::new();
        for pair in pairs {
            match pair.as_rule() {
                Rule::fact => {
                    for inner in pair.into_inner() {
                        match inner.as_rule() {
                            Rule::term => {
                                let term = Self::build_term(vec![inner])?;
                                terms.push(term);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        if terms.len() == 1 {
            Ok(Fact {
                head: terms.remove(0),
            })
        } else {
            Err(ZlfError::SyntaxError(0, "Invalid fact format".to_string()))
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn parse_rule(input: &str) -> Result<PrologRule> {
        let pairs = PrologParser::parse(Rule::rule, input)
            .map_err(|e| ZlfError::SyntaxError(0, e.to_string()))?;

        let mut head = None;
        let mut body = Vec::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::rule => {
                    for inner in pair.into_inner() {
                        match inner.as_rule() {
                            Rule::term => {
                                if head.is_none() {
                                    head = Some(Self::build_term(vec![inner])?);
                                }
                            }
                            Rule::body => {
                                // Parse body terms
                                for body_inner in inner.into_inner() {
                                    match body_inner.as_rule() {
                                        Rule::term => {
                                            body.push(Self::build_term(vec![body_inner])?);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        match head {
            Some(h) => Ok(PrologRule { head: h, body }),
            None => Err(ZlfError::SyntaxError(0, "Invalid rule format".to_string())),
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn parse_query(input: &str) -> Result<Query> {
        let input = input.trim();

        // Try to parse as query first (with ? prefix)
        if input.starts_with('?') {
            if let Some(mut terms) = crate::parser_expr::parse_query_terms(input)? {
                if terms.len() == 1 {
                    return Ok(Query::Goal(terms.remove(0)));
                }
                if !terms.is_empty() {
                    return Ok(Query::Goals(terms));
                }
            }
        }

        if input.starts_with(":-") && input.ends_with('.') {
            let body = input[2..input.len() - 1].trim();
            return Ok(Query::Directive(crate::parser_helpers::parse_directive(
                body,
            )?));
        }

        // Try to parse as rule (with :- separator)
        if input.contains(":-") {
            let rule = Self::parse_rule(input)?;
            return Ok(Query::RuleDef(rule));
        }

        // Try to parse as fact (term followed by .)
        if input.ends_with('.') {
            let pairs = PrologParser::parse(Rule::fact, input)
                .map_err(|e| ZlfError::SyntaxError(0, e.to_string()))?;

            for pair in pairs {
                match pair.as_rule() {
                    Rule::fact => {
                        for inner in pair.into_inner() {
                            match inner.as_rule() {
                                Rule::term => {
                                    let term = Self::build_term(vec![inner])?;
                                    // Convert fact to rule with empty body
                                    return Ok(Query::RuleDef(PrologRule {
                                        head: term,
                                        body: vec![],
                                    }));
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Err(ZlfError::SyntaxError(0, "Invalid query format".to_string()))
    }

    #[allow(clippy::too_many_lines)]
    fn build_term(pairs: Vec<Pair<Rule>>) -> Result<Term> {
        for pair in pairs {
            match pair.as_rule() {
                Rule::term => {
                    for inner in pair.into_inner() {
                        return Self::build_term(vec![inner]);
                    }
                }
                Rule::variable => {
                    let name = pair.as_str().to_string();
                    return Ok(Term::Variable(name));
                }
                Rule::atom => {
                    let name = pair.as_str().to_string();
                    return Ok(Term::Atom(name));
                }
                Rule::quoted_atom => {
                    let text = pair.as_str();
                    let value = text[1..text.len() - 1].replace("\\'", "'");
                    return Ok(Term::Atom(value));
                }
                Rule::cut => {
                    return Ok(Term::Atom("!".to_string()));
                }
                Rule::number => {
                    let text = pair.as_str();
                    if text.contains('.') {
                        let value: f64 = text.parse().map_err(|e| {
                            ZlfError::SyntaxError(0, format!("Invalid float: {}", e))
                        })?;
                        return Ok(Term::Float(value));
                    }
                    let value: i64 = text
                        .parse()
                        .map_err(|e| ZlfError::SyntaxError(0, format!("Invalid integer: {}", e)))?;
                    return Ok(Term::Integer(value));
                }
                Rule::string => {
                    return Ok(Term::String(crate::parser_helpers::string_text(
                        pair.as_str(),
                    )));
                }
                Rule::inequality => {
                    let mut inner = pair.into_inner();
                    let left = inner.next().ok_or_else(|| {
                        ZlfError::SyntaxError(0, "Invalid inequality".to_string())
                    })?;
                    let right = inner.next().ok_or_else(|| {
                        ZlfError::SyntaxError(0, "Invalid inequality".to_string())
                    })?;
                    return Ok(Term::Compound {
                        name: "\\=".to_string(),
                        args: vec![
                            Self::build_term(vec![left])?,
                            Self::build_term(vec![right])?,
                        ],
                    });
                }
                Rule::simple_term => {
                    for inner in pair.into_inner() {
                        return Self::build_term(vec![inner]);
                    }
                }
                Rule::compound => {
                    let mut inner = pair.into_inner();
                    let name = crate::parser_helpers::atom_text(inner.next().unwrap().as_str());
                    let mut args = Vec::new();

                    while let Some(arg) = inner.next() {
                        match arg.as_rule() {
                            Rule::term => {
                                args.push(Self::build_term(vec![arg])?);
                            }
                            _ => {}
                        }
                    }

                    return Ok(Term::Compound { name, args });
                }
                Rule::list => {
                    let inner = pair
                        .into_inner()
                        .next()
                        .ok_or_else(|| ZlfError::SyntaxError(0, "invalid list".to_string()))?;
                    return Self::build_list(inner);
                }
                Rule::object => {
                    let mut entries = Vec::new();
                    for pair in pair.into_inner().flat_map(|item| item.into_inner()) {
                        let mut inner = pair.into_inner();
                        let key = inner.next().unwrap().as_str().to_string();
                        let value = inner.next().unwrap();
                        entries.push((key, Self::build_term(vec![value])?));
                    }
                    return Ok(Term::Object(entries));
                }
                _ => {}
            }
        }

        Err(ZlfError::SyntaxError(0, "Failed to build term".to_string()))
    }

    fn build_list(pair: Pair<Rule>) -> Result<Term> {
        let improper = matches!(pair.as_rule(), Rule::improper_list);
        let mut items = pair.into_inner();
        let prefix = items.next();
        let mut list_items = Vec::new();
        if let Some(prefix) = prefix {
            if matches!(prefix.as_rule(), Rule::list_prefix) {
                for item in prefix.into_inner() {
                    if matches!(item.as_rule(), Rule::term) {
                        list_items.push(Self::build_term(vec![item])?);
                    }
                }
            } else if improper {
                return Err(ZlfError::SyntaxError(0, "invalid list prefix".to_string()));
            }
        }
        if !improper {
            return Ok(Term::List(list_items));
        }
        let tail = items
            .next()
            .ok_or_else(|| ZlfError::SyntaxError(0, "missing list tail".to_string()))?;
        Ok(crate::parser_helpers::canonical_list(
            list_items,
            Self::build_term(vec![tail])?,
        ))
    }
}
