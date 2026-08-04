use crate::{dberror::RC, tables::{RM_TableData, Schema, Record, Value, ValueUnion, DataType}};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset: i32 = 0;
    let n = attr_num as usize;
    for attr_pos in 0..n {
        match schema.data_types[attr_pos] {
            DataType::DtString => {
                offset += schema.type_length[attr_pos];
            }
            DataType::DtInt => {
                offset += 4; // sizeof(int)
            }
            DataType::DtFloat => {
                offset += 4; // sizeof(float)
            }
            DataType::DtBool => {
                offset += 1; // sizeof(bool)
            }
        }
    }
    *result = offset;
    RC::Ok
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    let mut result = String::new();
    let num_tuples = crate::record_mgr::get_num_tuples(rel);
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
    // We can't easily run a scan here without a mutable borrow; mimic the C behavior
    // but keep it simple for use with rel only.
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
    result.push(')');
    result.push_str(" with keys: (");
    for i in 0..schema.key_size as usize {
        if i != 0 {
            result.push_str(", ");
        }
        let k = schema.key_attrs[i] as usize;
        result.push_str(&schema.attr_names[k]);
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
            // C bug: appends "," AFTER index 0; reproduce same behavior:
            // actually C writes "" for i==0 and "," for others, and appends AFTER each attr's serialization.
        }
        // The original C does APPEND_STRING(serializeAttr...); APPEND(",") if i != 0
        // i.e. for i==0 nothing, for i>0 appends ","
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
    let n = attr_num as usize;
    let bytes = record.data.as_bytes();
    let off = offset as usize;
    let name = &schema.attr_names[n];
    match schema.data_types[n] {
        DataType::DtInt => {
            let mut buf = [0u8; 4];
            for j in 0..4 {
                if off + j < bytes.len() {
                    buf[j] = bytes[off + j];
                }
            }
            let val = i32::from_le_bytes(buf);
            format!("{}:{}", name, val)
        }
        DataType::DtString => {
            let len = schema.type_length[n] as usize;
            let mut s = String::new();
            for j in 0..len {
                if off + j >= bytes.len() {
                    break;
                }
                let b = bytes[off + j];
                if b == 0 {
                    break;
                }
                s.push(b as char);
            }
            format!("{}:{}", name, s)
        }
        DataType::DtFloat => {
            let mut buf = [0u8; 4];
            for j in 0..4 {
                if off + j < bytes.len() {
                    buf[j] = bytes[off + j];
                }
            }
            let val = f32::from_le_bytes(buf);
            format!("{}:{}", name, val)
        }
        DataType::DtBool => {
            let b = if off < bytes.len() { bytes[off] != 0 } else { false };
            format!("{}:{}", name, if b { "TRUE" } else { "FALSE" })
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
    if val.is_empty() {
        return Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        };
    }
    let first = val.chars().next().unwrap();
    let rest = &val[1..];
    match first {
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
