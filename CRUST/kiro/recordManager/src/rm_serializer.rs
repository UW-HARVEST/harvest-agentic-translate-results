use crate::{dberror::RC, tables::{RM_TableData, Schema, Record, Value, DataType, ValueUnion, RID}};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset = 0i32;
    for i in 0..attr_num as usize {
        match schema.data_types[i] {
            DataType::DtString => offset += schema.type_length[i],
            DataType::DtInt => offset += 4,
            DataType::DtFloat => offset += 4,
            DataType::DtBool => offset += 2, // C bool is short (2 bytes)
        }
    }
    *result = offset;
    RC::Ok
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    let num_tuples = crate::record_mgr::get_num_tuples(rel);
    format!("TABLE <{}> with <{}> tuples:\n{}", rel.name, num_tuples, serialize_schema(&rel.schema))
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    let mut result = String::new();
    for i in 0..rel.schema.num_attr as usize {
        if i != 0 { result.push_str(", "); }
        result.push_str(&rel.schema.attr_names[i]);
    }
    // Scan through records
    let mut scan = crate::record_mgr::RM_ScanHandle {
        rel: RM_TableData {
            name: rel.name.clone(),
            schema: rel.schema.clone(),
            mgmt_data: None,
        },
        mgmt_data: None,
    };
    // We can't easily do a scan here without the table being open, so this is a simplified version
    // In practice, this would need the table's mgmt_data
    result
}

pub fn serialize_schema(schema: &Schema) -> String {
    let mut result = format!("Schema with <{}> attributes (", schema.num_attr);
    for i in 0..schema.num_attr as usize {
        if i != 0 { result.push_str(", "); }
        result.push_str(&schema.attr_names[i]);
        result.push_str(": ");
        match schema.data_types[i] {
            DataType::DtInt => result.push_str("INT"),
            DataType::DtFloat => result.push_str("FLOAT"),
            DataType::DtString => result.push_str(&format!("STRING[{}]", schema.type_length[i])),
            DataType::DtBool => result.push_str("BOOL"),
        }
    }
    result.push_str(") with keys: (");
    for i in 0..schema.key_size as usize {
        if i != 0 { result.push_str(", "); }
        result.push_str(&schema.attr_names[schema.key_attrs[i] as usize]);
    }
    result.push_str(")\n");
    result
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut result = format!("[{}-{}] (", record.id.page, record.id.slot);
    for i in 0..schema.num_attr as usize {
        result.push_str(&serialize_attr(record, schema, i as i32));
        if i == 0 { /* no comma before first */ } else { result.push(','); }
    }
    result.push(')');
    result
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset = 0i32;
    attr_offset(schema, attr_num, &mut offset);
    let offset = offset as usize;
    let data_chars: Vec<char> = record.data.chars().collect();

    match schema.data_types[attr_num as usize] {
        DataType::DtInt => {
            let bytes: Vec<u8> = (0..4).map(|i| {
                if offset + i < data_chars.len() { data_chars[offset + i] as u8 } else { 0 }
            }).collect();
            let val = i32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            format!("{}:{}", schema.attr_names[attr_num as usize], val)
        }
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let s: String = (0..len).map(|i| {
                if offset + i < data_chars.len() { data_chars[offset + i] } else { '\0' }
            }).collect();
            let s = s.trim_end_matches('\0');
            format!("{}:{}", schema.attr_names[attr_num as usize], s)
        }
        DataType::DtFloat => {
            let bytes: Vec<u8> = (0..4).map(|i| {
                if offset + i < data_chars.len() { data_chars[offset + i] as u8 } else { 0 }
            }).collect();
            let val = f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            format!("{}:{:.6}", schema.attr_names[attr_num as usize], val)
        }
        DataType::DtBool => {
            let bytes: Vec<u8> = (0..2).map(|i| {
                if offset + i < data_chars.len() { data_chars[offset + i] as u8 } else { 0 }
            }).collect();
            let val = i16::from_ne_bytes([bytes[0], bytes[1]]);
            let bval = val != 0;
            format!("{}:{}", schema.attr_names[attr_num as usize], if bval { "TRUE" } else { "FALSE" })
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
        return Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    }
    match val.as_bytes()[0] {
        b'i' => Value { dt: DataType::DtInt, v: ValueUnion::IntV(val[1..].parse().unwrap_or(0)) },
        b'f' => Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(val[1..].parse().unwrap_or(0.0)) },
        b's' => Value { dt: DataType::DtString, v: ValueUnion::StringV(val[1..].to_string()) },
        b'b' => {
            let bval = val.as_bytes().get(1) == Some(&b't');
            Value { dt: DataType::DtBool, v: ValueUnion::BoolV(bval) }
        }
        _ => Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) },
    }
}
