use crate::{dberror::RC, tables::{RM_TableData, Schema, Record, Value, ValueUnion, DataType}};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset = 0i32;
    for attr_pos in 0..attr_num as usize {
        match schema.data_types[attr_pos] {
            DataType::DtString => {
                offset += schema.type_length[attr_pos];
            }
            DataType::DtInt => {
                offset += std::mem::size_of::<i32>() as i32;
            }
            DataType::DtFloat => {
                offset += std::mem::size_of::<f32>() as i32;
            }
            DataType::DtBool => {
                offset += std::mem::size_of::<bool>() as i32;
            }
        }
    }
    *result = offset;
    RC::Ok
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    let mut result = String::new();
    result.push_str(&format!(
        "TABLE <{}> with <{}> tuples:\n",
        rel.name, 0
    ));
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
        result.push_str(&schema.attr_names[schema.key_attrs[i] as usize]);
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
    let mut offset = 0i32;
    attr_offset(schema, attr_num, &mut offset);
    let offset = offset as usize;
    let attr_idx = attr_num as usize;
    let data_bytes = record.data.as_bytes();

    match schema.data_types[attr_idx] {
        DataType::DtInt => {
            let mut bytes = [0u8; 4];
            if offset + 4 <= data_bytes.len() {
                bytes.copy_from_slice(&data_bytes[offset..offset + 4]);
            }
            let val = i32::from_ne_bytes(bytes);
            format!("{}:{}", schema.attr_names[attr_idx], val)
        }
        DataType::DtString => {
            let len = schema.type_length[attr_idx] as usize;
            let end = (offset + len).min(data_bytes.len());
            let slice = &data_bytes[offset..end];
            // Trim trailing null characters
            let trimmed_end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
            let s = String::from_utf8_lossy(&slice[..trimmed_end]).to_string();
            format!("{}:{}", schema.attr_names[attr_idx], s)
        }
        DataType::DtFloat => {
            let mut bytes = [0u8; 4];
            if offset + 4 <= data_bytes.len() {
                bytes.copy_from_slice(&data_bytes[offset..offset + 4]);
            }
            let val = f32::from_ne_bytes(bytes);
            format!("{}:{:.6}", schema.attr_names[attr_idx], val)
        }
        DataType::DtBool => {
            let val = if offset < data_bytes.len() {
                data_bytes[offset] != 0
            } else {
                false
            };
            format!(
                "{}:{}",
                schema.attr_names[attr_idx],
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
    if val.is_empty() {
        return Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        };
    }
    let bytes = val.as_bytes();
    let first = bytes[0] as char;
    let rest = &val[1..];
    match first {
        'i' => {
            let i: i32 = rest.parse().unwrap_or(0);
            Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(i),
            }
        }
        'f' => {
            let f: f32 = rest.parse().unwrap_or(0.0);
            Value {
                dt: DataType::DtFloat,
                v: ValueUnion::FloatV(f),
            }
        }
        's' => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(rest.to_string()),
        },
        'b' => {
            let is_true = !rest.is_empty() && rest.as_bytes()[0] == b't';
            Value {
                dt: DataType::DtBool,
                v: ValueUnion::BoolV(is_true),
            }
        }
        _ => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        },
    }
}
