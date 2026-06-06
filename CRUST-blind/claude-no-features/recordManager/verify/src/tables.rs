use crate::dt::Bool;
use std::fmt;
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    DtInt = 0,
    DtString = 1,
    DtFloat = 2,
    DtBool = 3,
}
impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            DataType::DtInt => "0",
            DataType::DtString => "1",
            DataType::DtFloat => "2",
            DataType::DtBool => "3",
        };
        write!(f, "{}", name)
    }
}
#[derive(Debug, Clone)]
pub struct Value {
    pub dt: DataType,
    pub v: ValueUnion,
}
#[derive(Debug, Clone)]
pub enum ValueUnion {
    IntV(i32),
    StringV(String),
    FloatV(f32),
    BoolV(Bool),
}
#[derive(Debug, Clone)]
pub struct RID {
    pub page: i32,
    pub slot: i32,
}
#[derive(Debug, Clone)]
pub struct Record {
    pub id: RID,
    pub data: String,
}
#[derive(Debug, Clone)]
pub struct Schema {
    pub num_attr: i32,
    pub attr_names: Vec<String>,
    pub data_types: Vec<DataType>,
    pub type_length: Vec<i32>,
    pub key_attrs: Vec<i32>,
    pub key_size: i32,
}
#[derive(Debug)]
pub struct RM_TableData {
    pub name: String,
    pub schema: Schema,
    pub mgmt_data: Option<Box<dyn std::any::Any>>,
}

pub fn make_string_value(value: &Value) -> String {
    // For DT_STRING values, just produce the string. The C macro creates
    // a Value with DT_STRING and copies the string. Here we just clone.
    match &value.v {
        ValueUnion::StringV(s) => s.clone(),
        _ => String::new(),
    }
}

pub fn make_value(datatype: &str, value: &str) -> Value {
    match datatype {
        "INT" | "DT_INT" | "0" => {
            let v: i32 = value.parse().unwrap_or(0);
            Value { dt: DataType::DtInt, v: ValueUnion::IntV(v) }
        }
        "STRING" | "DT_STRING" | "1" => {
            Value { dt: DataType::DtString, v: ValueUnion::StringV(value.to_string()) }
        }
        "FLOAT" | "DT_FLOAT" | "2" => {
            let v: f32 = value.parse().unwrap_or(0.0);
            Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(v) }
        }
        "BOOL" | "DT_BOOL" | "3" => {
            let v = matches!(value, "true" | "TRUE" | "t" | "T" | "1");
            Value { dt: DataType::DtBool, v: ValueUnion::BoolV(v) }
        }
        _ => Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) },
    }
}

pub fn string_to_value(value: &str) -> Value {
    crate::rm_serializer::string_to_value(value)
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    crate::rm_serializer::serialize_table_info(rel)
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    crate::rm_serializer::serialize_table_content(rel)
}

pub fn serialize_schema(schema: &Schema) -> String {
    crate::rm_serializer::serialize_schema(schema)
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    crate::rm_serializer::serialize_record(record, schema)
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    crate::rm_serializer::serialize_attr(record, schema, attr_num)
}

pub fn serialize_value(val: &Value) -> String {
    crate::rm_serializer::serialize_value(val)
}
