use std::collections::HashMap;

use crate::parser::Term;

#[derive(Debug, Clone, Default)]
pub struct EnvironmentFrame {
    slots: HashMap<String, Option<Term>>,
    continuation: Option<usize>,
    previous: Option<usize>,
}

impl EnvironmentFrame {
    pub fn allocate(vars: &[String], continuation: Option<usize>, previous: Option<usize>) -> Self {
        let slots = vars.iter().map(|name| (name.clone(), None)).collect();
        Self {
            slots,
            continuation,
            previous,
        }
    }

    pub fn set(&mut self, name: &str, value: Term) -> bool {
        if let Some(slot) = self.slots.get_mut(name) {
            *slot = Some(value);
            true
        } else {
            false
        }
    }

    pub fn get(&self, name: &str) -> Option<&Term> {
        self.slots.get(name).and_then(Option::as_ref)
    }

    pub fn continuation(&self) -> Option<usize> {
        self.continuation
    }

    pub fn previous(&self) -> Option<usize> {
        self.previous
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }
}
