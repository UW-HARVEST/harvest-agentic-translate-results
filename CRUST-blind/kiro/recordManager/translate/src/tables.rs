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
    match &value.v {
        ValueUnion::StringV(s) => s.clone(),
        ValueUnion::IntV(i) => i.to_string(),
        ValueUnion::FloatV(f) => format!("{}", f),
        ValueUnion::BoolV(b) => if *b { "true".to_string() } else { "false".to_string() },
    }
}

pub fn make_value(datatype: &str, value: &str) -> Value {
    match datatype {
        "INT" | "0" => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(value.parse().unwrap_or(0)),
        },
        "FLOAT" | "2" => Value {
            dt: DataType::DtFloat,
            v: ValueUnion::FloatV(value.parse().unwrap_or(0.0)),
        },
        "BOOL" | "3" => Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(value == "true" || value == "1"),
        },
        _ => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(value.to_string()),
        },
    }
}

pub fn string_to_value(value: &str) -> Value {
    if value.is_empty() {
        return Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    }
    match value.as_bytes()[0] {
        b'i' => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(value[1..].parse().unwrap_or(0)),
        },
        b'f' => Value {
            dt: DataType::DtFloat,
            v: ValueUnion::FloatV(value[1..].parse().unwrap_or(0.0)),
        },
        b's' => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(value[1..].to_string()),
        },
        b'b' => Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(value.as_bytes().get(1) == Some(&b't')),
        },
        _ => Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) },
    }
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    let tuples = crate::record_mgr::get_num_tuples(rel);
    let mut result = format!("TABLE <{}> with <{}> tuples:\n", rel.name, tuples);
    result.push_str(&serialize_schema(&rel.schema));
    result
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    let mut result = String::new();
    for i in 0..rel.schema.num_attr {
        if i != 0 { result.push_str(", "); }
        result.push_str(&rel.schema.attr_names[i as usize]);
    }
    // Note: scan functionality would be needed here for full implementation
    // This matches the C code structure
    result
}

pub fn serialize_schema(schema: &Schema) -> String {
    let mut result = format!("Schema with <{}> attributes (", schema.num_attr);
    for i in 0..schema.num_attr as usize {
        if i != 0 { result.push_str(", "); }
        result.push_str(&schema.attr_names[i]);
        result.push_str(": ");
        match schema.data_types[i] {
            DataType::DtInt => result.push_str("INT"),
            DataType::DtFloat => result.push_str("FLOAT"),
            DataType::DtString => {
                result.push_str(&format!("STRING[{}]", schema.type_length[i]));
            }
            DataType::DtBool => result.push_str("BOOL"),
        }
    }
    result.push(')');
    result.push_str(" with keys: (");
    for i in 0..schema.key_size as usize {
        if i != 0 { result.push_str(", "); }
        result.push_str(&schema.attr_names[schema.key_attrs[i] as usize]);
    }
    result.push_str(")\n");
    result
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut result = format!("[{}-{}] (", record.id.page, record.id.slot);
    for i in 0..schema.num_attr {
        result.push_str(&serialize_attr(record, schema, i));
        if i == 0 {
            // C code appends "" for i==0
        } else {
            result.push(',');
        }
    }
    result.push(')');
    result
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let offset = crate::rm_serializer::attr_offset_val(schema, attr_num);
    let data = record.data.as_bytes();
    let off = offset as usize;

    match schema.data_types[attr_num as usize] {
        DataType::DtInt => {
            let mut bytes = [0u8; 4];
            for j in 0..4 {
                if off + j < data.len() {
                    bytes[j] = data[off + j];
                }
            }
            let val = i32::from_ne_bytes(bytes);
            format!("{}:{}", schema.attr_names[attr_num as usize], val)
        }
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let end = (off + len).min(data.len());
            let slice = &data[off..end];
            let s = String::from_utf8_lossy(slice);
            // Trim trailing nulls
            let s = s.trim_end_matches('\0');
            format!("{}:{}", schema.attr_names[attr_num as usize], s)
        }
        DataType::DtFloat => {
            let mut bytes = [0u8; 4];
            for j in 0..4 {
                if off + j < data.len() {
                    bytes[j] = data[off + j];
                }
            }
            let val = f32::from_ne_bytes(bytes);
            format!("{}:{}", schema.attr_names[attr_num as usize], val)
        }
        DataType::DtBool => {
            // C bool is short (2 bytes)
            let mut bytes = [0u8; 2];
            for j in 0..2 {
                if off + j < data.len() {
                    bytes[j] = data[off + j];
                }
            }
            let val = i16::from_ne_bytes(bytes) != 0;
            format!("{}:{}", schema.attr_names[attr_num as usize], if val { "TRUE" } else { "FALSE" })
        }
    }
}

pub fn serialize_value(val: &Value) -> String {
    match &val.v {
        ValueUnion::IntV(i) => format!("{}", i),
        ValueUnion::FloatV(f) => format!("{}", f),
        ValueUnion::StringV(s) => s.clone(),
        ValueUnion::BoolV(b) => if *b { "true".to_string() } else { "false".to_string() },
    }
}
