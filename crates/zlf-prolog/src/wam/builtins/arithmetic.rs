use crate::parser::Term;

use super::error::{WamError, WamResult};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum NumberValue {
    Integer(i64),
    Float(f64),
}

impl NumberValue {
    pub(crate) fn as_f64(self) -> f64 {
        match self {
            Self::Integer(value) => value as f64,
            Self::Float(value) => value,
        }
    }

    pub(crate) fn into_term(self) -> Term {
        match self {
            Self::Integer(value) => Term::Integer(value),
            Self::Float(value) => Term::Float(value),
        }
    }
}

pub(crate) fn eval_arithmetic(term: &Term) -> WamResult<NumberValue> {
    match term {
        Term::Integer(value) => Ok(NumberValue::Integer(*value)),
        Term::Float(value) => Ok(NumberValue::Float(*value)),
        Term::Compound { name, args } => eval_function(name, args),
        Term::Variable(_) => Err(arithmetic_error("instantiation_error")),
        _ => Err(arithmetic_error("type_error(evaluable)")),
    }
}

fn eval_function(name: &str, args: &[Term]) -> WamResult<NumberValue> {
    match (name, args) {
        ("+", [value]) => eval_arithmetic(value),
        ("-", [value]) => negate(eval_arithmetic(value)?),
        ("+", [left, right]) => add(eval_arithmetic(left)?, eval_arithmetic(right)?),
        ("-", [left, right]) => subtract(eval_arithmetic(left)?, eval_arithmetic(right)?),
        ("*", [left, right]) => multiply(eval_arithmetic(left)?, eval_arithmetic(right)?),
        ("/", [left, right]) => divide(eval_arithmetic(left)?, eval_arithmetic(right)?),
        ("//", [left, right]) => integer_divide(eval_arithmetic(left)?, eval_arithmetic(right)?),
        ("mod", [left, right]) => modulo(eval_arithmetic(left)?, eval_arithmetic(right)?),
        ("rem", [left, right]) => remainder(eval_arithmetic(left)?, eval_arithmetic(right)?),
        ("abs", [value]) => absolute(eval_arithmetic(value)?),
        ("min", [left, right]) => minimum(eval_arithmetic(left)?, eval_arithmetic(right)?),
        ("max", [left, right]) => maximum(eval_arithmetic(left)?, eval_arithmetic(right)?),
        _ => Err(arithmetic_error("type_error(evaluable)")),
    }
}

fn negate(value: NumberValue) -> WamResult<NumberValue> {
    match value {
        NumberValue::Integer(value) => value
            .checked_neg()
            .map(NumberValue::Integer)
            .ok_or_else(|| arithmetic_error("evaluation_error(int_overflow)")),
        NumberValue::Float(value) => Ok(NumberValue::Float(-value)),
    }
}

fn add(left: NumberValue, right: NumberValue) -> WamResult<NumberValue> {
    integer_or_float(left, right, i64::checked_add, |a, b| a + b)
}

fn subtract(left: NumberValue, right: NumberValue) -> WamResult<NumberValue> {
    integer_or_float(left, right, i64::checked_sub, |a, b| a - b)
}

fn multiply(left: NumberValue, right: NumberValue) -> WamResult<NumberValue> {
    integer_or_float(left, right, i64::checked_mul, |a, b| a * b)
}

fn divide(left: NumberValue, right: NumberValue) -> WamResult<NumberValue> {
    if right.as_f64() == 0.0 {
        return Err(arithmetic_error("evaluation_error(zero_divisor)"));
    }
    Ok(NumberValue::Float(left.as_f64() / right.as_f64()))
}

fn integer_divide(left: NumberValue, right: NumberValue) -> WamResult<NumberValue> {
    let (left, right) = integer_pair(left, right)?;
    if right == 0 {
        return Err(arithmetic_error("evaluation_error(zero_divisor)"));
    }
    Ok(NumberValue::Integer(left.div_euclid(right)))
}

fn modulo(left: NumberValue, right: NumberValue) -> WamResult<NumberValue> {
    let (left, right) = integer_pair(left, right)?;
    if right == 0 {
        return Err(arithmetic_error("evaluation_error(zero_divisor)"));
    }
    Ok(NumberValue::Integer(left.rem_euclid(right)))
}

fn remainder(left: NumberValue, right: NumberValue) -> WamResult<NumberValue> {
    let (left, right) = integer_pair(left, right)?;
    if right == 0 {
        return Err(arithmetic_error("evaluation_error(zero_divisor)"));
    }
    Ok(NumberValue::Integer(left % right))
}

fn absolute(value: NumberValue) -> WamResult<NumberValue> {
    match value {
        NumberValue::Integer(value) => value
            .checked_abs()
            .map(NumberValue::Integer)
            .ok_or_else(|| arithmetic_error("evaluation_error(int_overflow)")),
        NumberValue::Float(value) => Ok(NumberValue::Float(value.abs())),
    }
}

fn minimum(left: NumberValue, right: NumberValue) -> WamResult<NumberValue> {
    Ok(if left.as_f64() <= right.as_f64() {
        left
    } else {
        right
    })
}

fn maximum(left: NumberValue, right: NumberValue) -> WamResult<NumberValue> {
    Ok(if left.as_f64() >= right.as_f64() {
        left
    } else {
        right
    })
}

fn integer_or_float(
    left: NumberValue,
    right: NumberValue,
    integer: fn(i64, i64) -> Option<i64>,
    float: fn(f64, f64) -> f64,
) -> WamResult<NumberValue> {
    match (left, right) {
        (NumberValue::Integer(left), NumberValue::Integer(right)) => integer(left, right)
            .map(NumberValue::Integer)
            .ok_or_else(|| arithmetic_error("evaluation_error(int_overflow)")),
        (left, right) => Ok(NumberValue::Float(float(left.as_f64(), right.as_f64()))),
    }
}

fn integer_pair(left: NumberValue, right: NumberValue) -> WamResult<(i64, i64)> {
    match (left, right) {
        (NumberValue::Integer(left), NumberValue::Integer(right)) => Ok((left, right)),
        _ => Err(arithmetic_error("type_error(integer)")),
    }
}

fn arithmetic_error(message: &str) -> WamError {
    WamError::Provider(message.to_string())
}
