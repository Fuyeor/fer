// src/interpreter/env.rs
use crate::interpreter::value::RuntimeValue;
use std::collections::HashMap;

pub struct Environment {
    pub variables: HashMap<String, RuntimeValue>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }
}
