use std::collections::HashMap;

use zlf_core::Result;
use crate::parser::{Term, PrologRule, Query};

#[derive(Debug, Clone)]
pub enum Instruction {
    // Put instructions
    PutVariable(String),
    PutValue(String),
    PutConstant(Term),
    
    // Get instructions
    GetVariable(String),
    GetValue(String),
    GetConstant(Term),
    
    // Unify
    Unify,
    
    // Call
    Call(String),
    
    // Proceed
    Proceed,
    
    // Choice
    Trust,
    Retry,
    Cut,
    
    // Index
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct WAM {
    // Heap for terms
    heap: Vec<Term>,
    
    // Stack for frames
    stack: Vec<Frame>,
    
    // Trail for backtracking
    trail: Vec<TrailEntry>,
    
    // Choice points
    choice_points: Vec<ChoicePoint>,
    
    // Current environment
    env: HashMap<String, usize>,
    
    // Registers
    registers: Vec<Option<Term>>,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub locals: HashMap<String, usize>,
    pub continuation: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct TrailEntry {
    pub address: usize,
    pub value: Term,
}

#[derive(Debug, Clone)]
pub struct ChoicePoint {
    pub heap_pointer: usize,
    pub stack_pointer: usize,
    pub trail_pointer: usize,
    pub env: HashMap<String, usize>,
    pub instructions: Vec<Instruction>,
}

impl WAM {
    pub fn new() -> Self {
        Self {
            heap: Vec::new(),
            stack: Vec::new(),
            trail: Vec::new(),
            choice_points: Vec::new(),
            env: HashMap::new(),
            registers: vec![None; 16], // 16 registers
        }
    }

    pub fn execute(&mut self, query: &Query, rules: &[PrologRule]) -> Result<Vec<HashMap<String, Term>>> {
        match query {
            Query::Goal(term) => {
                self.execute_goal(term, rules)
            }
            Query::RuleDef(_rule) => {
                // Store rule for later use
                Ok(vec![])
            }
        }
    }

    fn execute_goal(&mut self, term: &Term, rules: &[PrologRule]) -> Result<Vec<HashMap<String, Term>>> {
        let mut results = Vec::new();
        
        // Try to match the goal with rules
        for rule in rules {
            if let Some(bindings) = self.match_term(term, &rule.head)? {
                // Execute rule body
                let mut body_results = Vec::new();
                for body_term in &rule.body {
                    let body_bindings = self.execute_goal(body_term, rules)?;
                    body_results.extend(body_bindings);
                }
                
                // Combine bindings
                if body_results.is_empty() {
                    results.push(bindings);
                } else {
                    for mut body_binding in body_results {
                        for (key, value) in &bindings {
                            body_binding.insert(key.clone(), value.clone());
                        }
                        results.push(body_binding);
                    }
                }
            }
        }
        
        // If no rules match, try to find direct matches in the database
        if results.is_empty() {
            if let Some(direct_match) = self.match_database(term)? {
                results.push(direct_match);
            }
        }
        
        Ok(results)
    }

    fn match_term(&self, pattern: &Term, candidate: &Term) -> Result<Option<HashMap<String, Term>>> {
        let mut bindings = HashMap::new();
        
        match (pattern, candidate) {
            (Term::Variable(name), _) => {
                bindings.insert(name.clone(), candidate.clone());
                Ok(Some(bindings))
            }
            (Term::Atom(a), Term::Atom(b)) => {
                if a == b {
                    Ok(Some(bindings))
                } else {
                    Ok(None)
                }
            }
            (Term::Number(a), Term::Number(b)) => {
                if (a - b).abs() < f64::EPSILON {
                    Ok(Some(bindings))
                } else {
                    Ok(None)
                }
            }
            (Term::String(a), Term::String(b)) => {
                if a == b {
                    Ok(Some(bindings))
                } else {
                    Ok(None)
                }
            }
            (Term::Compound { name: n1, args: a1 }, Term::Compound { name: n2, args: a2 }) => {
                if n1 != n2 || a1.len() != a2.len() {
                    return Ok(None);
                }
                
                for (p, c) in a1.iter().zip(a2.iter()) {
                    if let Some(sub_bindings) = self.match_term(p, c)? {
                        bindings.extend(sub_bindings);
                    } else {
                        return Ok(None);
                    }
                }
                
                Ok(Some(bindings))
            }
            (Term::List(l1), Term::List(l2)) => {
                if l1.len() != l2.len() {
                    return Ok(None);
                }
                
                for (p, c) in l1.iter().zip(l2.iter()) {
                    if let Some(sub_bindings) = self.match_term(p, c)? {
                        bindings.extend(sub_bindings);
                    } else {
                        return Ok(None);
                    }
                }
                
                Ok(Some(bindings))
            }
            _ => Ok(None),
        }
    }

    fn match_database(&self, _term: &Term) -> Result<Option<HashMap<String, Term>>> {
        // This would normally query the graph database
        // For now, return None (no direct match)
        Ok(None)
    }

    pub fn unify(&mut self, term1: &Term, term2: &Term) -> Result<bool> {
        match (term1, term2) {
            (Term::Variable(_), _) | (_, Term::Variable(_)) => {
                // Variables always unify with anything
                Ok(true)
            }
            (Term::Atom(a), Term::Atom(b)) => Ok(a == b),
            (Term::Number(a), Term::Number(b)) => Ok((a - b).abs() < f64::EPSILON),
            (Term::String(a), Term::String(b)) => Ok(a == b),
            (Term::Compound { name: n1, args: a1 }, Term::Compound { name: n2, args: a2 }) => {
                if n1 != n2 || a1.len() != a2.len() {
                    return Ok(false);
                }
                
                for (t1, t2) in a1.iter().zip(a2.iter()) {
                    if !self.unify(t1, t2)? {
                        return Ok(false);
                    }
                }
                
                Ok(true)
            }
            (Term::List(l1), Term::List(l2)) => {
                if l1.len() != l2.len() {
                    return Ok(false);
                }
                
                for (t1, t2) in l1.iter().zip(l2.iter()) {
                    if !self.unify(t1, t2)? {
                        return Ok(false);
                    }
                }
                
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn reset(&mut self) {
        self.heap.clear();
        self.stack.clear();
        self.trail.clear();
        self.choice_points.clear();
        self.env.clear();
        self.registers.iter_mut().for_each(|r| *r = None);
    }
}

impl Default for WAM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::PrologParser;

    #[test]
    fn test_wam_creation() {
        let wam = WAM::new();
        assert!(wam.heap.is_empty());
        assert!(wam.stack.is_empty());
    }

    #[test]
    fn test_unify_atoms() {
        let mut wam = WAM::new();
        
        let t1 = Term::Atom("alice".to_string());
        let t2 = Term::Atom("alice".to_string());
        
        assert!(wam.unify(&t1, &t2).unwrap());
        
        let t3 = Term::Atom("bob".to_string());
        assert!(!wam.unify(&t1, &t3).unwrap());
    }

    #[test]
    fn test_unify_variables() {
        let mut wam = WAM::new();
        
        let t1 = Term::Variable("X".to_string());
        let t2 = Term::Atom("alice".to_string());
        
        assert!(wam.unify(&t1, &t2).unwrap());
        assert!(wam.unify(&t2, &t1).unwrap());
    }

    #[test]
    fn test_unify_compound() {
        let mut wam = WAM::new();
        
        let t1 = Term::Compound {
            name: "knows".to_string(),
            args: vec![
                Term::Variable("X".to_string()),
                Term::Atom("bob".to_string()),
            ],
        };
        
        let t2 = Term::Compound {
            name: "knows".to_string(),
            args: vec![
                Term::Atom("alice".to_string()),
                Term::Atom("bob".to_string()),
            ],
        };
        
        assert!(wam.unify(&t1, &t2).unwrap());
    }

    #[test]
    fn test_match_term() {
        let wam = WAM::new();
        
        let pattern = Term::Compound {
            name: "knows".to_string(),
            args: vec![
                Term::Variable("X".to_string()),
                Term::Atom("bob".to_string()),
            ],
        };
        
        let candidate = Term::Compound {
            name: "knows".to_string(),
            args: vec![
                Term::Atom("alice".to_string()),
                Term::Atom("bob".to_string()),
            ],
        };
        
        let bindings = wam.match_term(&pattern, &candidate).unwrap();
        assert!(bindings.is_some());
        assert_eq!(bindings.unwrap().get("X"), Some(&Term::Atom("alice".to_string())));
    }

    #[test]
    fn test_execute_goal_with_facts() {
        let mut wam = WAM::new();
        
        // Define facts as rules with empty bodies
        // knows(alice, bob).
        // knows(bob, charlie).
        let rules = vec![
            PrologRule {
                head: Term::Compound {
                    name: "knows".to_string(),
                    args: vec![
                        Term::Atom("alice".to_string()),
                        Term::Atom("bob".to_string()),
                    ],
                },
                body: vec![],
            },
            PrologRule {
                head: Term::Compound {
                    name: "knows".to_string(),
                    args: vec![
                        Term::Atom("bob".to_string()),
                        Term::Atom("charlie".to_string()),
                    ],
                },
                body: vec![],
            },
        ];
        
        // Query: ?knows(alice, X).
        let query = PrologParser::parse_query("?knows(alice, X).").unwrap();
        let results = wam.execute(&query, &rules).unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get("X"), Some(&Term::Atom("bob".to_string())));
    }

    #[test]
    fn test_execute_goal_with_rule() {
        let mut wam = WAM::new();
        
        // Define facts
        let rules = vec![
            // knows(alice, bob).
            PrologRule {
                head: Term::Compound {
                    name: "knows".to_string(),
                    args: vec![
                        Term::Atom("alice".to_string()),
                        Term::Atom("bob".to_string()),
                    ],
                },
                body: vec![],
            },
            // knows(bob, charlie).
            PrologRule {
                head: Term::Compound {
                    name: "knows".to_string(),
                    args: vec![
                        Term::Atom("bob".to_string()),
                        Term::Atom("charlie".to_string()),
                    ],
                },
                body: vec![],
            },
            // friend(X, Y) :- knows(X, Y), knows(Y, X).
            PrologRule {
                head: Term::Compound {
                    name: "friend".to_string(),
                    args: vec![
                        Term::Variable("X".to_string()),
                        Term::Variable("Y".to_string()),
                    ],
                },
                body: vec![
                    Term::Compound {
                        name: "knows".to_string(),
                        args: vec![
                            Term::Variable("X".to_string()),
                            Term::Variable("Y".to_string()),
                        ],
                    },
                    Term::Compound {
                        name: "knows".to_string(),
                        args: vec![
                            Term::Variable("Y".to_string()),
                            Term::Variable("X".to_string()),
                        ],
                    },
                ],
            },
        ];
        
        // Query: ?knows(alice, X).
        let query = PrologParser::parse_query("?knows(alice, X).").unwrap();
        let results = wam.execute(&query, &rules).unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get("X"), Some(&Term::Atom("bob".to_string())));
    }

    #[test]
    fn test_execute_goal_no_match() {
        let mut wam = WAM::new();
        
        // Define facts
        let rules = vec![
            PrologRule {
                head: Term::Compound {
                    name: "knows".to_string(),
                    args: vec![
                        Term::Atom("alice".to_string()),
                        Term::Atom("bob".to_string()),
                    ],
                },
                body: vec![],
            },
        ];
        
        // Query: ?knows(alice, charlie). - should not match
        let query = PrologParser::parse_query("?knows(alice, charlie).").unwrap();
        let results = wam.execute(&query, &rules).unwrap();
        
        assert!(results.is_empty());
    }

    #[test]
    fn test_execute_goal_multiple_results() {
        let mut wam = WAM::new();
        
        // Define facts
        let rules = vec![
            PrologRule {
                head: Term::Compound {
                    name: "knows".to_string(),
                    args: vec![
                        Term::Atom("alice".to_string()),
                        Term::Atom("bob".to_string()),
                    ],
                },
                body: vec![],
            },
            PrologRule {
                head: Term::Compound {
                    name: "knows".to_string(),
                    args: vec![
                        Term::Atom("alice".to_string()),
                        Term::Atom("charlie".to_string()),
                    ],
                },
                body: vec![],
            },
        ];
        
        // Query: ?knows(alice, X). - should return both bob and charlie
        let query = PrologParser::parse_query("?knows(alice, X).").unwrap();
        let results = wam.execute(&query, &rules).unwrap();
        
        assert_eq!(results.len(), 2);
        let names: Vec<&str> = results.iter()
            .filter_map(|r| r.get("X"))
            .filter_map(|t| t.as_atom())
            .collect();
        assert!(names.contains(&"bob"));
        assert!(names.contains(&"charlie"));
    }
}
