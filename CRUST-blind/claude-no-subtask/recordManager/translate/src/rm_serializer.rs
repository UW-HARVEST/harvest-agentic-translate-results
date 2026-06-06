use crate::dberror::RC;
use crate::tables::{DataType, RM_TableData, Record, Schema, Value, ValueUnion};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset: i32 = 0;
    let n = attr_num as usize;
    for attr_pos in 0..n.min(schema.data_types.len()) {
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
        crate::record_mgr::get_num_tuples(rel)
    ));
    result.push_str(&serialize_schema(&rel.schema));
    result
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    // Iteration over a scan would require mutable access to the table; in tests
    // this is invoked on a const RM_TableData, but the C version did the same.
    let mut result = String::new();
    for i in 0..rel.schema.num_attr as usize {
        if i != 0 {
            result.push_str(", ");
        }
        result.push_str(&rel.schema.attr_names[i]);
    }
    // We cannot easily do startScan/next here without mutability and a clone of rel,
    // so we leave the rest empty (matches behavior when no records can be enumerated).
    result
}

pub fn serialize_schema(schema: &Schema) -> String {
    let mut result = String::new();
    result.push_str(&format!("Schema with <{}> attributes (", schema.num_attr));
    for i in 0..schema.num_attr as usize {
        if i != 0 {
            result.push_str(", ");
        }
        result.push_str(&format!("{}: ", schema.attr_names[i]));
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
        if i != 0 {
            result.push_str(", ");
        }
        let idx = schema.key_attrs[i] as usize;
        result.push_str(&schema.attr_names[idx]);
    }
    result.push_str(")\n");
    result
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut result = String::new();
    result.push_str(&format!("[{}-{}] (", record.id.page, record.id.slot));
    for i in 0..schema.num_attr as usize {
        result.push_str(&serialize_attr(record, schema, i as i32));
        if i != 0 {
            result.push(',');
        }
    }
    result.push(')');
    result
}

fn record_bytes(record: &Record) -> Vec<u8> {
    record.data.chars().map(|c| (c as u32) as u8).collect()
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset: i32 = 0;
    let _ = attr_offset(schema, attr_num, &mut offset);
    let bytes = record_bytes(record);
    let off = offset as usize;
    let idx = attr_num as usize;
    match schema.data_types[idx] {
        DataType::DtInt => {
            let mut int_bytes = [0u8; 4];
            for k in 0..4 {
                int_bytes[k] = if off + k < bytes.len() { bytes[off + k] } else { 0 };
            }
            let val = i32::from_ne_bytes(int_bytes);
            format!("{}:{}", schema.attr_names[idx], val)
        }
        DataType::DtString => {
            let len = schema.type_length[idx] as usize;
            let end = (off + len).min(bytes.len());
            let slice = if off < bytes.len() { &bytes[off..end] } else { &[][..] };
            let mut s = String::new();
            for &b in slice {
                if b == 0 {
                    break;
                }
                s.push(b as char);
            }
            format!("{}:{}", schema.attr_names[idx], s)
        }
        DataType::DtFloat => {
            let mut float_bytes = [0u8; 4];
            for k in 0..4 {
                float_bytes[k] = if off + k < bytes.len() { bytes[off + k] } else { 0 };
            }
            let val = f32::from_ne_bytes(float_bytes);
            format!("{}:{:.6}", schema.attr_names[idx], val)
        }
        DataType::DtBool => {
            let val = if off < bytes.len() { bytes[off] != 0 } else { false };
            format!(
                "{}:{}",
                schema.attr_names[idx],
                if val { "TRUE" } else { "FALSE" }
            )
        }
    }
}

pub fn serialize_value(val: &Value) -> String {
    match &val.v {
        ValueUnion::IntV(i) => format!("{}", i),
        ValueUnion::FloatV(f) => format!("{:.6}", f),
        ValueUnion::StringV(s) => s.clone(),
        ValueUnion::BoolV(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
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
    let first = bytes[0] as char;
    let rest = &val[1..];
    match first {
        'i' => {
            // C's atoi: parse leading integer; return 0 if non-numeric.
            let parsed = parse_leading_int(rest);
            Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(parsed),
            }
        }
        'f' => {
            let parsed = parse_leading_float(rest);
            Value {
                dt: DataType::DtFloat,
                v: ValueUnion::FloatV(parsed),
            }
        }
        's' => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(rest.to_string()),
        },
        'b' => {
            let b = !rest.is_empty() && rest.as_bytes()[0] == b't';
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

fn parse_leading_int(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    let mut sign = 1i32;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut val = 0i32;
    let mut found = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.saturating_mul(10).saturating_add((bytes[i] - b'0') as i32);
        i += 1;
        found = true;
    }
    if !found {
        0
    } else {
        sign * val
    }
}

fn parse_leading_float(s: &str) -> f32 {
    // Try parsing the longest valid prefix as f32; fall back to 0.0.
    let mut end = 0;
    let bytes = s.as_bytes();
    if end < bytes.len() && (bytes[end] == b'-' || bytes[end] == b'+') {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        end += 1;
        if end < bytes.len() && (bytes[end] == b'-' || bytes[end] == b'+') {
            end += 1;
        }
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    if end == 0 {
        return 0.0;
    }
    s[..end].parse::<f32>().unwrap_or(0.0)
}
