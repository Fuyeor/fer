// src/interpreter/mod.rs
pub mod env;
pub mod value;

use crate::interpreter::env::Environment;
use crate::interpreter::value::RuntimeValue;
use crate::parser::ast::*;

pub struct Interpreter {
    pub env: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
        }
    }

    pub fn run(&mut self, module: Module) {
        for stmt in module.body {
            self.eval_statement(stmt);
        }
    }

    fn eval_statement(&mut self, stmt: Statement) {
        match stmt {
            Statement::Declaration { name, value, .. } => {
                let val = self.eval_expression(value);
                self.env.variables.insert(name, val);
            }
            Statement::Expression(expr) => {
                self.eval_expression(expr);
            }
            _ => {} // Future: Type definitions...
        }
    }

    fn eval_expression(&mut self, expr: Expression) -> RuntimeValue {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Int(n) => RuntimeValue::Int(n),
                Literal::String(s) => RuntimeValue::String(s),
                _ => RuntimeValue::Null,
            },
            Expression::Identifier(name) => self
                .env
                .variables
                .get(&name)
                .cloned()
                .unwrap_or(RuntimeValue::Null),
            Expression::Call { callee, args } => {
                // Currently, we only have the built-in 'print'
                if let Expression::Identifier(name) = *callee {
                    if name == "print" {
                        let arg_val = self.eval_expression(args[0].clone());
                        match arg_val {
                            RuntimeValue::String(s) => println!("{}", s),
                            RuntimeValue::Int(n) => println!("{}", n),
                            _ => println!("{:?}", arg_val),
                        }
                    }
                }
                RuntimeValue::Null
            }
            // Computing binary trees
            Expression::BinaryOp { left, op, right } => {
                let l_val = self.eval_expression(*left);
                let r_val = self.eval_expression(*right);
                match (l_val, r_val, op) {
                    (RuntimeValue::Int(l), RuntimeValue::Int(r), Op::Add) => {
                        RuntimeValue::Int(l + r)
                    }
                    (RuntimeValue::Int(l), RuntimeValue::Int(r), Op::Sub) => {
                        RuntimeValue::Int(l - r)
                    }
                    (RuntimeValue::Int(l), RuntimeValue::Int(r), Op::Mul) => {
                        RuntimeValue::Int(l * r)
                    }
                    (RuntimeValue::Int(l), RuntimeValue::Int(r), Op::Div) => {
                        RuntimeValue::Int(l / r)
                    }
                    _ => panic!("Runtime Error: Unsupported binary operation"),
                }
            }

            // Iterate through all subexpressions, evaluate them
            // and then merge them into a single large string
            Expression::InterpolatedString(parts) => {
                let mut result = String::new();
                for part in parts {
                    let val = self.eval_expression(part);
                    match val {
                        RuntimeValue::String(s) => result.push_str(&s),
                        RuntimeValue::Int(n) => result.push_str(&n.to_string()),
                        RuntimeValue::Bool(b) => result.push_str(&b.to_string()),
                        _ => result.push_str(&format!("{:?}", val)),
                    }
                }
                RuntimeValue::String(result)
            }
            _ => RuntimeValue::Null,
        }
    }
}
