use crate::{dberror::RC, tables::{DataType, RM_TableData, Schema, Record, Value, ValueUnion}};
pub struct VarString {
pub buf: String,
pub size: usize,
pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset: i32 = 0;
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
    // We don't have access to getNumTuples without mgmt_data; emulate the format.
    let num_tuples = 0;
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
    let attr_name = &schema.attr_names[attr_idx];
    // record.data is a String; just concatenate with offset descriptor
    // Without binary encoding, produce the textual form using attr_idx
    let _ = offset;
    let data_segment = record.data.as_str();
    match schema.data_types[attr_idx] {
        DataType::DtInt => {
            // Try to parse a chunk
            format!("{}:{}", attr_name, data_segment.parse::<i32>().unwrap_or(0))
        }
        DataType::DtString => {
            format!("{}:{}", attr_name, data_segment)
        }
        DataType::DtFloat => {
            format!("{}:{}", attr_name, data_segment.parse::<f32>().unwrap_or(0.0))
        }
        DataType::DtBool => {
            let b = data_segment == "t" || data_segment == "true" || data_segment == "1";
            format!("{}:{}", attr_name, if b { "TRUE" } else { "FALSE" })
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
            let b = !rest.is_empty() && (rest.as_bytes()[0] as char) == 't';
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
