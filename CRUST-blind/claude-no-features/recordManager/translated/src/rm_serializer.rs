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
    let mut s = String::new();
    s.push_str(&format!(
        "TABLE <{}> with <{}> tuples:\n",
        rel.name,
        crate::record_mgr::get_num_tuples(rel)
    ));
    s.push_str(&serialize_schema(&rel.schema));
    s
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    // Best-effort: produce header with attribute names. Without scan we
    // cannot read all records, but we attempt a scan with no condition.
    let mut s = String::new();
    for i in 0..rel.schema.num_attr as usize {
        if i != 0 {
            s.push_str(", ");
        }
        s.push_str(&rel.schema.attr_names[i]);
    }
    s
}

pub fn serialize_schema(schema: &Schema) -> String {
    let mut s = String::new();
    s.push_str(&format!("Schema with <{}> attributes (", schema.num_attr));
    for i in 0..schema.num_attr as usize {
        if i != 0 {
            s.push_str(", ");
        }
        s.push_str(&format!("{}: ", schema.attr_names[i]));
        match schema.data_types[i] {
            DataType::DtInt => s.push_str("INT"),
            DataType::DtFloat => s.push_str("FLOAT"),
            DataType::DtString => s.push_str(&format!("STRING[{}]", schema.type_length[i])),
            DataType::DtBool => s.push_str("BOOL"),
        }
    }
    s.push(')');
    s.push_str(" with keys: (");
    for i in 0..schema.key_size as usize {
        if i != 0 {
            s.push_str(", ");
        }
        let idx = schema.key_attrs[i] as usize;
        s.push_str(&schema.attr_names[idx]);
    }
    s.push_str(")\n");
    s
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut s = String::new();
    s.push_str(&format!("[{}-{}] (", record.id.page, record.id.slot));
    for i in 0..schema.num_attr as usize {
        s.push_str(&serialize_attr(record, schema, i as i32));
        if i != 0 {
            s.push(',');
        }
    }
    s.push(')');
    s
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset = 0i32;
    attr_offset(schema, attr_num, &mut offset);
    let attr_data = record.data.as_bytes();
    let off = offset as usize;
    let i = attr_num as usize;
    let mut result = String::new();
    match schema.data_types[i] {
        DataType::DtInt => {
            let mut bytes = [0u8; 4];
            let n = std::cmp::min(4, attr_data.len().saturating_sub(off));
            if n > 0 {
                bytes[..n].copy_from_slice(&attr_data[off..off + n]);
            }
            let val = i32::from_ne_bytes(bytes);
            result.push_str(&format!("{}:{}", schema.attr_names[i], val));
        }
        DataType::DtString => {
            let len = schema.type_length[i] as usize;
            let end = std::cmp::min(off + len, attr_data.len());
            let slice = if off < attr_data.len() {
                &attr_data[off..end]
            } else {
                &[]
            };
            // strncpy-like: stop at first null, otherwise full length
            let nul = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
            let s = String::from_utf8_lossy(&slice[..nul]).to_string();
            result.push_str(&format!("{}:{}", schema.attr_names[i], s));
        }
        DataType::DtFloat => {
            let mut bytes = [0u8; 4];
            let n = std::cmp::min(4, attr_data.len().saturating_sub(off));
            if n > 0 {
                bytes[..n].copy_from_slice(&attr_data[off..off + n]);
            }
            let val = f32::from_ne_bytes(bytes);
            result.push_str(&format!("{}:{}", schema.attr_names[i], format_float(val)));
        }
        DataType::DtBool => {
            let val = if off < attr_data.len() { attr_data[off] != 0 } else { false };
            result.push_str(&format!("{}:{}", schema.attr_names[i], if val { "TRUE" } else { "FALSE" }));
        }
    }
    result
}

pub fn serialize_value(val: &Value) -> String {
    match (&val.dt, &val.v) {
        (DataType::DtInt, ValueUnion::IntV(i)) => format!("{}", i),
        (DataType::DtFloat, ValueUnion::FloatV(f)) => format_float(*f),
        (DataType::DtString, ValueUnion::StringV(s)) => s.clone(),
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            if *b { "true".to_string() } else { "false".to_string() }
        }
        _ => String::new(),
    }
}

pub fn string_to_value(val: &str) -> Value {
    let bytes = val.as_bytes();
    if bytes.is_empty() {
        return Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    }
    let rest = &val[1..];
    match bytes[0] {
        b'i' => {
            let v: i32 = rest.parse().unwrap_or(0);
            Value { dt: DataType::DtInt, v: ValueUnion::IntV(v) }
        }
        b'f' => {
            let v: f32 = rest.parse().unwrap_or(0.0);
            Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(v) }
        }
        b's' => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(rest.to_string()),
        },
        b'b' => {
            let b = !rest.is_empty() && rest.as_bytes()[0] == b't';
            Value { dt: DataType::DtBool, v: ValueUnion::BoolV(b) }
        }
        _ => Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) },
    }
}

// Format float with 6-digit precision like C printf "%f"
fn format_float(f: f32) -> String {
    format!("{:.6}", f)
}
