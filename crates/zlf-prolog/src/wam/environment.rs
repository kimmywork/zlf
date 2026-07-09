use std::collections::HashMap;

use crate::parser::Term;

#[derive(Debug, Clone, Default)]
pub struct EnvironmentFrame {
    slots: HashMap<String, Option<Term>>,
    permanent_slots: Vec<Option<usize>>,
    continuation: Option<usize>,
    previous: Option<usize>,
    cut_base: usize,
}

impl EnvironmentFrame {
    pub fn allocate(
        vars: &[String],
        continuation: Option<usize>,
        previous: Option<usize>,
        cut_base: usize,
        permanent_count: usize,
    ) -> Self {
        let slots = vars.iter().map(|name| (name.clone(), None)).collect();
        Self {
            slots,
            permanent_slots: vec![None; permanent_count],
            continuation,
            previous,
            cut_base,
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

    pub fn cut_base(&self) -> usize {
        self.cut_base
    }

    pub fn permanent_slot(&self, slot: usize) -> Option<Option<usize>> {
        self.permanent_slots.get(slot).copied()
    }

    pub fn set_permanent_slot(&mut self, slot: usize, addr: usize) -> bool {
        if let Some(value) = self.permanent_slots.get_mut(slot) {
            *value = Some(addr);
            true
        } else {
            false
        }
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }
}
