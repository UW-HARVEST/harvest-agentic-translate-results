use crate::{dberror::RC, tables::{RM_TableData, Schema, Record, Value, DataType, ValueUnion}};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

/// C sizeof(int) = 4, sizeof(float) = 4, sizeof(bool) = sizeof(short) = 2
const SIZEOF_INT: i32 = 4;
const SIZEOF_FLOAT: i32 = 4;
const SIZEOF_BOOL: i32 = 2;

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset = 0i32;
    for i in 0..attr_num as usize {
        match schema.data_types[i] {
            DataType::DtString => offset += schema.type_length[i],
            DataType::DtInt => offset += SIZEOF_INT,
            DataType::DtFloat => offset += SIZEOF_FLOAT,
            DataType::DtBool => offset += SIZEOF_BOOL,
        }
    }
    *result = offset;
    RC::Ok
}

/// Helper used by tables.rs serialize_attr
pub fn attr_offset_val(schema: &Schema, attr_num: i32) -> i32 {
    let mut offset = 0i32;
    for i in 0..attr_num as usize {
        match schema.data_types[i] {
            DataType::DtString => offset += schema.type_length[i],
            DataType::DtInt => offset += SIZEOF_INT,
            DataType::DtFloat => offset += SIZEOF_FLOAT,
            DataType::DtBool => offset += SIZEOF_BOOL,
        }
    }
    offset
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    crate::tables::serialize_table_info(rel)
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    crate::tables::serialize_table_content(rel)
}

pub fn serialize_schema(schema: &Schema) -> String {
    crate::tables::serialize_schema(schema)
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    crate::tables::serialize_record(record, schema)
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    crate::tables::serialize_attr(record, schema, attr_num)
}

pub fn serialize_value(val: &Value) -> String {
    crate::tables::serialize_value(val)
}

pub fn string_to_value(val: &str) -> Value {
    crate::tables::string_to_value(val)
}
