#[path = "builtins/arithmetic.rs"]
mod builtin_arithmetic;
#[path = "builtins/catalog.rs"]
pub mod builtin_catalog;
#[path = "builtins/control.rs"]
mod builtin_control;
#[path = "builtins/conversion.rs"]
mod builtin_conversion;
#[path = "builtins/dynamic.rs"]
mod builtin_dynamic;
#[path = "builtins/executor.rs"]
pub mod builtin_executor;
#[path = "builtins/list.rs"]
mod builtin_list;
#[path = "builtins/term.rs"]
mod builtin_term;
#[path = "builtins/stdlib.rs"]
mod stdlib_rules;

#[path = "engine/cell.rs"]
pub mod cell;
#[path = "engine/choice_control.rs"]
pub mod choice_control;
#[path = "engine/choice_point.rs"]
pub mod choice_point;
#[path = "engine/constant.rs"]
mod constant;
#[path = "engine/environment.rs"]
pub mod environment;
#[path = "engine/environment_stack.rs"]
pub mod environment_stack;
#[path = "engine/error.rs"]
pub mod error;
#[path = "engine/execution_result.rs"]
pub mod execution_result;
#[path = "engine/executor.rs"]
pub mod executor;
#[path = "engine/executor_index.rs"]
mod executor_index;
#[path = "engine/executor_solve.rs"]
pub mod executor_solve;
#[path = "engine/executor_terms.rs"]
mod executor_terms;
#[path = "engine/external_provider.rs"]
mod external_provider;
#[path = "engine/heap.rs"]
pub mod heap;
#[path = "engine/instruction.rs"]
pub mod instruction;
#[path = "engine/machine.rs"]
pub mod machine;
#[path = "engine/machine_terms.rs"]
mod machine_terms;
#[path = "engine/register.rs"]
pub mod register;
#[path = "engine/structure_mode.rs"]
pub mod structure_mode;
#[path = "engine/structure_ops.rs"]
pub mod structure_ops;
#[path = "engine/term_reader.rs"]
pub mod term_reader;
#[path = "engine/trail.rs"]
pub mod trail;
#[path = "engine/unification.rs"]
pub mod unification;

#[path = "compile/codegen.rs"]
pub mod codegen;
#[path = "compile/codegen_terms.rs"]
mod codegen_terms;
#[path = "compile/compiler.rs"]
pub mod compiler;
#[path = "compile/permanent_vars.rs"]
pub mod permanent_vars;
#[path = "compile/program.rs"]
pub mod program;
#[path = "compile/program_codegen.rs"]
pub mod program_codegen;
#[path = "compile/query_codegen.rs"]
pub mod query_codegen;
#[path = "compile/rule_codegen.rs"]
pub mod rule_codegen;

#[path = "providers/composite.rs"]
pub mod composite_provider;
#[path = "providers/fact.rs"]
pub mod fact_provider;
#[path = "providers/graph_algorithm.rs"]
pub mod graph_algorithm_provider;
#[path = "providers/graph_algorithm_terms.rs"]
mod graph_algorithm_terms;
#[path = "providers/graph_view.rs"]
pub mod graph_view_provider;
#[path = "providers/index_limits.rs"]
mod index_limits;
#[path = "providers/index.rs"]
pub mod index_provider;
#[path = "providers/index_vector.rs"]
mod index_vector_provider;
#[path = "providers/introspection.rs"]
pub mod introspection_provider;
#[path = "providers/storage_goal.rs"]
mod storage_goal;
#[path = "providers/storage.rs"]
pub mod storage_provider;
#[path = "providers/view_helpers.rs"]
pub mod view_helpers;

#[path = "storage/fact_key.rs"]
pub mod fact_key;
#[path = "storage/fact_lowering.rs"]
pub mod fact_lowering;
#[path = "storage/rule_store.rs"]
pub mod rule_store;
#[path = "storage/storage_retract.rs"]
pub mod storage_retract;
#[path = "storage/storage_writer.rs"]
pub mod storage_writer;

#[path = "metadata/dependency_graph.rs"]
pub mod dependency_graph;
#[path = "metadata/predicate.rs"]
pub mod predicate;
#[path = "metadata/predicate_catalog.rs"]
pub mod predicate_catalog;
#[path = "metadata/predicate_registry.rs"]
pub mod predicate_registry;
#[path = "metadata/proof.rs"]
pub mod proof;

pub mod runtime;
pub mod tabling;

pub use builtin_catalog::builtin_predicates;
pub use builtin_executor::BuiltinExecutor;
pub use cell::Cell;
pub use choice_point::ChoicePointFrame;
pub use codegen::WamCodegen;
pub use compiler::M0Compiler;
pub use composite_provider::CompositeFactProvider;
pub use dependency_graph::RuleDependencyGraph;
pub use error::{WamError, WamResult};
pub use execution_result::ExecutionResult;
pub use executor::WamExecutor;
pub use fact_key::{
    term_to_delete_pattern, term_to_fact_key, DeletePattern, FactKey, MutationEvent,
};
pub use fact_provider::{FactProvider, StaticFactProvider};
pub use graph_algorithm_provider::GraphAlgorithmProvider;
pub use graph_view_provider::GraphViewProvider;
pub use heap::Heap;
pub use index_limits::{IndexAnswerLimits, IndexAnswerMetrics};
pub use index_provider::IndexFactProvider;
pub use instruction::Instruction;
pub use introspection_provider::IntrospectionProvider;
pub use machine::M0Machine;
pub use predicate::{compound_args, predicate_key, PredicateKey};
pub use predicate_catalog::{graph_algorithm_predicates, graph_view_predicates, index_predicates};
pub use predicate_registry::{PredicateKind, PredicateRegistry};
pub use program::WamProgram;
pub use program_codegen::{compile_query_program, compile_query_program_with_bindings};
pub use proof::{ProofAnswer, ProofClause, ProofKind, ProofNode, ProofTree};
pub use query_codegen::CompiledQuery;
pub use register::RegisterFile;
pub use rule_store::{CompiledRuleArtifact, StorageRuleStore};
pub use runtime::WamRuntime;
pub use storage_provider::StorageFactProvider;
pub use storage_writer::StorageFactWriter;
pub use tabling::{
    NormalizedTerm, PersistedTable, RocksTableBackend, TableAnswer, TableBackend,
    TableDependencies, TableEntry, TableKey, TableLimits, TableManager, TableMetricsSnapshot,
    TableState, TableStore,
};
pub use trail::Trail;
pub use unification::Unifier;

#[cfg(test)]
mod tests;
