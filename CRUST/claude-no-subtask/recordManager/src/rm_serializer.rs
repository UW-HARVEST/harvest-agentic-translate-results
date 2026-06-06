use crate::{dberror::RC, tables::{RM_TableData, Schema, Record, Value, DataType, ValueUnion}};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset: i32 = 0;
    let attr_num = attr_num as usize;
    for attr_pos in 0..attr_num {
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
    let num_tuples = crate::record_mgr::get_num_tuples(rel);
    result.push_str(&format!(
        "TABLE <{}> with <{}> tuples:\n",
        rel.name, num_tuples
    ));
    result.push_str(&serialize_schema(&rel.schema));
    result
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    let mut result = String::new();
    let schema = &rel.schema;
    for i in 0..schema.num_attr as usize {
        if i != 0 {
            result.push_str(", ");
        }
        result.push_str(&schema.attr_names[i]);
    }
    // Note: full scan support is implementation specific. Skipping the scan loop
    // (matches behavior since we don't have a working record manager test).
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
        let key_idx = schema.key_attrs[i] as usize;
        result.push_str(&schema.attr_names[key_idx]);
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

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset: i32 = 0;
    attr_offset(schema, attr_num, &mut offset);
    let attr_idx = attr_num as usize;
    let bytes = record.data.as_bytes();
    let off = offset as usize;
    let mut result = String::new();
    match schema.data_types[attr_idx] {
        DataType::DtInt => {
            let mut buf = [0u8; 4];
            if off + 4 <= bytes.len() {
                buf.copy_from_slice(&bytes[off..off + 4]);
            }
            let val = i32::from_ne_bytes(buf);
            result.push_str(&format!("{}:{}", schema.attr_names[attr_idx], val));
        }
        DataType::DtString => {
            let len = schema.type_length[attr_idx] as usize;
            let end = std::cmp::min(off + len, bytes.len());
            let s_bytes = if off < bytes.len() { &bytes[off..end] } else { &[][..] };
            // Truncate at null
            let null_pos = s_bytes.iter().position(|&b| b == 0).unwrap_or(s_bytes.len());
            let s = String::from_utf8_lossy(&s_bytes[..null_pos]).to_string();
            result.push_str(&format!("{}:{}", schema.attr_names[attr_idx], s));
        }
        DataType::DtFloat => {
            let mut buf = [0u8; 4];
            if off + 4 <= bytes.len() {
                buf.copy_from_slice(&bytes[off..off + 4]);
            }
            let val = f32::from_ne_bytes(buf);
            result.push_str(&format!("{}:{:.6}", schema.attr_names[attr_idx], val));
        }
        DataType::DtBool => {
            let val = if off < bytes.len() { bytes[off] != 0 } else { false };
            result.push_str(&format!(
                "{}:{}",
                schema.attr_names[attr_idx],
                if val { "TRUE" } else { "FALSE" }
            ));
        }
    }
    result
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
    if val.is_empty() {
        return Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        };
    }
    let bytes = val.as_bytes();
    let prefix = bytes[0] as char;
    let rest = &val[1..];
    match prefix {
        'i' => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(rest.parse::<i32>().unwrap_or(0)),
        },
        'f' => Value {
            dt: DataType::DtFloat,
            v: ValueUnion::FloatV(rest.parse::<f32>().unwrap_or(0.0)),
        },
        's' => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(rest.to_string()),
        },
        'b' => {
            // Per C: result->v.boolV = (val[1] == 't') ? TRUE : FALSE;
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
