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
    // Returns a serialized representation of the string value's contents.
    if let ValueUnion::StringV(s) = &value.v {
        s.clone()
    } else {
        String::new()
    }
}

pub fn make_value(datatype: &str, value: &str) -> Value {
    match datatype {
        "INT" | "DT_INT" | "0" => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(value.parse::<i32>().unwrap_or(0)),
        },
        "FLOAT" | "DT_FLOAT" | "2" => Value {
            dt: DataType::DtFloat,
            v: ValueUnion::FloatV(value.parse::<f32>().unwrap_or(0.0)),
        },
        "BOOL" | "DT_BOOL" | "3" => Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(value == "true" || value == "t" || value == "TRUE"),
        },
        _ => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(value.to_string()),
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
