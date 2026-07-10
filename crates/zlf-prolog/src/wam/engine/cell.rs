use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cell {
    Ref(usize),
    Str(usize),
    Functor { name: String, arity: usize },
    Constant(String),
}

impl Cell {
    pub fn functor(name: impl Into<String>, arity: usize) -> Self {
        Self::Functor {
            name: name.into(),
            arity,
        }
    }

    pub fn is_unbound_ref_at(&self, addr: usize) -> bool {
        matches!(self, Self::Ref(target) if *target == addr)
    }
}
