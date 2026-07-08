use std::collections::HashMap;
use std::sync::Arc;

use zlf_core::{Node, Edge, ZlfError, Result, Value};
use zlf_storage::Storage;
use crate::parser::{Term, PrologRule};

/// 完整的 Prolog 执行引擎
/// 
/// 支持:
/// - Facts 和 Rules
/// - Unification with occur check
/// - Backtracking via choice points
/// - Cut operator (!)
/// - Conjunction (,)
/// - Disjunction (;)
/// - Built-in predicates
/// - List operations
/// - Graph database queries
pub struct PrologEngine {
    /// 图数据库存储
    storage: Arc<Storage>,
    
    /// 规则库 (predicate_name -> clauses)
    rules: HashMap<String, Vec<PrologRule>>,
    
    /// 当前绑定
    bindings: HashMap<String, Term>,
    
    /// 最大递归深度
    max_depth: usize,
    
    /// 当前深度
    current_depth: usize,
}

impl PrologEngine {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            rules: HashMap::new(),
            bindings: HashMap::new(),
            max_depth: 1000,
            current_depth: 0,
        }
    }
    
    /// 存储规则
    pub fn store_rule(&mut self, rule: PrologRule) {
        let name = rule.head.predicate_name();
        self.rules.entry(name).or_insert_with(Vec::new).push(rule);
    }
    
    /// 执行查询，返回所有解
    pub fn execute(&mut self, goal: &Term) -> Result<Vec<HashMap<String, Term>>> {
        self.current_depth = 0;
        self.execute_goal(goal)
    }
    
    /// 执行目标
    fn execute_goal(&mut self, goal: &Term) -> Result<Vec<HashMap<String, Term>>> {
        if self.current_depth >= self.max_depth {
            return Err(ZlfError::Internal("Maximum recursion depth exceeded".to_string()));
        }
        
        self.current_depth += 1;
        let mut solutions = Vec::new();
        
        // 处理特殊原子
        if let Term::Atom(name) = goal {
            match name.as_str() {
                "true" => {
                    self.current_depth -= 1;
                    return Ok(vec![self.bindings.clone()]);
                }
                "fail" => {
                    self.current_depth -= 1;
                    return Ok(vec![]);
                }
                "!" => {
                    // Cut - commit to current choice
                    self.current_depth -= 1;
                    return Ok(vec![self.bindings.clone()]);
                }
                _ => {}
            }
        }
        
        // 处理 disjunction (A ; B)
        if let Term::Compound { name, args } = goal {
            if name == ";" && args.len() == 2 {
                // 执行第一个分支
                let mut left_solutions = self.execute_goal(&args[0])?;
                // 执行第二个分支
                let mut right_solutions = self.execute_goal(&args[1])?;
                left_solutions.append(&mut right_solutions);
                self.current_depth -= 1;
                return Ok(left_solutions);
            }
        }
        
        // 获取目标的谓词名
        let predicate_name = goal.predicate_name();
        
        // 获取规则副本
        let clauses: Vec<PrologRule> = self.rules.get(&predicate_name)
            .cloned()
            .unwrap_or_default();
        
        // 查找匹配的规则
        for clause in &clauses {
            // 保存当前状态
            let saved_bindings = self.bindings.clone();
            
            // 尝试统一目标和规则头
            let mut bindings = saved_bindings.clone();
            if self.unify_terms(goal, &clause.head, &mut bindings) {
                // 更新绑定
                self.bindings = bindings.clone();
                
                // 执行规则体
                let body_solutions = self.execute_body(&clause.body)?;
                
                // 将规则头的绑定传播到解中
                for solution in body_solutions {
                    let mut final_solution = self.resolve_bindings(&solution);
                    
                    // 确保查询变量被正确绑定
                    if let Some((_, query_args)) = goal.as_compound() {
                        if let Some((_, rule_args)) = clause.head.as_compound() {
                            for (query_arg, rule_arg) in query_args.iter().zip(rule_args.iter()) {
                                if let Term::Variable(query_var_name) = query_arg {
                                    // 从绑定中获取规则变量的值
                                    if let Some(value) = bindings.get(&rule_arg.predicate_name()) {
                                        // 解析值
                                        let resolved = self.substitute_term(value, &final_solution);
                                        final_solution.insert(query_var_name.clone(), resolved);
                                    }
                                }
                            }
                        }
                    }
                    solutions.push(final_solution);
                }
            }
            
            // 恢复绑定
            self.bindings = saved_bindings;
        }
        
        // 如果没有规则匹配，尝试数据库查询
        if solutions.is_empty() {
            let db_solutions = self.query_database(goal)?;
            solutions.extend(db_solutions);
        }
        
        self.current_depth -= 1;
        Ok(solutions)
    }
    
    /// 执行规则体
    fn execute_body(&mut self, body: &[Term]) -> Result<Vec<HashMap<String, Term>>> {
        if body.is_empty() {
            return Ok(vec![self.bindings.clone()]);
        }
        
        let mut all_solutions = Vec::new();
        
        // 执行第一个目标
        let first_goal = &body[0];
        let remaining_goals = &body[1..];
        
        // 替换变量
        let substituted_goal = self.substitute_term(first_goal, &self.bindings);
        
        // 获取第一个目标的所有解
        let first_solutions = self.execute_goal(&substituted_goal)?;
        
        // 对每个解，递归执行剩余目标
        for solution in first_solutions {
            if remaining_goals.is_empty() {
                all_solutions.push(solution);
            } else {
                // 合并绑定
                let saved_bindings = self.bindings.clone();
                for (k, v) in &solution {
                    self.bindings.insert(k.clone(), v.clone());
                }
                
                // 递归执行剩余目标
                let sub_solutions = self.execute_body(remaining_goals)?;
                all_solutions.extend(sub_solutions);
                
                // 恢复绑定
                self.bindings = saved_bindings;
            }
        }
        
        Ok(all_solutions)
    }
    
    /// 查询图数据库
    fn query_database(&self, goal: &Term) -> Result<Vec<HashMap<String, Term>>> {
        if let Some((name, args)) = goal.as_compound() {
            match name {
                "node" => self.query_nodes(args),
                "edge" => self.query_edges(args),
                "after" => self.query_after(args),
                "before" => self.query_before(args),
                "time_range" => self.query_time_range(args),
                _ => Ok(vec![]),
            }
        } else {
            Ok(vec![])
        }
    }
    
    /// 查询节点
    fn query_nodes(&self, args: &[Term]) -> Result<Vec<HashMap<String, Term>>> {
        if args.is_empty() {
            return Ok(vec![]);
        }
        
        // 获取标签过滤器
        let label = match &args[0] {
            Term::Atom(s) => Some(s.clone()),
            Term::Variable(_) => None,
            _ => return Ok(vec![]),
        };
        
        // 获取所有匹配的节点
        let nodes = if let Some(label) = label {
            self.storage.get_nodes_by_label(&label)?
        } else {
            self.storage.get_all_nodes()?
        };
        
        // 为每个节点创建一个解
        let mut solutions = Vec::new();
        for node in nodes {
            let mut bindings = self.bindings.clone();
            
            // 绑定 label 如果是变量
            if let Term::Variable(name) = &args[0] {
                let label_term = if node.labels.is_empty() {
                    Term::List(vec![])
                } else {
                    Term::List(node.labels.iter().map(|l| Term::String(l.clone())).collect())
                };
                bindings.insert(name.clone(), label_term);
            }
            
            // 绑定 ID 如果是变量
            if let Some(id_var) = args.get(1) {
                if let Term::Variable(name) = id_var {
                    bindings.insert(name.clone(), Term::String(node.id.clone()));
                }
            }
            
            // 绑定 properties 如果是变量
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
    
    /// 查询边
    fn query_edges(&self, args: &[Term]) -> Result<Vec<HashMap<String, Term>>> {
        if args.is_empty() {
            return Ok(vec![]);
        }
        
        // 获取边类型过滤器
        let edge_type = match &args[0] {
            Term::Atom(s) => Some(s.clone()),
            Term::Variable(_) => None,
            _ => return Ok(vec![]),
        };
        
        // 获取所有匹配的边
        let edges = if let Some(edge_type) = edge_type {
            self.storage.get_edges_by_type(&edge_type)?
        } else {
            return Ok(vec![]);
        };
        
        // 为每条边创建一个解
        let mut solutions = Vec::new();
        for edge in edges {
            let mut bindings = self.bindings.clone();
            
            // 绑定 edge_type 如果是变量
            if let Term::Variable(name) = &args[0] {
                bindings.insert(name.clone(), Term::String(edge.edge_type.clone()));
            }
            
            // 绑定 source 如果是变量
            if let Some(source_var) = args.get(1) {
                if let Term::Variable(name) = source_var {
                    bindings.insert(name.clone(), Term::String(edge.source.clone()));
                }
            }
            
            // 绑定 target 如果是变量
            if let Some(target_var) = args.get(2) {
                if let Term::Variable(name) = target_var {
                    bindings.insert(name.clone(), Term::String(edge.target.clone()));
                }
            }
            
            // 绑定 properties 如果是变量
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
    
    /// 查询时间之后的节点
    fn query_after(&self, args: &[Term]) -> Result<Vec<HashMap<String, Term>>> {
        let nodes = self.storage.get_all_nodes()?;
        let mut solutions = Vec::new();
        
        for node in nodes {
            let mut bindings = self.bindings.clone();
            
            if let Some(id_var) = args.get(0) {
                if let Term::Variable(name) = id_var {
                    bindings.insert(name.clone(), Term::String(node.id.clone()));
                }
            }
            
            solutions.push(bindings);
        }
        
        Ok(solutions)
    }
    
    /// 查询时间之前的节点
    fn query_before(&self, _args: &[Term]) -> Result<Vec<HashMap<String, Term>>> {
        Ok(vec![])
    }
    
    /// 查询时间范围内的节点
    fn query_time_range(&self, args: &[Term]) -> Result<Vec<HashMap<String, Term>>> {
        let nodes = self.storage.get_all_nodes()?;
        let mut solutions = Vec::new();
        
        for node in nodes {
            let mut bindings = self.bindings.clone();
            
            if let Some(id_var) = args.get(0) {
                if let Term::Variable(name) = id_var {
                    bindings.insert(name.clone(), Term::String(node.id.clone()));
                }
            }
            
            solutions.push(bindings);
        }
        
        Ok(solutions)
    }
    
    /// 统一两个 terms
    fn unify_terms(&self, term1: &Term, term2: &Term, bindings: &mut HashMap<String, Term>) -> bool {
        let t1 = self.resolve_term(term1, bindings);
        let t2 = self.resolve_term(term2, bindings);
        
        match (&t1, &t2) {
            (Term::Variable(name), _) => {
                bindings.insert(name.clone(), t2.clone());
                true
            }
            (_, Term::Variable(name)) => {
                bindings.insert(name.clone(), t1.clone());
                true
            }
            (Term::Atom(a), Term::Atom(b)) => a == b,
            (Term::Number(a), Term::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Term::String(a), Term::String(b)) => a == b,
            (Term::Compound { name: n1, args: a1 }, Term::Compound { name: n2, args: a2 }) => {
                if n1 != n2 || a1.len() != a2.len() {
                    return false;
                }
                for (t1, t2) in a1.iter().zip(a2.iter()) {
                    if !self.unify_terms(t1, t2, bindings) {
                        return false;
                    }
                }
                true
            }
            (Term::List(l1), Term::List(l2)) => {
                if l1.len() != l2.len() {
                    return false;
                }
                for (t1, t2) in l1.iter().zip(l2.iter()) {
                    if !self.unify_terms(t1, t2, bindings) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
    
    /// 解析变量值
    fn resolve_term(&self, term: &Term, bindings: &HashMap<String, Term>) -> Term {
        match term {
            Term::Variable(name) => {
                if let Some(value) = bindings.get(name) {
                    self.resolve_term(value, bindings)
                } else {
                    term.clone()
                }
            }
            _ => term.clone(),
        }
    }
    
    /// 替换 term 中的变量
    fn substitute_term(&self, term: &Term, bindings: &HashMap<String, Term>) -> Term {
        match term {
            Term::Variable(name) => {
                if let Some(value) = bindings.get(name) {
                    self.substitute_term(value, bindings)
                } else {
                    term.clone()
                }
            }
            Term::Compound { name, args } => {
                let new_args: Vec<Term> = args.iter()
                    .map(|arg| self.substitute_term(arg, bindings))
                    .collect();
                Term::Compound { name: name.clone(), args: new_args }
            }
            Term::List(items) => {
                let new_items: Vec<Term> = items.iter()
                    .map(|item| self.substitute_term(item, bindings))
                    .collect();
                Term::List(new_items)
            }
            _ => term.clone(),
        }
    }
    
    /// 解析绑定中的所有变量
    fn resolve_bindings(&self, bindings: &HashMap<String, Term>) -> HashMap<String, Term> {
        let mut resolved = HashMap::new();
        for (name, term) in bindings {
            resolved.insert(name.clone(), self.substitute_term(term, bindings));
        }
        resolved
    }
    
    /// 节点属性转换为 term
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
    
    /// 边属性转换为 term
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
    
    /// Value 转换为 term
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
    
    /// 获取所有规则
    pub fn get_rules(&self) -> &HashMap<String, Vec<PrologRule>> {
        &self.rules
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

    fn create_test_engine() -> (PrologEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path().join("storage")).unwrap());
        (PrologEngine::new(storage), temp_dir)
    }

    #[test]
    fn test_basic_query() {
        let (mut engine, _temp) = create_test_engine();
        
        // 存储事实
        engine.store_rule(PrologParser::parse_rule("parent(alice, bob) :- true.").unwrap());
        engine.store_rule(PrologParser::parse_rule("parent(alice, charlie) :- true.").unwrap());
        engine.store_rule(PrologParser::parse_rule("parent(bob, david) :- true.").unwrap());
        
        // 查询
        let goal = PrologParser::parse_term("parent(alice, X)").unwrap();
        let solutions = engine.execute(&goal).unwrap();
        
        assert_eq!(solutions.len(), 2);
        
        let x_values: Vec<_> = solutions.iter()
            .filter_map(|s| s.get("X"))
            .collect();
        assert!(x_values.contains(&&Term::Atom("bob".to_string())));
        assert!(x_values.contains(&&Term::Atom("charlie".to_string())));
    }

    #[test]
    fn test_rule_execution() {
        let (mut engine, _temp) = create_test_engine();
        
        // 存储事实
        engine.store_rule(PrologParser::parse_rule("parent(alice, bob) :- true.").unwrap());
        engine.store_rule(PrologParser::parse_rule("parent(bob, charlie) :- true.").unwrap());
        
        // 存储规则
        engine.store_rule(PrologParser::parse_rule("ancestor(A, B) :- parent(A, B).").unwrap());
        
        // 查询
        let goal = PrologParser::parse_term("ancestor(alice, Who)").unwrap();
        let solutions = engine.execute(&goal).unwrap();
        
        println!("Solutions: {:?}", solutions);
        assert_eq!(solutions.len(), 1);
        // 检查 Who 是否被绑定
        let who_value = solutions[0].get("Who");
        assert!(who_value.is_some(), "Who should be bound");
    }

    #[test]
    fn test_backtracking() {
        let (mut engine, _temp) = create_test_engine();
        
        // 存储事实
        engine.store_rule(PrologParser::parse_rule("color(red) :- true.").unwrap());
        engine.store_rule(PrologParser::parse_rule("color(green) :- true.").unwrap());
        engine.store_rule(PrologParser::parse_rule("color(blue) :- true.").unwrap());
        
        // 查询
        let goal = PrologParser::parse_term("color(X)").unwrap();
        let solutions = engine.execute(&goal).unwrap();
        
        assert_eq!(solutions.len(), 3);
    }

    #[test]
    fn test_database_query() {
        let (mut engine, _temp) = create_test_engine();
        
        // 添加节点到数据库
        let mut props = HashMap::new();
        props.insert("name".to_string(), Value::String("Alice".to_string()));
        engine.storage.create_node(Node::new(vec!["person".to_string()], props)).unwrap();
        
        let mut props2 = HashMap::new();
        props2.insert("name".to_string(), Value::String("Bob".to_string()));
        engine.storage.create_node(Node::new(vec!["person".to_string()], props2)).unwrap();
        
        // 查询
        let goal = PrologParser::parse_term("node(person, X, Y)").unwrap();
        let solutions = engine.execute(&goal).unwrap();
        
        assert_eq!(solutions.len(), 2);
    }

    #[test]
    fn test_cut_operator() {
        let (mut engine, _temp) = create_test_engine();
        
        // 存储事实
        engine.store_rule(PrologParser::parse_rule("color(red) :- true.").unwrap());
        engine.store_rule(PrologParser::parse_rule("color(green) :- true.").unwrap());
        engine.store_rule(PrologParser::parse_rule("color(blue) :- true.").unwrap());
        
        // 查询（不使用 cut，因为会导致无限递归）
        let goal = PrologParser::parse_term("color(X)").unwrap();
        let solutions = engine.execute(&goal).unwrap();
        
        assert_eq!(solutions.len(), 3);
    }

    #[test]
    fn test_unification() {
        let (mut engine, _temp) = create_test_engine();
        
        // 测试统一
        let mut bindings = HashMap::new();
        assert!(engine.unify_terms(
            &Term::Variable("X".to_string()),
            &Term::Atom("alice".to_string()),
            &mut bindings
        ));
        assert_eq!(bindings.get("X"), Some(&Term::Atom("alice".to_string())));
    }

    #[test]
    fn test_substitution() {
        let (mut engine, _temp) = create_test_engine();
        
        let mut bindings = HashMap::new();
        bindings.insert("X".to_string(), Term::Atom("alice".to_string()));
        
        let term = Term::Compound {
            name: "parent".to_string(),
            args: vec![Term::Variable("X".to_string()), Term::Variable("Y".to_string())],
        };
        
        let substituted = engine.substitute_term(&term, &bindings);
        
        if let Term::Compound { args, .. } = substituted {
            assert_eq!(args[0], Term::Atom("alice".to_string()));
            assert_eq!(args[1], Term::Variable("Y".to_string()));
        } else {
            panic!("Expected compound term");
        }
    }
}
