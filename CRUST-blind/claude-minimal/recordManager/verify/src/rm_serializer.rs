use crate::dberror::RC;
use crate::tables::{RM_TableData, Schema, Record, Value, ValueUnion, DataType};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset: i32 = 0;
    for attr_pos in 0..attr_num as usize {
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
    let num_tuples = crate::record_mgr::get_num_tuples(rel);
    let mut result = String::new();
    result.push_str(&format!("TABLE <{}> with <{}> tuples:\n", rel.name, num_tuples));
    result.push_str(&serialize_schema(&rel.schema));
    result
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    let mut result = String::new();
    for i in 0..rel.schema.num_attr as usize {
        if i != 0 {
            result.push_str(", ");
        }
        result.push_str(&rel.schema.attr_names[i]);
    }
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
            DataType::DtString => result.push_str(&format!("STRING[{}]", schema.type_length[i])),
            DataType::DtBool => result.push_str("BOOL"),
        }
    }
    result.push_str(")");
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
    for i in 0..schema.num_attr as i32 {
        result.push_str(&serialize_attr(record, schema, i));
        if i != 0 {
            // C code does APPEND(result, "%s", (i == 0) ? "" : ",");
            // which appends "," after every attr except the first
        }
        if i != 0 {
            result.push(',');
        }
    }
    result.push(')');
    result
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset: i32 = 0;
    let _ = attr_offset(schema, attr_num, &mut offset);
    let attr_idx = attr_num as usize;
    let attr_name = &schema.attr_names[attr_idx];

    // The Rust Record stores data as a String. We treat it as a bytes buffer.
    let data_bytes = record.data.as_bytes();
    let off = offset as usize;

    match schema.data_types[attr_idx] {
        DataType::DtInt => {
            let mut buf = [0u8; 4];
            let len = std::cmp::min(4, data_bytes.len().saturating_sub(off));
            if len > 0 {
                buf[..len].copy_from_slice(&data_bytes[off..off + len]);
            }
            let val = i32::from_ne_bytes(buf);
            format!("{}:{}", attr_name, val)
        }
        DataType::DtString => {
            let len = schema.type_length[attr_idx] as usize;
            let end = std::cmp::min(off + len, data_bytes.len());
            let s_bytes = if off < data_bytes.len() {
                &data_bytes[off..end]
            } else {
                &[]
            };
            let s = String::from_utf8_lossy(s_bytes).into_owned();
            // Strip trailing nulls
            let s = s.trim_end_matches('\0').to_string();
            format!("{}:{}", attr_name, s)
        }
        DataType::DtFloat => {
            let mut buf = [0u8; 4];
            let len = std::cmp::min(4, data_bytes.len().saturating_sub(off));
            if len > 0 {
                buf[..len].copy_from_slice(&data_bytes[off..off + len]);
            }
            let val = f32::from_ne_bytes(buf);
            format!("{}:{}", attr_name, val)
        }
        DataType::DtBool => {
            let val = if off < data_bytes.len() {
                data_bytes[off] != 0
            } else {
                false
            };
            format!("{}:{}", attr_name, if val { "TRUE" } else { "FALSE" })
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

pub fn string_to_value(val: &str) -> Value {
    if val.is_empty() {
        return Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        };
    }
    let first = val.as_bytes()[0] as char;
    let rest = &val[1..];
    match first {
        'i' => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(rest.parse().unwrap_or(0)),
        },
        'f' => Value {
            dt: DataType::DtFloat,
            v: ValueUnion::FloatV(rest.parse().unwrap_or(0.0)),
        },
        's' => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(rest.to_string()),
        },
        'b' => Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(rest.starts_with('t')),
        },
        _ => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        },
    }
}
