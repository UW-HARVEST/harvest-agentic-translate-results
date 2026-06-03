use crate::{
    dberror::RC,
    record_mgr,
    tables::{DataType, RM_TableData, Record, Schema, Value, ValueUnion},
};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset: i32 = 0;
    let attr_pos_max = attr_num as usize;
    for attr_pos in 0..attr_pos_max {
        match schema.data_types[attr_pos] {
            DataType::DtString => offset += schema.type_length[attr_pos],
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
    result.push_str(&format!(
        "TABLE <{}> with <{}> tuples:\n",
        rel.name,
        record_mgr::get_num_tuples(rel)
    ));
    result.push_str(&serialize_schema(&rel.schema));
    result
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    // Without easy access to scan with mutability constraints, replicate the logic
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
        let idx = i as usize;
        let prefix = if i != 0 { ", " } else { "" };
        result.push_str(&format!("{}{}: ", prefix, schema.attr_names[idx]));
        match schema.data_types[idx] {
            DataType::DtInt => result.push_str("INT"),
            DataType::DtFloat => result.push_str("FLOAT"),
            DataType::DtString => {
                result.push_str(&format!("STRING[{}]", schema.type_length[idx]));
            }
            DataType::DtBool => result.push_str("BOOL"),
        }
    }
    result.push(')');
    result.push_str(" with keys: (");
    for i in 0..schema.key_size {
        let idx = i as usize;
        let prefix = if i != 0 { ", " } else { "" };
        let key_attr_idx = schema.key_attrs[idx] as usize;
        result.push_str(&format!("{}{}", prefix, schema.attr_names[key_attr_idx]));
    }
    result.push_str(")\n");
    result
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut result = String::new();
    result.push_str(&format!("[{}-{}] (", record.id.page, record.id.slot));
    for i in 0..schema.num_attr {
        result.push_str(&serialize_attr(record, schema, i));
        result.push_str(if i == 0 { "" } else { "," });
    }
    result.push(')');
    result
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset: i32 = 0;
    attr_offset(schema, attr_num, &mut offset);
    let idx = attr_num as usize;
    let off = offset as usize;
    let bytes = record.data.as_bytes();

    match schema.data_types[idx] {
        DataType::DtInt => {
            let val = read_i32(bytes, off);
            format!("{}:{}", schema.attr_names[idx], val)
        }
        DataType::DtString => {
            let len = schema.type_length[idx] as usize;
            let end = (off + len).min(bytes.len());
            // emulate strncpy: read up to len bytes or until first null
            let slice = &bytes[off..end];
            let null_pos = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
            let s = String::from_utf8_lossy(&slice[..null_pos]).to_string();
            format!("{}:{}", schema.attr_names[idx], s)
        }
        DataType::DtFloat => {
            let val = read_f32(bytes, off);
            format!("{}:{:.6}", schema.attr_names[idx], val)
        }
        DataType::DtBool => {
            let val = bytes.get(off).map(|&b| b != 0).unwrap_or(false);
            format!(
                "{}:{}",
                schema.attr_names[idx],
                if val { "TRUE" } else { "FALSE" }
            )
        }
    }
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    let sz = std::mem::size_of::<i32>();
    if offset + sz > data.len() {
        return 0;
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&data[offset..offset + sz]);
    i32::from_le_bytes(bytes)
}

fn read_f32(data: &[u8], offset: usize) -> f32 {
    let sz = std::mem::size_of::<f32>();
    if offset + sz > data.len() {
        return 0.0;
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&data[offset..offset + sz]);
    f32::from_le_bytes(bytes)
}

pub fn serialize_value(val: &Value) -> String {
    match &val.v {
        ValueUnion::IntV(i) => format!("{}", i),
        ValueUnion::FloatV(f) => format!("{:.6}", f),
        ValueUnion::StringV(s) => s.clone(),
        ValueUnion::BoolV(b) => (if *b { "true" } else { "false" }).to_string(),
    }
}

pub fn string_to_value(val: &str) -> Value {
    let bytes = val.as_bytes();
    if bytes.is_empty() {
        return Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        };
    }
    let prefix = bytes[0] as char;
    let rest = &val[1..];
    match prefix {
        'i' => {
            let v = parse_atoi(rest);
            Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(v),
            }
        }
        'f' => {
            let v = parse_atof(rest);
            Value {
                dt: DataType::DtFloat,
                v: ValueUnion::FloatV(v),
            }
        }
        's' => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(rest.to_string()),
        },
        'b' => {
            let b = rest.starts_with('t');
            Value {
                dt: DataType::DtBool,
                v: ValueUnion::BoolV(b),
            }
        }
        _ => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        },
    }
}

// emulate atoi: parses a leading optional sign and digits, stopping at non-digit
fn parse_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut v: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        v = v * 10 + (bytes[i] - b'0') as i64;
        i += 1;
    }
    (v as i32) * sign
}

fn parse_atof(s: &str) -> f32 {
    // collect leading numeric part
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    let start = i;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    let slice = &s[start..i];
    slice.parse::<f32>().unwrap_or(0.0)
}
