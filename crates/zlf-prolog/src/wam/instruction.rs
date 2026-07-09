use serde::{Deserialize, Serialize};

use super::predicate::PredicateKey;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    PutVariable {
        register: usize,
    },
    PutValue {
        source: usize,
        target: usize,
    },
    PutPermanentValue {
        slot: usize,
        register: usize,
    },
    PutConstant {
        value: String,
        register: usize,
    },
    PutStructure {
        name: String,
        arity: usize,
        register: usize,
    },
    PutList {
        arity: usize,
        register: usize,
    },
    SetVariable {
        register: usize,
    },
    SetValue {
        register: usize,
    },
    SetPermanentValue {
        slot: usize,
    },
    SetConstant {
        value: String,
    },
    GetConstant {
        value: String,
        register: usize,
    },
    GetStructure {
        name: String,
        arity: usize,
        register: usize,
    },
    GetList {
        arity: usize,
        register: usize,
    },
    GetValue {
        left: usize,
        right: usize,
    },
    GetPermanentValue {
        slot: usize,
        register: usize,
    },
    UnifyConstant {
        value: String,
    },
    UnifyVariable {
        register: usize,
    },
    UnifyValue {
        register: usize,
    },
    UnifyPermanentValue {
        slot: usize,
    },
    UnifyRegisters {
        left: usize,
        right: usize,
    },
    Call(PredicateKey),
    Execute(PredicateKey),
    Proceed,
    Cut,
    NeckCut,
    GetLevel {
        slot: usize,
    },
    CutLevel {
        slot: usize,
    },
    Allocate,
    AllocatePermanent {
        permanent_count: usize,
    },
    Deallocate,
    TryMeElse(usize),
    RetryMeElse(usize),
    TrustMe,
}

impl Instruction {
    pub fn put_constant(value: impl Into<String>, register: usize) -> Self {
        Self::PutConstant {
            value: value.into(),
            register,
        }
    }

    pub fn get_constant(value: impl Into<String>, register: usize) -> Self {
        Self::GetConstant {
            value: value.into(),
            register,
        }
    }

    pub fn get_structure(name: impl Into<String>, arity: usize, register: usize) -> Self {
        Self::GetStructure {
            name: name.into(),
            arity,
            register,
        }
    }

    pub fn unify_constant(value: impl Into<String>) -> Self {
        Self::UnifyConstant {
            value: value.into(),
        }
    }
}
