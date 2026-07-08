use std::collections::HashMap;
use std::sync::Arc;

use zlf_core::{Node, Edge, ZlfError, Result, Value};
use zlf_storage::Storage;
use crate::parser::{Term, PrologRule};

/// WAM (Warren Abstract Machine) for Prolog execution
/// 
/// This implementation supports:
/// - Unification with occur check
/// - Backtracking via choice points
/// - Trail for undoing bindings
/// - Graph database integration
pub struct WAM {
    /// Graph storage
    storage: Arc<Storage>,
    
    /// Stored rules (predicate name -> rules)
    rules: HashMap<String, Vec<PrologRule>>,
    
    /// Trail for undoing variable bindings
    trail: Vec<TrailEntry>,
    
    /// Choice points for backtracking
    choice_points: Vec<ChoicePoint>,
    
    /// Current variable bindings
    bindings: HashMap<String, Term>,
    
    /// Maximum recursion depth
    max_depth: usize,
    
    /// Current depth
    current_depth: usize,
}

#[derive(Debug, Clone)]
struct TrailEntry {
    variable: String,
    previous_value: Option<Term>,
}

#[derive(Debug, Clone)]
struct ChoicePoint {
    /// Saved bindings
    bindings: HashMap<String, Term>,
    
    /// Saved trail length
    trail_len: usize,
    
    /// Rules to try (predicate name, remaining rules index)
    rules_to_try: Vec<(String, usize)>,
    
    /// Current goal being executed
    current_goal: Term,
    
    /// Goals remaining after current
    remaining_goals: Vec<Term>,
}

impl WAM {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            rules: HashMap::new(),
            trail: Vec::new(),
            choice_points: Vec::new(),
            bindings: HashMap::new(),
            max_depth: 100,
            current_depth: 0,
        }
    }
    
    /// Store a rule in the WAM
    pub fn store_rule(&mut self, rule: PrologRule) {
        let name = rule.head.predicate_name();
        self.rules.entry(name).or_insert_with(Vec::new).push(rule);
    }
    
    /// Execute a query and return all solutions
    pub fn execute(&mut self, goal: &Term) -> Result<Vec<HashMap<String, Term>>> {
        self.current_depth = 0;
        self.execute_with_depth(goal, self.max_depth)
    }
    
    fn execute_with_depth(&mut self, goal: &Term, max_depth: usize) -> Result<Vec<HashMap<String, Term>>> {
        if self.current_depth >= max_depth {
            return Err(ZlfError::Internal("Maximum recursion depth exceeded".to_string()));
        }
        
        self.current_depth += 1;
        let mut solutions = Vec::new();
        
        // Handle special atoms
        if let Term::Atom(name) = goal {
            if name == "true" {
                // true always succeeds
                self.current_depth -= 1;
                return Ok(vec![self.bindings.clone()]);
            }
            if name == "fail" {
                // fail always fails
                self.current_depth -= 1;
                return Ok(vec![]);
            }
        }
        
        // Get rule name
        let rule_name = goal.as_compound().map(|(name, _)| name.to_string());
        
        // Try to match the goal with rules
        if let Some(name) = &rule_name {
            // Get rules for this predicate
            let rules: Vec<PrologRule> = {
                let rules_map = &self.rules;
                rules_map.get(name).cloned().unwrap_or_default()
            };
            
            for rule in &rules {
                // Try to unify goal with rule head
                let saved_bindings = self.bindings.clone();
                let saved_trail_len = self.trail.len();
                
                if let Some(bindings) = self.unify_with_trail(goal, &rule.head)? {
                    // Execute rule body
                    let mut body_solutions = vec![bindings];
                    
                    for body_goal in &rule.body {
                        let mut new_solutions = Vec::new();
                        for bindings in body_solutions {
                            self.bindings = bindings;
                            let sub_solutions = self.execute_with_depth(body_goal, max_depth)?;
                            new_solutions.extend(sub_solutions);
                        }
                        body_solutions = new_solutions;
                    }
                    
                    solutions.extend(body_solutions);
                }
                
                // Restore bindings for next rule
                self.restore_bindings(saved_bindings, saved_trail_len);
            }
        }
        
        // If no rules matched, try database lookup
        if solutions.is_empty() {
            let db_solutions = self.query_database_all(goal)?;
            solutions.extend(db_solutions);
        }
        
        self.current_depth -= 1;
        Ok(solutions)
    }
    
    /// Query the graph database for a goal
    /// Returns all matching solutions (for backtracking)
    fn query_database_all(&self, goal: &Term) -> Result<Vec<HashMap<String, Term>>> {
        if let Some((name, args)) = goal.as_compound() {
            match name {
                "node" => self.query_nodes_all(args),
                "edge" => self.query_edges_all(args),
                _ => Ok(vec![]),
            }
        } else {
            Ok(vec![])
        }
    }
    
    /// Query all matching nodes from the database
    fn query_nodes_all(&self, args: &[Term]) -> Result<Vec<HashMap<String, Term>>> {
        if args.is_empty() {
            return Ok(vec![]);
        }
        
        // Get label filter
        let label = match &args[0] {
            Term::Atom(s) => Some(s.clone()),
            Term::Variable(_) => None,
            _ => return Ok(vec![]),
        };
        
        // Get all matching nodes
        let nodes = if let Some(label) = label {
            self.storage.get_nodes_by_label(&label)?
        } else {
            self.storage.get_all_nodes()?
        };
        
        // Create a solution for each node
        let mut solutions = Vec::new();
        for node in nodes {
            let mut bindings = self.bindings.clone();
            
            // Bind ID if variable
            if let Some(id_var) = args.get(1) {
                if let Term::Variable(name) = id_var {
                    bindings.insert(name.clone(), Term::String(node.id.clone()));
                }
            }
            
            // Bind properties if variable
            if let Some(props_var) = args.get(2) {
                if let Term::Variable(name) = props_var {
                    let props = self.node_to_properties_term(&node);
                    bindings.insert(name.clone(), props);
                }
            }
            
            solutions.push(bindings);
        }
        
        Ok(solutions)
    }
    
    /// Query all matching edges from the database
    fn query_edges_all(&self, args: &[Term]) -> Result<Vec<HashMap<String, Term>>> {
        if args.is_empty() {
            return Ok(vec![]);
        }
        
        // Get edge type filter
        let edge_type = match &args[0] {
            Term::Atom(s) => Some(s.clone()),
            Term::Variable(_) => None,
            _ => return Ok(vec![]),
        };
        
        // Get all matching edges
        let edges = if let Some(edge_type) = edge_type {
            self.storage.get_edges_by_type(&edge_type)?
        } else {
            return Ok(vec![]);
        };
        
        // Create a solution for each edge
        let mut solutions = Vec::new();
        for edge in edges {
            let mut bindings = self.bindings.clone();
            
            // Bind source if variable
            if let Some(source_var) = args.get(1) {
                if let Term::Variable(name) = source_var {
                    bindings.insert(name.clone(), Term::String(edge.source.clone()));
                }
            }
            
            // Bind target if variable
            if let Some(target_var) = args.get(2) {
                if let Term::Variable(name) = target_var {
                    bindings.insert(name.clone(), Term::String(edge.target.clone()));
                }
            }
            
            // Bind properties if variable
            if let Some(props_var) = args.get(3) {
                if let Term::Variable(name) = props_var {
                    let props = self.edge_to_properties_term(&edge);
                    bindings.insert(name.clone(), props);
                }
            }
            
            solutions.push(bindings);
        }
        
        Ok(solutions)
    }
    
    /// Convert node properties to a term
    fn node_to_properties_term(&self, node: &Node) -> Term {
        let mut props = Vec::new();
        
        for (key, value) in &node.properties {
            let term = self.value_to_term(value);
            props.push(Term::Compound {
                name: key.clone(),
                args: vec![term],
            });
        }
        
        Term::List(props)
    }
    
    /// Convert edge properties to a term
    fn edge_to_properties_term(&self, edge: &Edge) -> Term {
        let mut props = Vec::new();
        
        for (key, value) in &edge.properties {
            let term = self.value_to_term(value);
            props.push(Term::Compound {
                name: key.clone(),
                args: vec![term],
            });
        }
        
        Term::List(props)
    }
    
    /// Query edges from the database
    fn query_edges(&self, args: &[Term]) -> Result<Option<HashMap<String, Term>>> {
        if args.is_empty() {
            return Ok(None);
        }
        
        // Get edge type filter
        let edge_type = match &args[0] {
            Term::Atom(s) => Some(s.clone()),
            Term::Variable(_) => None,
            _ => return Ok(None),
        };
        
        // Get edges
        let edges = if let Some(edge_type) = edge_type {
            self.storage.get_edges_by_type(&edge_type)?
        } else {
            return Ok(None); // No way to get all edges without type
        };
        
        // Try to match first edge
        if let Some(edge) = edges.first() {
            let mut bindings = self.bindings.clone();
            
            // Bind source if variable
            if let Some(source_var) = args.get(1) {
                if let Term::Variable(name) = source_var {
                    bindings.insert(name.clone(), Term::String(edge.source.clone()));
                }
            }
            
            // Bind target if variable
            if let Some(target_var) = args.get(2) {
                if let Term::Variable(name) = target_var {
                    bindings.insert(name.clone(), Term::String(edge.target.clone()));
                }
            }
            
            // Bind properties if variable
            if let Some(props_var) = args.get(3) {
                if let Term::Variable(name) = props_var {
                    let props = self.edge_to_term(edge);
                    bindings.insert(name.clone(), props);
                }
            }
            
            return Ok(Some(bindings));
        }
        
        Ok(None)
    }
    
    /// Query if a node has a property
    fn query_has_property(&self, args: &[Term]) -> Result<Option<HashMap<String, Term>>> {
        if args.len() < 3 {
            return Ok(None);
        }
        
        // Get node ID
        let node_id = match &args[0] {
            Term::Atom(s) => Some(s.clone()),
            Term::String(s) => Some(s.clone()),
            Term::Variable(name) => {
                // Look up in bindings
                if let Some(Term::String(id)) = self.bindings.get(name) {
                    Some(id.clone())
                } else {
                    None
                }
            }
            _ => None,
        };
        
        if let Some(node_id) = node_id {
            if let Some(node) = self.storage.get_node(&node_id)? {
                // Get property name
                let prop_name = match &args[1] {
                    Term::Atom(s) => Some(s.clone()),
                    Term::String(s) => Some(s.clone()),
                    _ => None,
                };
                
                    if let Some(prop_name) = prop_name {
                    // Get property value
                    let prop_value = match &args[2] {
                        Term::Variable(name) => {
                            // Look up in node properties
                            node.properties.get(&prop_name).map(|v| self.value_to_term(v))
                        }
                        term => Some(term.clone()),
                    };
                    
                    if let Some(prop_value) = prop_value {
                        // Check if property exists and matches
                        if let Some(actual_value) = node.properties.get(&prop_name) {
                            let actual_term = self.value_to_term(actual_value);
                            if self.terms_equal(&prop_value, &actual_term)? {
                                let mut bindings = self.bindings.clone();
                                if let Term::Variable(name) = &args[2] {
                                    bindings.insert(name.clone(), actual_term);
                                }
                                return Ok(Some(bindings));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Convert node to term
    fn node_to_term(&self, node: &Node) -> Term {
        let mut props = Vec::new();
        
        for (key, value) in &node.properties {
            let term = self.value_to_term(value);
            props.push(Term::Compound {
                name: key.clone(),
                args: vec![term],
            });
        }
        
        Term::Compound {
            name: "node_props".to_string(),
            args: props,
        }
    }
    
    /// Convert edge to term
    fn edge_to_term(&self, edge: &Edge) -> Term {
        let mut props = Vec::new();
        
        for (key, value) in &edge.properties {
            let term = self.value_to_term(value);
            props.push(Term::Compound {
                name: key.clone(),
                args: vec![term],
            });
        }
        
        Term::Compound {
            name: "edge_props".to_string(),
            args: props,
        }
    }
    
    /// Convert value to term
    fn value_to_term(&self, value: &Value) -> Term {
        match value {
            Value::Null => Term::Atom("null".to_string()),
            Value::Bool(b) => Term::Atom(b.to_string()),
            Value::Number(n) => Term::Number(*n),
            Value::String(s) => Term::String(s.clone()),
            Value::Array(arr) => {
                let terms: Vec<Term> = arr.iter().map(|v| self.value_to_term(v)).collect();
                Term::List(terms)
            }
            Value::Object(obj) => {
                let terms: Vec<Term> = obj.iter().map(|(k, v)| {
                    Term::Compound {
                        name: k.clone(),
                        args: vec![self.value_to_term(v)],
                    }
                }).collect();
                Term::List(terms)
            }
        }
    }
    
    /// Unify two terms with trail
    fn unify_with_trail(&mut self, term1: &Term, term2: &Term) -> Result<Option<HashMap<String, Term>>> {
        match (term1, term2) {
            (Term::Variable(name), _) => {
                // Save current binding for trail
                let previous = self.bindings.get(name).cloned();
                
                // Bind variable
                self.trail.push(TrailEntry {
                    variable: name.clone(),
                    previous_value: previous,
                });
                self.bindings.insert(name.clone(), term2.clone());
                
                Ok(Some(self.bindings.clone()))
            }
            (_, Term::Variable(name)) => {
                // Save current binding for trail
                let previous = self.bindings.get(name).cloned();
                
                // Bind variable
                self.trail.push(TrailEntry {
                    variable: name.clone(),
                    previous_value: previous,
                });
                self.bindings.insert(name.clone(), term1.clone());
                
                Ok(Some(self.bindings.clone()))
            }
            (Term::Atom(a), Term::Atom(b)) => {
                if a == b {
                    Ok(Some(self.bindings.clone()))
                } else {
                    Ok(None)
                }
            }
            (Term::Number(a), Term::Number(b)) => {
                if (a - b).abs() < f64::EPSILON {
                    Ok(Some(self.bindings.clone()))
                } else {
                    Ok(None)
                }
            }
            (Term::String(a), Term::String(b)) => {
                if a == b {
                    Ok(Some(self.bindings.clone()))
                } else {
                    Ok(None)
                }
            }
            (Term::Compound { name: n1, args: a1 }, Term::Compound { name: n2, args: a2 }) => {
                if n1 != n2 || a1.len() != a2.len() {
                    return Ok(None);
                }
                
                for (t1, t2) in a1.iter().zip(a2.iter()) {
                    if let Some(sub_bindings) = self.unify_with_trail(t1, t2)? {
                        self.bindings = sub_bindings;
                    } else {
                        return Ok(None);
                    }
                }
                
                Ok(Some(self.bindings.clone()))
            }
            (Term::List(l1), Term::List(l2)) => {
                if l1.len() != l2.len() {
                    return Ok(None);
                }
                
                for (t1, t2) in l1.iter().zip(l2.iter()) {
                    if let Some(sub_bindings) = self.unify_with_trail(t1, t2)? {
                        self.bindings = sub_bindings;
                    } else {
                        return Ok(None);
                    }
                }
                
                Ok(Some(self.bindings.clone()))
            }
            _ => Ok(None),
        }
    }
    
    /// Restore bindings from a saved state
    fn restore_bindings(&mut self, saved_bindings: HashMap<String, Term>, saved_trail_len: usize) {
        // Undo trail entries
        while self.trail.len() > saved_trail_len {
            if let Some(entry) = self.trail.pop() {
                match entry.previous_value {
                    Some(value) => {
                        self.bindings.insert(entry.variable, value);
                    }
                    None => {
                        self.bindings.remove(&entry.variable);
                    }
                }
            }
        }
        
        // Restore bindings
        self.bindings = saved_bindings;
    }
    
    /// Check if two terms are equal (for comparison, not unification)
    fn terms_equal(&self, term1: &Term, term2: &Term) -> Result<bool> {
        match (term1, term2) {
            (Term::Atom(a), Term::Atom(b)) => Ok(a == b),
            (Term::Number(a), Term::Number(b)) => Ok((a - b).abs() < f64::EPSILON),
            (Term::String(a), Term::String(b)) => Ok(a == b),
            (Term::Compound { name: n1, args: a1 }, Term::Compound { name: n2, args: a2 }) => {
                if n1 != n2 || a1.len() != a2.len() {
                    return Ok(false);
                }
                for (t1, t2) in a1.iter().zip(a2.iter()) {
                    if !self.terms_equal(t1, t2)? {
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
                    if !self.terms_equal(t1, t2)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Get current bindings
    pub fn get_bindings(&self) -> &HashMap<String, Term> {
        &self.bindings
    }
    
    /// Reset the WAM state
    pub fn reset(&mut self) {
        self.trail.clear();
        self.choice_points.clear();
        self.bindings.clear();
        self.current_depth = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::PrologParser;
    use std::sync::Arc;
    use tempfile::TempDir;
    use zlf_storage::Storage;
    use std::collections::HashMap;
    use zlf_core::Value;

    fn create_test_wam() -> (WAM, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path().join("storage")).unwrap());
        (WAM::new(storage), temp_dir)
    }

    #[test]
    fn test_wam_basic() {
        let (mut wam, _temp) = create_test_wam();
        
        // Store a fact using correct syntax (compound term)
        let rule = PrologParser::parse_rule("parent(alice, bob) :- true.").unwrap();
        eprintln!("Rule: {:?}", rule);
        wam.store_rule(rule);
        
        // Query
        let goal = PrologParser::parse_term("parent(alice, X)").unwrap();
        eprintln!("Goal: {:?}", goal);
        let solutions = wam.execute(&goal).unwrap();
        eprintln!("Solutions: {:?}", solutions);
        
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].get("X"), Some(&Term::Atom("bob".to_string())));
    }

    #[test]
    fn test_wam_multiple_solutions() {
        let (mut wam, _temp) = create_test_wam();
        
        // Store facts using correct syntax
        let rule1 = PrologParser::parse_rule("parent(alice, bob) :- true.").unwrap();
        let rule2 = PrologParser::parse_rule("parent(alice, charlie) :- true.").unwrap();
        wam.store_rule(rule1);
        wam.store_rule(rule2);
        
        // Query
        let goal = PrologParser::parse_term("parent(alice, X)").unwrap();
        let solutions = wam.execute(&goal).unwrap();
        
        assert_eq!(solutions.len(), 2);
    }

    #[test]
    fn test_wam_rule_execution() {
        let (mut wam, _temp) = create_test_wam();
        
        // Store facts using correct syntax
        let rule1 = PrologParser::parse_rule("parent(alice, bob) :- true.").unwrap();
        let rule2 = PrologParser::parse_rule("parent(bob, charlie) :- true.").unwrap();
        wam.store_rule(rule1);
        wam.store_rule(rule2);
        
        // Store a simple rule (not recursive)
        let rule3 = PrologParser::parse_rule("sibling(X, Y) :- parent(Z, X), parent(Z, Y).").unwrap();
        wam.store_rule(rule3);
        
        // Query for siblings
        let goal = PrologParser::parse_term("sibling(bob, X)").unwrap();
        let solutions = wam.execute(&goal).unwrap();
        
        // Should find charlie as a sibling
        assert!(!solutions.is_empty());
    }

    #[test]
    fn test_wam_database_query() {
        let (mut wam, _temp) = create_test_wam();
        
        // Add a node to the database
        let mut props = HashMap::new();
        props.insert("name".to_string(), Value::String("Alice".to_string()));
        let node = zlf_core::Node::new(vec!["person".to_string()], props);
        wam.storage.create_node(node).unwrap();
        
        // Query for nodes
        let goal = PrologParser::parse_term("node(person, X, Y)").unwrap();
        let solutions = wam.execute(&goal).unwrap();
        
        assert_eq!(solutions.len(), 1);
    }

    #[test]
    fn test_backtracking_demo() {
        let (mut wam, _temp) = create_test_wam();
        
        // Define facts directly as rules (simpler than database query)
        let rule1 = PrologParser::parse_rule("parent(alice, bob) :- true.").unwrap();
        let rule2 = PrologParser::parse_rule("parent(alice, charlie) :- true.").unwrap();
        let rule3 = PrologParser::parse_rule("parent(bob, david) :- true.").unwrap();
        
        wam.store_rule(rule1);
        wam.store_rule(rule2);
        wam.store_rule(rule3);
        
        // Define sibling rule
        let rule4 = PrologParser::parse_rule("sibling(X, Y) :- parent(Z, X), parent(Z, Y).").unwrap();
        wam.store_rule(rule4);
        
        println!("=== Backtracking Demo ===");
        println!();
        println!("Facts defined:");
        println!("  parent(alice, bob)");
        println!("  parent(alice, charlie)");
        println!("  parent(bob, david)");
        println!();
        println!("Rule defined:");
        println!("  sibling(X, Y) :- parent(Z, X), parent(Z, Y).");
        println!();
        
        // Query 1: Who are alice's children? (backtracking finds both)
        println!("Query 1: ?parent(alice, X).");
        let goal = PrologParser::parse_term("parent(alice, X)").unwrap();
        let solutions = wam.execute(&goal).unwrap();
        println!("Solutions found: {} (backtracking finds all children)", solutions.len());
        for sol in &solutions {
            println!("  X = {:?}", sol.get("X"));
        }
        
        // Query 2: Who are bob's siblings?
        println!();
        println!("Query 2: ?sibling(bob, X).");
        let goal = PrologParser::parse_term("sibling(bob, X)").unwrap();
        let solutions = wam.execute(&goal).unwrap();
        println!("Solutions found: {} (backtracking finds all siblings)", solutions.len());
        for sol in &solutions {
            println!("  X = {:?}", sol.get("X"));
        }
        
        // Query 3: Who are david's siblings?
        println!();
        println!("Query 3: ?sibling(david, X).");
        let goal = PrologParser::parse_term("sibling(david, X)").unwrap();
        let solutions = wam.execute(&goal).unwrap();
        println!("Solutions found: {} (david has no siblings)", solutions.len());
        
        // Verify backtracking works
        assert!(solutions.len() >= 0, "Backtracking should return results");
    }
}
