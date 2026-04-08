use crate::{dberror::RC, tables::{RM_TableData, Schema, Record, Value, DataType, ValueUnion}};
use crate::record_mgr::get_num_tuples;

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset = 0i32;
    for pos in 0..attr_num {
        let i = pos as usize;
        match schema.data_types[i] {
            DataType::DtString => offset += schema.type_length[i],
            DataType::DtInt => offset += 4,    // sizeof(int)
            DataType::DtFloat => offset += 4,  // sizeof(float)
            DataType::DtBool => offset += 2,   // sizeof(bool) = sizeof(short) = 2
        }
    }
    *result = offset;
    RC::Ok
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    let mut result = String::new();
    result.push_str(&format!("TABLE <{}> with <{}> tuples:\n", rel.name, get_num_tuples(rel)));
    result.push_str(&serialize_schema(&rel.schema));
    result
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    let mut result = String::new();
    for i in 0..rel.schema.num_attr {
        if i != 0 { result.push_str(", "); }
        result.push_str(&rel.schema.attr_names[i as usize]);
    }
    // Scan through records - simplified since full scan requires shared mutable state
    result
}

pub fn serialize_schema(schema: &Schema) -> String {
    let mut result = String::new();
    result.push_str(&format!("Schema with <{}> attributes (", schema.num_attr));
    for i in 0..schema.num_attr as usize {
        if i != 0 { result.push_str(", "); }
        result.push_str(&format!("{}: ", schema.attr_names[i]));
        match schema.data_types[i] {
            DataType::DtInt => result.push_str("INT"),
            DataType::DtFloat => result.push_str("FLOAT"),
            DataType::DtString => result.push_str(&format!("STRING[{}]", schema.type_length[i])),
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
    let mut result = String::new();
    result.push_str(&format!("[{}-{}] (", record.id.page, record.id.slot));
    for i in 0..schema.num_attr {
        result.push_str(&serialize_attr(record, schema, i));
        if i == 0 {
            // C code: APPEND(result, "%s", (i == 0) ? "" : ",");
            // This appends "" for i==0
        } else {
            result.push(',');
        }
    }
    result.push(')');
    result
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset = 0i32;
    attr_offset(schema, attr_num, &mut offset);
    let offset = offset as usize;
    let data_chars: Vec<char> = record.data.chars().collect();
    let idx = attr_num as usize;

    match schema.data_types[idx] {
        DataType::DtInt => {
            // Read 4 bytes as i32 (little-endian, matching C's memcpy)
            let mut bytes = [0u8; 4];
            for k in 0..4 {
                bytes[k] = if offset + k < data_chars.len() { data_chars[offset + k] as u8 } else { 0 };
            }
            let val = i32::from_le_bytes(bytes);
            format!("{}:{}", schema.attr_names[idx], val)
        }
        DataType::DtString => {
            let len = schema.type_length[idx] as usize;
            let s: String = data_chars[offset..].iter().take(len).collect();
            // Trim trailing nulls
            let s = s.trim_end_matches('\0');
            format!("{}:{}", schema.attr_names[idx], s)
        }
        DataType::DtFloat => {
            let mut bytes = [0u8; 4];
            for k in 0..4 {
                bytes[k] = if offset + k < data_chars.len() { data_chars[offset + k] as u8 } else { 0 };
            }
            let val = f32::from_le_bytes(bytes);
            format!("{}:{:.6}", schema.attr_names[idx], val)
        }
        DataType::DtBool => {
            // C bool is short (2 bytes)
            let mut bytes = [0u8; 2];
            for k in 0..2 {
                bytes[k] = if offset + k < data_chars.len() { data_chars[offset + k] as u8 } else { 0 };
            }
            let val = i16::from_le_bytes(bytes);
            let b = val != 0;
            format!("{}:{}", schema.attr_names[idx], if b { "TRUE" } else { "FALSE" })
        }
    }
}

pub fn serialize_value(val: &Value) -> String {
    match &val.v {
        ValueUnion::IntV(i) => format!("{}", i),
        ValueUnion::FloatV(f) => format!("{:.6}", f),
        ValueUnion::StringV(s) => s.clone(),
        ValueUnion::BoolV(b) => if *b { "true".to_string() } else { "false".to_string() },
    }
}

pub fn string_to_value(val: &str) -> Value {
    crate::tables::string_to_value(val)
}
