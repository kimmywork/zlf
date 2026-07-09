use crate::parser::{PrologRule, Term};

use super::codegen::WamCodegen;
use super::error::{WamError, WamResult};
use super::instruction::Instruction;
use super::predicate::{predicate_key, PredicateKey};
use super::program::WamProgram;
use super::query_codegen::CompiledQuery;
use super::rule_store::CompiledRuleArtifact;

pub fn compile_query_program(
    query: &Term,
    facts: &[Term],
    rules: &[PrologRule],
) -> WamResult<WamProgram> {
    Ok(compile_query_program_with_bindings(query, facts, rules)?.program)
}

pub fn compile_query_program_with_bindings(
    query: &Term,
    facts: &[Term],
    rules: &[PrologRule],
) -> WamResult<CompiledQuery> {
    let binding_start = max_program_arity(query, facts, rules, &[]);
    let compiled = WamCodegen::compile_query_goal_with_binding_start(query, binding_start)?;
    let mut clause_temp_start = query_temp_start(&compiled);
    let mut groups = ClauseGroups::new();
    for fact in facts {
        let clause = WamCodegen::compile_fact_head_with_temp_start(fact, clause_temp_start)?;
        clause_temp_start = next_clause_temp(clause_temp_start, &clause);
        groups.push(fact, clause)?;
    }
    for rule in rules {
        let clause = WamCodegen::compile_rule_clause_with_temp_start(rule, clause_temp_start)?;
        clause_temp_start = next_clause_temp(clause_temp_start, &clause);
        groups.push(&rule.head, clause)?;
    }
    assemble_query_program(compiled, groups)
}

pub fn compile_query_program_with_rule_artifacts(
    query: &Term,
    facts: &[Term],
    rules: &[PrologRule],
    artifacts: &[CompiledRuleArtifact],
) -> WamResult<CompiledQuery> {
    let binding_start = max_program_arity(query, facts, rules, artifacts);
    let compiled = WamCodegen::compile_query_goal_with_binding_start(query, binding_start)?;
    let mut clause_temp_start = query_temp_start(&compiled);
    let mut groups = ClauseGroups::new();
    for fact in facts {
        let clause = WamCodegen::compile_fact_head_with_temp_start(fact, clause_temp_start)?;
        clause_temp_start = next_clause_temp(clause_temp_start, &clause);
        groups.push(fact, clause)?;
    }
    for rule in rules {
        let clause = WamCodegen::compile_rule_clause_with_temp_start(rule, clause_temp_start)?;
        clause_temp_start = next_clause_temp(clause_temp_start, &clause);
        groups.push(&rule.head, clause)?;
    }
    for artifact in artifacts {
        let clause = artifact.materialize(clause_temp_start);
        clause_temp_start = next_clause_temp(clause_temp_start, &clause);
        groups.push_key(artifact.key.clone(), clause);
    }
    assemble_query_program(compiled, groups)
}

fn assemble_query_program(
    compiled: CompiledQuery,
    groups: ClauseGroups,
) -> WamResult<CompiledQuery> {
    let mut instructions = compiled.program.instructions().to_vec();
    instructions.push(Instruction::Proceed);
    let mut entries = Vec::new();
    for (key, clauses) in groups.into_inner() {
        let entry = instructions.len();
        append_clauses(&mut instructions, &clauses);
        entries.push((key, entry));
    }
    let program = entries
        .into_iter()
        .fold(WamProgram::new(instructions), |program, (key, entry)| {
            program.with_entry(key, entry)
        });
    Ok(CompiledQuery {
        program,
        bindings: compiled.bindings,
    })
}

fn next_clause_temp(current: usize, clause: &WamProgram) -> usize {
    max_register(clause).map_or(current, |register| current.max(register + 1))
}

fn max_register(clause: &WamProgram) -> Option<usize> {
    clause
        .instructions()
        .iter()
        .flat_map(instruction_registers)
        .max()
}

fn instruction_registers(instruction: &Instruction) -> Vec<usize> {
    match instruction {
        Instruction::PutVariable { register }
        | Instruction::PutConstant { register, .. }
        | Instruction::PutStructure { register, .. }
        | Instruction::SetVariable { register }
        | Instruction::SetValue { register }
        | Instruction::GetConstant { register, .. }
        | Instruction::GetStructure { register, .. }
        | Instruction::UnifyVariable { register }
        | Instruction::UnifyValue { register } => vec![*register],
        Instruction::PutValue { source, target }
        | Instruction::GetValue {
            left: source,
            right: target,
        }
        | Instruction::UnifyRegisters {
            left: source,
            right: target,
        } => vec![*source, *target],
        _ => Vec::new(),
    }
}

fn max_program_arity(
    query: &Term,
    facts: &[Term],
    rules: &[PrologRule],
    artifacts: &[CompiledRuleArtifact],
) -> usize {
    std::iter::once(query)
        .chain(facts.iter())
        .chain(rules.iter().flat_map(rule_terms))
        .chain(
            artifacts
                .iter()
                .flat_map(|artifact| rule_terms(&artifact.source)),
        )
        .filter_map(|term| predicate_key(term).map(|key| key.arity))
        .max()
        .unwrap_or_default()
}

fn rule_terms(rule: &PrologRule) -> impl Iterator<Item = &Term> {
    std::iter::once(&rule.head).chain(rule.body.iter())
}

fn query_temp_start(compiled: &CompiledQuery) -> usize {
    compiled
        .bindings
        .values()
        .copied()
        .max()
        .map_or(0, |register| register + 1)
}

struct ClauseGroups(Vec<(PredicateKey, Vec<WamProgram>)>);

impl ClauseGroups {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, head: &Term, clause: WamProgram) -> WamResult<()> {
        let key = predicate_key(head).ok_or(WamError::ExpectedFunctor(0))?;
        self.push_key(key, clause);
        Ok(())
    }

    fn push_key(&mut self, key: PredicateKey, clause: WamProgram) {
        if let Some((_, clauses)) = self.0.iter_mut().find(|(item, _)| *item == key) {
            clauses.push(clause);
        } else {
            self.0.push((key, vec![clause]));
        }
    }

    fn into_inner(self) -> Vec<(PredicateKey, Vec<WamProgram>)> {
        self.0
    }
}

fn append_clauses(instructions: &mut Vec<Instruction>, clauses: &[WamProgram]) {
    let starts = clause_starts(instructions.len(), clauses);
    for (index, clause) in clauses.iter().enumerate() {
        append_clause_prefix(instructions, index, &starts);
        instructions.extend_from_slice(clause.instructions());
    }
}

fn clause_starts(base: usize, clauses: &[WamProgram]) -> Vec<usize> {
    let mut offset = base;
    clauses
        .iter()
        .map(|clause| {
            let start = offset;
            offset += clause.instructions().len() + usize::from(clauses.len() > 1);
            start
        })
        .collect()
}

fn append_clause_prefix(instructions: &mut Vec<Instruction>, index: usize, starts: &[usize]) {
    if starts.len() <= 1 {
        return;
    }
    match index {
        0 => instructions.push(Instruction::TryMeElse(starts[1])),
        i if i + 1 == starts.len() => instructions.push(Instruction::TrustMe),
        i => instructions.push(Instruction::RetryMeElse(starts[i + 1])),
    }
}
