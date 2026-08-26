// compiler-rs/runtime/src/value.rs



use std::fmt;



use ir::hir::HirId;



/// A runtime value produced by evaluating Fer HIR.

#[derive(Debug, Clone, PartialEq)]

pub enum Value {
  
    Unit,
  
    Integer(i128),
  
    Float(f64),
  
    String(String),
  
    Bool(bool),
  
    Char(String),
  
    Regex(String),
  
    Array(Vec<Value>),
  
    Object(Vec<ObjectEntry>),
  
    Function(HirId),
  
}



impl fmt::Display for Value {
  
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
      
        match self {
          
            Self::Unit => formatter.write_str("()"),
              
            Self::Integer(value) => value.fmt(formatter),
              
            Self::Float(value) => value.fmt(formatter),
              
            Self::String(value) | Self::Char(value) | Self::Regex(value) => {
              
                formatter.write_str(value)
              
            }
              
            Self::Bool(value) => value.fmt(formatter),
              
            Self::Array(values) => {
              
                formatter.write_str("[")?;
              
                for (index, value) in values.iter().enumerate() {
                  
                    if index != 0 {
                      
                        formatter.write_str(", ")?;
                      
                    }
                  
                    value.fmt(formatter)?;
                  
                }
              
                formatter.write_str("]")
              
            }
              
            Self::Object(fields) => {
              
                formatter.write_str("{")?;
              
                for (index, field) in fields.iter().enumerate() {
                  
                    if index != 0 {
                      
                        formatter.write_str(", ")?;
                      
                    }
                  
                    write!(formatter, "{} = {}", field.name, field.value)?;
                  
                }
              
                formatter.write_str("}")
              
            }
              
            Self::Function(item) => write!(formatter, "<function:{}>", item.index()),
              
        }
      
    }
  
}



impl Value {
  
    /// Return the stable diagnostic category of this value.
  
    pub(crate) const fn kind(&self) -> ValueKind {
      
        match self {
          
            Self::Unit => ValueKind::Unit,
              
            Self::Integer(_) => ValueKind::Integer,
              
            Self::Float(_) => ValueKind::Float,
              
            Self::String(_) => ValueKind::String,
              
            Self::Bool(_) => ValueKind::Bool,
              
            Self::Char(_) => ValueKind::Char,
              
            Self::Regex(_) => ValueKind::Regex,
              
            Self::Array(_) => ValueKind::Array,
              
            Self::Object(_) => ValueKind::Object,
              
            Self::Function(_) => ValueKind::Function,
              
        }
      
    }
  
}



/// A runtime value category used in structured execution errors.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum ValueKind {
  
    Unit,
  
    Integer,
  
    Float,
  
    String,
  
    Bool,
  
    Char,
  
    Regex,
  
    Array,
  
    Object,
  
    Function,
  
}



/// A runtime object field with its source name retained for lookup and display.

#[derive(Debug, Clone, PartialEq)]

pub struct ObjectEntry {
  
    pub name: String,
  
    pub value: Value,
  
}











































































