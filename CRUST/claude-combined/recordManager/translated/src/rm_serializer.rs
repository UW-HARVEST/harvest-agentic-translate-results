use crate::{dberror::RC, tables::{DataType, RM_TableData, Schema, Record, Value, ValueUnion}};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset: i32 = 0;
    for attr_pos in 0..attr_num {
        match schema.data_types[attr_pos as usize] {
            DataType::DtString => offset += schema.type_length[attr_pos as usize],
            DataType::DtInt => offset += std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => offset += std::mem::size_of::<f32>() as i32,
            DataType::DtBool => offset += std::mem::size_of::<bool>() as i32,
        }
    }
    *result = offset;
    RC::Ok
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    let mut result = String::new();
    // We don't have getNumTuples implemented to interrogate mgmt_data; mimic format with 0 tuples placeholder
    let num_tuples = 0;
    result.push_str(&format!("TABLE <{}> with <{}> tuples:\n", rel.name, num_tuples));
    result.push_str(&serialize_schema(&rel.schema));
    result
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    let mut result = String::new();
    for i in 0..rel.schema.num_attr {
        if i != 0 {
            result.push_str(", ");
        }
        result.push_str(&rel.schema.attr_names[i as usize]);
    }
    result
}

pub fn serialize_schema(schema: &Schema) -> String {
    let mut result = String::new();
    result.push_str(&format!("Schema with <{}> attributes (", schema.num_attr));
    for i in 0..schema.num_attr {
        if i != 0 {
            result.push_str(", ");
        }
        result.push_str(&format!("{}: ", schema.attr_names[i as usize]));
        match schema.data_types[i as usize] {
            DataType::DtInt => result.push_str("INT"),
            DataType::DtFloat => result.push_str("FLOAT"),
            DataType::DtString => result.push_str(&format!("STRING[{}]", schema.type_length[i as usize])),
            DataType::DtBool => result.push_str("BOOL"),
        }
    }
    result.push(')');
    result.push_str(" with keys: (");
    for i in 0..schema.key_size {
        if i != 0 {
            result.push_str(", ");
        }
        let attr_idx = schema.key_attrs[i as usize] as usize;
        result.push_str(&schema.attr_names[attr_idx]);
    }
    result.push_str(")\n");
    result
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut result = String::new();
    result.push_str(&format!("[{}-{}] (", record.id.page, record.id.slot));
    for i in 0..schema.num_attr {
        result.push_str(&serialize_attr(record, schema, i));
        if i != 0 {
            result.push(',');
        }
    }
    result.push(')');
    result
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset = 0;
    attr_offset(schema, attr_num, &mut offset);
    let attr_name = &schema.attr_names[attr_num as usize];
    // Best-effort serialization since data is stored as a String in Rust
    let data_bytes = record.data.as_bytes();
    let off = offset as usize;
    match schema.data_types[attr_num as usize] {
        DataType::DtInt => {
            let val = if off + 4 <= data_bytes.len() {
                i32::from_ne_bytes([data_bytes[off], data_bytes[off+1], data_bytes[off+2], data_bytes[off+3]])
            } else {
                0
            };
            format!("{}:{}", attr_name, val)
        }
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let end = std::cmp::min(off + len, data_bytes.len());
            let s = String::from_utf8_lossy(&data_bytes[off..end]);
            format!("{}:{}", attr_name, s)
        }
        DataType::DtFloat => {
            let val = if off + 4 <= data_bytes.len() {
                f32::from_ne_bytes([data_bytes[off], data_bytes[off+1], data_bytes[off+2], data_bytes[off+3]])
            } else {
                0.0_f32
            };
            format!("{}:{}", attr_name, val)
        }
        DataType::DtBool => {
            let val = if off < data_bytes.len() { data_bytes[off] != 0 } else { false };
            format!("{}:{}", attr_name, if val { "TRUE" } else { "FALSE" })
        }
    }
}

pub fn serialize_value(val: &Value) -> String {
    match (&val.dt, &val.v) {
        (DataType::DtInt, ValueUnion::IntV(i)) => format!("{}", i),
        (DataType::DtFloat, ValueUnion::FloatV(f)) => format!("{:.6}", f),
        (DataType::DtString, ValueUnion::StringV(s)) => s.clone(),
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            if *b { "true".to_string() } else { "false".to_string() }
        }
        _ => String::new(),
    }
}

pub fn string_to_value(val: &str) -> Value {
    if val.is_empty() {
        return Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    }
    let first = val.chars().next().unwrap();
    let rest = &val[1..];
    match first {
        'i' => {
            let n: i32 = rest.parse().unwrap_or(0);
            Value { dt: DataType::DtInt, v: ValueUnion::IntV(n) }
        }
        'f' => {
            let f: f32 = rest.parse().unwrap_or(0.0);
            Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(f) }
        }
        's' => {
            Value { dt: DataType::DtString, v: ValueUnion::StringV(rest.to_string()) }
        }
        'b' => {
            let b = rest.starts_with('t');
            Value { dt: DataType::DtBool, v: ValueUnion::BoolV(b) }
        }
        _ => Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) },
    }
}
