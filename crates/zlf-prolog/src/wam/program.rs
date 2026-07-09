use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::instruction::Instruction;
use super::predicate::PredicateKey;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WamProgram {
    instructions: Vec<Instruction>,
    entries: HashMap<PredicateKey, usize>,
}

impl WamProgram {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            entries: HashMap::new(),
        }
    }

    pub fn with_entry(mut self, key: PredicateKey, offset: usize) -> Self {
        self.entries.insert(key, offset);
        self
    }

    pub fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn entry(&self, key: &PredicateKey) -> Option<usize> {
        self.entries.get(key).copied()
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}
