use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WamError {
    #[error("heap address out of bounds: {0}")]
    AddressOutOfBounds(usize),
    #[error("expected functor at heap address {0}")]
    ExpectedFunctor(usize),
    #[error("invalid instruction state: {0}")]
    InvalidInstructionState(&'static str),
    #[error("unsupported term for WAM codegen: {0}")]
    UnsupportedTerm(&'static str),
    #[error("fact provider error: {0}")]
    Provider(String),
}

pub type WamResult<T> = std::result::Result<T, WamError>;
