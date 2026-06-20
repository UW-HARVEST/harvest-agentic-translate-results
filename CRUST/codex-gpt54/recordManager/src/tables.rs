use crate::dt::Bool;
use std::fmt;
#[derive(Debug, Clone)]
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
serialize_value(value)
}
pub fn make_value(datatype: &str, value: &str) -> Value {
    match datatype {
        "0" => Value { dt: DataType::DtInt, v: ValueUnion::IntV(value.parse().unwrap_or(0)) },
        "1" => Value { dt: DataType::DtString, v: ValueUnion::StringV(value.to_string()) },
        "2" => Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(value.parse().unwrap_or(0.0)) },
        "3" => Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(matches!(value, "1" | "true" | "t" | "TRUE")),
        },
        _ => Value { dt: DataType::DtInt, v: ValueUnion::IntV(value.parse().unwrap_or(0)) },
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

pub(crate) fn bytes_from_string(value: &str) -> Vec<u8> {
    value.as_bytes().to_vec()
}

pub(crate) fn string_from_bytes(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(value) => value,
        Err(err) => {
            let raw = err.into_bytes();
            // String is the public storage type for binary record/page data in this port.
            unsafe { String::from_utf8_unchecked(raw) }
        }
    }
}

pub(crate) fn fixed_len_bytes(value: &str, len: usize) -> Vec<u8> {
    let mut out = vec![0; len];
    let src = value.as_bytes();
    let copy_len = src.len().min(len);
    out[..copy_len].copy_from_slice(&src[..copy_len]);
    out
}
