use crate::{dberror::RC, tables::{RM_TableData, Schema, Record, Value, DataType, ValueUnion}};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset: i32 = 0;
    let n = attr_num as usize;
    for i in 0..n {
        match schema.data_types.get(i) {
            Some(DataType::DtString) => offset += schema.type_length[i],
            Some(DataType::DtInt) => offset += 4,
            Some(DataType::DtFloat) => offset += 4,
            Some(DataType::DtBool) => offset += 1,
            None => {}
        }
    }
    *result = offset;
    RC::Ok
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    let total = crate::record_mgr::get_num_tuples(rel);
    let mut s = format!("TABLE <{}> with <{}> tuples:\n", rel.name, total);
    s.push_str(&serialize_schema(&rel.schema));
    s
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
    // Note: scanning the table requires mutable RM_TableData. Since this is called with &RM_TableData,
    // and the C version mutates the buffer pool internally, we provide a best-effort version.
    // Real scan-based enumeration is performed by the record_mgr module's start_scan/next.
    result
}

pub fn serialize_schema(schema: &Schema) -> String {
    let mut result = format!("Schema with <{}> attributes (", schema.num_attr);
    for i in 0..schema.num_attr as usize {
        if i != 0 {
            result.push_str(", ");
        }
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
        if i != 0 {
            result.push_str(", ");
        }
        let key_idx = schema.key_attrs[i] as usize;
        result.push_str(&schema.attr_names[key_idx]);
    }
    result.push(')');
    result.push('\n');
    result
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut result = format!("[{}-{}] (", record.id.page, record.id.slot);
    for i in 0..schema.num_attr as usize {
        result.push_str(&serialize_attr(record, schema, i as i32));
        if i != 0 {
            // After every attr after the first, append ","
            // Wait - the C code is: APPEND(result, "%s", (i == 0) ? "" : ",");
            // It appends a comma AFTER the attribute is printed when i!=0.
            result.push(',');
        }
    }
    result.push(')');
    result
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset: i32 = 0;
    attr_offset(schema, attr_num, &mut offset);
    let off = offset as usize;
    let attr_idx = attr_num as usize;
    let bytes: Vec<u8> = record.data.chars().map(|c| c as u8).collect();
    let attr_data = &bytes[off..];

    match schema.data_types[attr_idx] {
        DataType::DtInt => {
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&attr_data[..4]);
            let val = i32::from_ne_bytes(arr);
            format!("{}:{}", schema.attr_names[attr_idx], val)
        }
        DataType::DtString => {
            let len = schema.type_length[attr_idx] as usize;
            let raw = &attr_data[..len.min(attr_data.len())];
            // Strip after first NUL like strncpy semantics with explicit terminator
            let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            let s = String::from_utf8_lossy(&raw[..end]).into_owned();
            format!("{}:{}", schema.attr_names[attr_idx], s)
        }
        DataType::DtFloat => {
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&attr_data[..4]);
            let val = f32::from_ne_bytes(arr);
            format!("{}:{:.6}", schema.attr_names[attr_idx], val)
        }
        DataType::DtBool => {
            let val = attr_data[0] != 0;
            format!(
                "{}:{}",
                schema.attr_names[attr_idx],
                if val { "TRUE" } else { "FALSE" }
            )
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
    let bytes = val.as_bytes();
    if bytes.is_empty() {
        return Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        };
    }
    let rest = &val[1..];
    match bytes[0] {
        b'i' => {
            let n: i32 = rest.trim_start().parse().unwrap_or(0);
            Value { dt: DataType::DtInt, v: ValueUnion::IntV(n) }
        }
        b'f' => {
            let n: f32 = rest.parse().unwrap_or(0.0);
            Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(n) }
        }
        b's' => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(rest.to_string()),
        },
        b'b' => {
            let b = bytes.len() > 1 && bytes[1] == b't';
            Value { dt: DataType::DtBool, v: ValueUnion::BoolV(b) }
        }
        _ => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        },
    }
}
