use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "bool")]
    Bool(bool),
    #[serde(rename = "number")]
    Number(f64),
    #[serde(rename = "string")]
    String(String),
    #[serde(rename = "array")]
    Array(Vec<Value>),
    #[serde(rename = "object")]
    Object(HashMap<String, Value>),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(v) => Some(v),
            _ => None,
        }
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Number(v as f64)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Number(v as f64)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Number(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(v: HashMap<String, Value>) -> Self {
        Value::Object(v)
    }
}
