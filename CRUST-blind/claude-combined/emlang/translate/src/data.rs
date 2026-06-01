use std::fmt;
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Int,
    Str,
}
#[derive(Debug, Clone)]
pub struct Data {
    pub dtype: DataType,
    pub value: DataValue,
}
#[derive(Debug, Clone)]
pub enum DataValue {
    Int(i64),
    Str(String),
}
impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DataType::Int => "int",
            DataType::Str => "str",
        };
        write!(f, "{}", s)
    }
}
impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Match C non-debug version: prints just the value
        match &self.value {
            DataValue::Int(i) => write!(f, "{}", *i as i32),
            DataValue::Str(s) => write!(f, "{}", s),
        }
    }
}
impl Data {
    pub fn new(dtype: DataType) -> Data {
        // Mimic zero-initialization in C
        let value = match dtype {
            DataType::Int => DataValue::Int(0),
            DataType::Str => DataValue::Str(String::new()),
        };
        Data { dtype, value }
    }
    pub fn new_int(val: i64) -> Data {
        Data {
            dtype: DataType::Int,
            value: DataValue::Int(val),
        }
    }
    pub fn new_str(val: String) -> Data {
        Data {
            dtype: DataType::Str,
            value: DataValue::Str(val),
        }
    }
}
