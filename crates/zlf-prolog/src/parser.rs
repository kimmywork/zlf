#![allow(
    clippy::never_loop,
    clippy::single_match,
    clippy::while_let_on_iterator
)]

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};

use zlf_core::{Result, ZlfError};

#[derive(Parser)]
#[grammar = "prolog.pest"]
pub struct PrologParser;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Term {
    Variable(String),
    Atom(String),
    Number(f64),
    String(String),
    Compound { name: String, args: Vec<Term> },
    List(Vec<Term>),
    Object(Vec<(String, Term)>),
}

impl Term {
    pub fn predicate_name(&self) -> String {
        match self {
            Term::Compound { name, .. } => name.clone(),
            Term::Atom(name) => name.clone(),
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fact {
    pub head: Term,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrologRule {
    pub head: Term,
    pub body: Vec<Term>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Query {
    Goal(Term),
    Goals(Vec<Term>),
    RuleDef(PrologRule),
}

impl PrologParser {
    pub fn parse_term(input: &str) -> Result<Term> {
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

    pub fn parse_query(input: &str) -> Result<Query> {
        let input = input.trim();

        // Try to parse as query first (with ? prefix)
        if input.starts_with('?') {
            let pairs = PrologParser::parse(Rule::query, input)
                .map_err(|e| ZlfError::SyntaxError(0, e.to_string()))?;

            let mut terms = Vec::new();
            for pair in pairs {
                match pair.as_rule() {
                    Rule::query => {
                        for inner in pair.into_inner() {
                            if inner.as_rule() == Rule::body {
                                for body_inner in inner.into_inner() {
                                    if body_inner.as_rule() == Rule::term {
                                        terms.push(Self::build_term(vec![body_inner])?);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            if terms.len() == 1 {
                return Ok(Query::Goal(terms.remove(0)));
            } else if !terms.is_empty() {
                return Ok(Query::Goals(terms));
            }
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
                Rule::number => {
                    let value: f64 = pair
                        .as_str()
                        .parse()
                        .map_err(|e| ZlfError::SyntaxError(0, format!("Invalid number: {}", e)))?;
                    return Ok(Term::Number(value));
                }
                Rule::string => {
                    let content = pair.as_str();
                    let content = &content[1..content.len() - 1]; // Remove quotes
                    return Ok(Term::String(content.to_string()));
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
                    let name = inner.next().unwrap().as_str().to_string();
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
                    let mut items = Vec::new();
                    for inner in pair.into_inner() {
                        match inner.as_rule() {
                            Rule::term => items.push(Self::build_term(vec![inner])?),
                            Rule::list_items => {
                                for item in inner.into_inner() {
                                    if matches!(item.as_rule(), Rule::term) {
                                        items.push(Self::build_term(vec![item])?);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    return Ok(Term::List(items));
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
}

impl Term {
    pub fn is_variable(&self) -> bool {
        matches!(self, Term::Variable(_))
    }

    pub fn is_atom(&self) -> bool {
        matches!(self, Term::Atom(_))
    }

    pub fn as_variable(&self) -> Option<&str> {
        match self {
            Term::Variable(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_atom(&self) -> Option<&str> {
        match self {
            Term::Atom(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Term::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Term::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_compound(&self) -> Option<(&str, &[Term])> {
        match self {
            Term::Compound { name, args } => Some((name, args)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_variable() {
        let term = PrologParser::parse_term("X").unwrap();
        assert!(term.is_variable());
        assert_eq!(term.as_variable(), Some("X"));
    }

    #[test]
    fn test_parse_atom() {
        let term = PrologParser::parse_term("alice").unwrap();
        assert!(term.is_atom());
        assert_eq!(term.as_atom(), Some("alice"));
    }

    #[test]
    fn test_parse_number() {
        let term = PrologParser::parse_term("42").unwrap();
        assert_eq!(term.as_number(), Some(42.0));
    }

    #[test]
    fn test_parse_string() {
        let term = PrologParser::parse_term("\"hello\"").unwrap();
        assert_eq!(term.as_string(), Some("hello"));
    }

    #[test]
    fn test_parse_compound() {
        let term = PrologParser::parse_term("knows(alice, bob)").unwrap();
        let (name, args) = term.as_compound().unwrap();
        assert_eq!(name, "knows");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_parse_fact() {
        let fact = PrologParser::parse_fact("node(person, alice).").unwrap();
        let (name, args) = fact.head.as_compound().unwrap();
        assert_eq!(name, "node");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_parse_rule() {
        let rule =
            PrologParser::parse_rule("colleague(X, Y) :- works_at(X, C), works_at(Y, C).").unwrap();
        let (name, args) = rule.head.as_compound().unwrap();
        assert_eq!(name, "colleague");
        assert_eq!(args.len(), 2);
        assert_eq!(rule.body.len(), 2);
    }

    #[test]
    fn test_parse_query() {
        let query = PrologParser::parse_query("?colleague(alice, Who).").unwrap();
        match query {
            Query::Goal(term) => {
                let (name, args) = term.as_compound().unwrap();
                assert_eq!(name, "colleague");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Goal"),
        }
    }

    #[test]
    fn test_parse_multi_goal_query() {
        let query =
            PrologParser::parse_query("?works_at(alice, C), prop(C, name, \"ACME\").").unwrap();
        match query {
            Query::Goals(terms) => {
                assert_eq!(terms.len(), 2);
                assert_eq!(terms[0].predicate_name(), "works_at");
                assert_eq!(terms[1].predicate_name(), "prop");
            }
            _ => panic!("Expected Goals"),
        }
    }

    #[test]
    fn test_parse_inequality() {
        let term = PrologParser::parse_term("X \\= Y").unwrap();
        let (name, args) = term.as_compound().unwrap();
        assert_eq!(name, "\\=");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_parse_recursive_rule() {
        let rule = PrologParser::parse_rule("ancestor(X, Y) :- parent(X, Y).").unwrap();
        let (name, args) = rule.head.as_compound().unwrap();
        assert_eq!(name, "ancestor");
        assert_eq!(args.len(), 2);
        assert_eq!(rule.body.len(), 1);
    }

    #[test]
    fn test_parse_rule_with_multiple_clauses() {
        let rule =
            PrologParser::parse_rule("colleague(X, Y) :- works_at(X, C), works_at(Y, C).").unwrap();
        let (name, args) = rule.head.as_compound().unwrap();
        assert_eq!(name, "colleague");
        assert_eq!(args.len(), 2);
        assert_eq!(rule.body.len(), 2);
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let result = PrologParser::parse_fact("invalid syntax");
        assert!(result.is_err());
    }
}
