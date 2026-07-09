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
    let compiled = WamCodegen::compile_query_goal_with_bindings(query)?;
    let temp_start = query_temp_start(&compiled);
    let mut groups = ClauseGroups::new();
    for fact in facts {
        groups.push(
            fact,
            WamCodegen::compile_fact_head_with_temp_start(fact, temp_start)?,
        )?;
    }
    for rule in rules {
        groups.push(
            &rule.head,
            WamCodegen::compile_rule_clause_with_temp_start(rule, temp_start)?,
        )?;
    }
    assemble_query_program(compiled, groups)
}

pub fn compile_query_program_with_rule_artifacts(
    query: &Term,
    facts: &[Term],
    rules: &[PrologRule],
    artifacts: &[CompiledRuleArtifact],
) -> WamResult<CompiledQuery> {
    let compiled = WamCodegen::compile_query_goal_with_bindings(query)?;
    let temp_start = query_temp_start(&compiled);
    let mut groups = ClauseGroups::new();
    for fact in facts {
        groups.push(
            fact,
            WamCodegen::compile_fact_head_with_temp_start(fact, temp_start)?,
        )?;
    }
    for rule in rules {
        groups.push(
            &rule.head,
            WamCodegen::compile_rule_clause_with_temp_start(rule, temp_start)?,
        )?;
    }
    for artifact in artifacts {
        groups.push_key(artifact.key.clone(), artifact.materialize(temp_start));
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
