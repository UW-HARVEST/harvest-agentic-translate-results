use crate::dt::Bool;
use std::fmt;

#[derive(Debug, Clone)]
pub enum DataType {
    DtInt = 0,
    DtString = 1,
    DtFloat = 2,
    DtBool = 3,
}

impl Default for DataType {
    fn default() -> Self {
        Self::DtInt
    }
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

#[derive(Debug, Clone, Default)]
pub struct RID {
    pub page: i32,
    pub slot: i32,
}

#[derive(Debug, Clone, Default)]
pub struct Record {
    pub id: RID,
    pub data: String,
}

#[derive(Debug, Clone, Default)]
pub struct Schema {
    pub num_attr: i32,
    pub attr_names: Vec<String>,
    pub data_types: Vec<DataType>,
    pub type_length: Vec<i32>,
    pub key_attrs: Vec<i32>,
    pub key_size: i32,
}

#[derive(Debug, Default)]
pub struct RM_TableData {
    pub name: String,
    pub schema: Schema,
    pub mgmt_data: Option<Box<dyn std::any::Any>>,
}

pub(crate) const BOOL_SIZE: usize = 2;

pub(crate) fn bytes_to_data(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}

pub(crate) fn data_to_bytes(data: &str) -> Vec<u8> {
    data.chars().map(|c| c as u32 as u8).collect()
}

pub(crate) fn ensure_byte_len(bytes: &mut Vec<u8>, len: usize) {
    if bytes.len() < len {
        bytes.resize(len, 0);
    } else if bytes.len() > len {
        bytes.truncate(len);
    }
}

pub(crate) fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    i32::from_ne_bytes(raw)
}

pub(crate) fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

pub(crate) fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    f32::from_ne_bytes(raw)
}

pub(crate) fn write_f32(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

pub(crate) fn read_bool(bytes: &[u8], offset: usize) -> Bool {
    let mut raw = [0u8; 2];
    raw.copy_from_slice(&bytes[offset..offset + 2]);
    i16::from_ne_bytes(raw) != 0
}

pub(crate) fn write_bool(bytes: &mut [u8], offset: usize, value: Bool) {
    let raw: i16 = if value { 1 } else { 0 };
    bytes[offset..offset + 2].copy_from_slice(&raw.to_ne_bytes());
}

pub(crate) fn clone_schema(schema: &Schema) -> Schema {
    schema.clone()
}

pub fn make_string_value(value: &Value) -> String {
    match &value.v {
        ValueUnion::StringV(text) => text.clone(),
        _ => serialize_value(value),
    }
}

pub fn make_value(datatype: &str, value: &str) -> Value {
    match datatype {
        "0" | "i" => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(value.parse().unwrap_or(-1)),
        },
        "1" | "s" => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(value.to_string()),
        },
        "2" | "f" => Value {
            dt: DataType::DtFloat,
            v: ValueUnion::FloatV(value.parse().unwrap_or(0.0)),
        },
        "3" | "b" => Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(matches!(value, "true" | "t" | "1")),
        },
        _ => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        },
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
