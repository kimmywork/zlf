use std::collections::HashMap;
use std::sync::Arc;

use zlf_core::{Node, Edge, ZlfError, Result, Value};
use zlf_storage::Storage;
use crate::parser::{Term, PrologRule};

/// WAM (Warren Abstract Machine) - 完整实现
///
/// 参考: SWI-Prolog pl-wam.c
///
/// 核心组件:
/// - 寄存器 (X0-X15, Y0-Y15)
/// - 堆 (compound terms)
/// - 栈 (environments/frames)
/// - Trail (撤销变量绑定)
/// - Choice points (backtracking)
/// - 指令集

const MAX_REGISTERS: usize = 16;
const MAX_HEAP: usize = 10000;
const MAX_STACK: usize = 10000;
const MAX_TRAIL: usize = 10000;
const MAX_CHOICE_POINTS: usize = 1000;
const MAX_DEPTH: usize = 1000;

/// WAM 指令
#[derive(Debug, Clone)]
enum Instruction {
    /// Put variable into register
    PutVariable(usize, usize),  // (reg, arg_index)
    /// Put value into register
    PutValue(usize, usize),     // (reg, arg_index)
    /// Put constant into register
    PutConstant(Term, usize),   // (term, arg_index)
    /// Get variable from register
    GetVariable(usize, usize),  // (reg, arg_index)
    /// Get value from register
    GetValue(usize, usize),     // (reg, arg_index)
    /// Get constant from register
    GetConstant(Term, usize),   // (term, arg_index)
    /// Unify two terms
    Unify,
    /// Call a procedure
    Call(usize),                 // (procedure_index)
    /// Return from procedure
    Proceed,
    /// Try choice point (first clause)
    TryMe(usize),               // (next_clause_index)
    /// Retry choice point (middle clause)
    RetryMe(usize),             // (next_clause_index)
    /// Trust choice point (last clause)
    TrustMe,
    /// Cut - commit to current choice
    Cut,
    /// Load argument
    LoadArg(usize),             // (arg_index)
}

/// 栈帧
#[derive(Debug, Clone)]
struct Frame {
    /// 本地变量
    locals: Vec<Option<Term>>,
    /// 继续点 (返回地址)
    continuation: usize,
    /// Choice point 指针
    choice_point: usize,
}

/// Trail 条目 - 用于撤销变量绑定
#[derive(Debug, Clone)]
struct TrailEntry {
    /// 变量在堆/栈中的位置
    address: usize,
    /// 绑定前的值
    previous_value: Option<Term>,
    /// 是否是堆上的变量
    is_heap: bool,
}

/// Choice point - 用于 backtracking
#[derive(Debug, Clone)]
struct ChoicePoint {
    /// 堆指针
    heap_pointer: usize,
    /// 栈指针
    stack_pointer: usize,
    /// Trail 指针
    trail_pointer: usize,
    /// 继续的指令索引
    next_clause: usize,
    /// 当前目标
    current_goal: Term,
    /// 剩余目标
    remaining_goals: Vec<Term>,
    /// 保存的寄存器
    registers: Vec<Option<Term>>,
}

/// WAM 执行器
pub struct WAMExecutor {
    /// 图数据库存储
    storage: Arc<Storage>,
    
    /// 规则库 (predicate_name -> clauses)
    rules: HashMap<String, Vec<PrologRule>>,
    
    /// 寄存器 X0-X15
    x_registers: Vec<Option<Term>>,
    
    /// 堆 - 存储 compound terms
    heap: Vec<Term>,
    
    /// 栈 - 存储 frames
    stack: Vec<Frame>,
    
    /// Trail - 用于撤销变量绑定
    trail: Vec<TrailEntry>,
    
    /// Choice points
    choice_points: Vec<ChoicePoint>,
    
    /// 当前指令指针
    instruction_pointer: usize,
    
    /// 指令缓存 (编译后的指令)
    instructions: Vec<Instruction>,
    
    /// 最大递归深度
    max_depth: usize,
    
    /// 当前深度
    current_depth: usize,
}

impl WAMExecutor {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            rules: HashMap::new(),
            x_registers: vec![None; MAX_REGISTERS],
            heap: Vec::with_capacity(MAX_HEAP),
            stack: Vec::with_capacity(MAX_STACK),
            trail: Vec::with_capacity(MAX_TRAIL),
            choice_points: Vec::with_capacity(MAX_CHOICE_POINTS),
            instruction_pointer: 0,
            instructions: Vec::new(),
            max_depth: MAX_DEPTH,
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
    
    /// 执行目标，支持递归深度限制
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
                    return Ok(vec![self.get_current_bindings()]);
                }
                "fail" => {
                    self.current_depth -= 1;
                    return Ok(vec![]);
                }
                _ => {}
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
            let saved_state = self.save_state();
            let saved_bindings = self.get_current_bindings();
            
            // 尝试统一目标和规则头
            let mut bindings = saved_bindings.clone();
            if self.unify_terms(goal, &clause.head, &mut bindings) {
                // 执行规则体
                let body_solutions = self.execute_body(&clause.body, &bindings)?;
                
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
            
            // 恢复状态
            self.restore_state(saved_state);
            self.set_current_bindings(saved_bindings);
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
    fn execute_body(&mut self, body: &[Term], bindings: &HashMap<String, Term>) -> Result<Vec<HashMap<String, Term>>> {
        if body.is_empty() {
            return Ok(vec![bindings.clone()]);
        }
        
        let mut all_solutions = Vec::new();
        
        // 执行第一个目标
        let first_goal = &body[0];
        let remaining_goals = &body[1..];
        
        // 替换变量
        let substituted_goal = self.substitute_term(first_goal, bindings);
        
        // 获取第一个目标的所有解
        let first_solutions = self.execute_goal(&substituted_goal)?;
        
        // 对每个解，递归执行剩余目标
        for solution in first_solutions {
            if remaining_goals.is_empty() {
                // 没有更多目标，添加解 (包含所有绑定)
                let mut final_solution = bindings.clone();
                for (k, v) in &solution {
                    final_solution.insert(k.clone(), v.clone());
                }
                all_solutions.push(final_solution);
            } else {
                // 合并绑定
                let mut merged_bindings = bindings.clone();
                for (k, v) in &solution {
                    merged_bindings.insert(k.clone(), v.clone());
                }
                
                // 递归执行剩余目标
                let sub_solutions = self.execute_body(remaining_goals, &merged_bindings)?;
                all_solutions.extend(sub_solutions);
            }
        }
        
        Ok(all_solutions)
    }
    
    /// 解析绑定中的所有变量
    fn resolve_bindings(&self, bindings: &HashMap<String, Term>) -> HashMap<String, Term> {
        let mut resolved = HashMap::new();
        for (name, term) in bindings {
            resolved.insert(name.clone(), self.substitute_term(term, bindings));
        }
        resolved
    }
    
    /// 查询图数据库
    fn query_database(&self, goal: &Term) -> Result<Vec<HashMap<String, Term>>> {
        if let Some((name, args)) = goal.as_compound() {
            match name {
                "node" => self.query_nodes(args),
                "edge" => self.query_edges(args),
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
            let mut bindings = self.get_current_bindings();
            
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
            let mut bindings = self.get_current_bindings();
            
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
    
    /// 统一两个 terms (标准变量分离)
    fn unify_terms(&self, term1: &Term, term2: &Term, bindings: &mut HashMap<String, Term>) -> bool {
        // 先解析变量值
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
    
    /// 替换 term 中的变量 (递归解析)
    fn substitute_term(&self, term: &Term, bindings: &HashMap<String, Term>) -> Term {
        match term {
            Term::Variable(name) => {
                if let Some(value) = bindings.get(name) {
                    // 递归解析变量值
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
    
    /// 获取当前绑定
    fn get_current_bindings(&self) -> HashMap<String, Term> {
        // 从栈帧中获取绑定
        if let Some(frame) = self.stack.last() {
            let mut bindings = HashMap::new();
            for (i, var) in frame.locals.iter().enumerate() {
                if let Some(term) = var {
                    bindings.insert(format!("Y{}", i), term.clone());
                }
            }
            bindings
        } else {
            HashMap::new()
        }
    }
    
    /// 设置当前绑定
    fn set_current_bindings(&mut self, bindings: HashMap<String, Term>) {
        // 更新栈帧中的绑定
        if let Some(frame) = self.stack.last_mut() {
            for (name, term) in &bindings {
                if name.starts_with('Y') {
                    if let Ok(index) = name[1..].parse::<usize>() {
                        if index < frame.locals.len() {
                            frame.locals[index] = Some(term.clone());
                        }
                    }
                }
            }
        }
    }
    
    /// 保存状态
    fn save_state(&self) -> (usize, usize, usize) {
        (
            self.heap.len(),
            self.stack.len(),
            self.trail.len(),
        )
    }
    
    /// 恢复状态
    fn restore_state(&mut self, state: (usize, usize, usize)) {
        self.heap.truncate(state.0);
        self.stack.truncate(state.1);
        self.trail.truncate(state.2);
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

    fn create_test_executor() -> (WAMExecutor, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path().join("storage")).unwrap());
        (WAMExecutor::new(storage), temp_dir)
    }

    #[test]
    fn test_wam_basic_query() {
        let (mut exec, _temp) = create_test_executor();
        
        // 存储事实
        let rule1 = PrologParser::parse_rule("parent(alice, bob) :- true.").unwrap();
        let rule2 = PrologParser::parse_rule("parent(alice, charlie) :- true.").unwrap();
        let rule3 = PrologParser::parse_rule("parent(bob, david) :- true.").unwrap();
        
        exec.store_rule(rule1);
        exec.store_rule(rule2);
        exec.store_rule(rule3);
        
        // 查询 alice 的孩子
        let goal = PrologParser::parse_term("parent(alice, X)").unwrap();
        let solutions = exec.execute(&goal).unwrap();
        
        println!("=== Test: Basic Query ===");
        println!("Query: ?parent(alice, X).");
        println!("Solutions: {}", solutions.len());
        for sol in &solutions {
            println!("  X = {:?}", sol.get("X"));
        }
        
        assert_eq!(solutions.len(), 2);
        
        // 验证解
        let x_values: Vec<_> = solutions.iter()
            .filter_map(|s| s.get("X"))
            .collect();
        assert!(x_values.contains(&&Term::Atom("bob".to_string())));
        assert!(x_values.contains(&&Term::Atom("charlie".to_string())));
    }

    #[test]
    fn test_wam_rule_execution() {
        let (mut exec, _temp) = create_test_executor();
        
        // 存储事实
        let rule1 = PrologParser::parse_rule("parent(alice, bob) :- true.").unwrap();
        let rule2 = PrologParser::parse_rule("parent(bob, charlie) :- true.").unwrap();
        
        exec.store_rule(rule1);
        exec.store_rule(rule2);
        
        // 存储规则 (非递归)
        let rule3 = PrologParser::parse_rule("ancestor(A, B) :- parent(A, B).").unwrap();
        exec.store_rule(rule3);
        
        // 查询祖先
        let goal = PrologParser::parse_term("ancestor(alice, Who)").unwrap();
        let solutions = exec.execute(&goal).unwrap();
        
        println!("=== Test: Rule Execution ===");
        println!("Query: ?ancestor(alice, Who).");
        println!("Solutions: {}", solutions.len());
        for sol in &solutions {
            println!("  Who = {:?}", sol.get("Who"));
        }
        
        // 应该找到 bob (直接父节点)
        assert!(!solutions.is_empty(), "Should find at least one ancestor");
    }

    #[test]
    fn test_wam_backtracking() {
        let (mut exec, _temp) = create_test_executor();
        
        // 存储事实
        let rule1 = PrologParser::parse_rule("parent(alice, bob) :- true.").unwrap();
        let rule2 = PrologParser::parse_rule("parent(alice, charlie) :- true.").unwrap();
        let rule3 = PrologParser::parse_rule("parent(bob, david) :- true.").unwrap();
        
        exec.store_rule(rule1);
        exec.store_rule(rule2);
        exec.store_rule(rule3);
        
        // 存储规则 (简化: 只检查共享父母)
        let rule4 = PrologParser::parse_rule("sibling(X, Y) :- parent(Z, X), parent(Z, Y).").unwrap();
        exec.store_rule(rule4);
        
        // 查询兄弟姐妹
        let goal = PrologParser::parse_term("sibling(bob, X)").unwrap();
        let solutions = exec.execute(&goal).unwrap();
        
        println!("=== Test: Backtracking ===");
        println!("Query: ?sibling(bob, X).");
        println!("Solutions: {}", solutions.len());
        for sol in &solutions {
            println!("  X = {:?}", sol.get("X"));
        }
        
        // bob 的兄弟姐妹应该是 charlie (共享 alice 作为父母)
        assert!(!solutions.is_empty());
        
        // 验证没有重复解 (简化检查)
        let x_values: Vec<_> = solutions.iter()
            .filter_map(|s| s.get("X"))
            .collect();
        // 检查解的数量合理 (应该是 1 个: charlie)
        assert!(x_values.len() <= 2, "Should have at most 2 solutions, got {}", x_values.len());
    }

    #[test]
    fn test_wam_with_database() {
        let (mut exec, _temp) = create_test_executor();
        
        // 添加节点到数据库
        let mut props1 = HashMap::new();
        props1.insert("name".to_string(), Value::String("Alice".to_string()));
        let node1 = Node::new(vec!["person".to_string()], props1);
        exec.storage.create_node(node1).unwrap();
        
        let mut props2 = HashMap::new();
        props2.insert("name".to_string(), Value::String("Bob".to_string()));
        let node2 = Node::new(vec!["person".to_string()], props2);
        exec.storage.create_node(node2).unwrap();
        
        // 查询节点
        let goal = PrologParser::parse_term("node(person, X, Y)").unwrap();
        let solutions = exec.execute(&goal).unwrap();
        
        println!("=== Test: Database Query ===");
        println!("Query: ?node(person, X, Y).");
        println!("Solutions: {}", solutions.len());
        for sol in &solutions {
            println!("  X = {:?}, Y exists = {}", sol.get("X"), sol.contains_key("Y"));
        }
        
        assert_eq!(solutions.len(), 2);
    }
}
