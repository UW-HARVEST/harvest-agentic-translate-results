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
    for i in 0..n {
        match schema.data_types[i] {
            DataType::DtString => offset += schema.type_length[i],
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
    use crate::record_mgr::{close_scan, create_record, next, start_scan};
    let mut result = String::new();
    for (i, name) in rel.schema.attr_names.iter().enumerate() {
        if i != 0 {
            result.push_str(", ");
        }
        result.push_str(name);
    }
    // We need a scan handle and a record. Use the public API.
    let mut scan = crate::record_mgr::RM_ScanHandle {
        rel: crate::tables::RM_TableData {
            name: rel.name.clone(),
            schema: rel.schema.clone(),
            mgmt_data: None,
        },
        mgmt_data: None,
    };
    // Provide a no-condition scan: pass a constant true expression.
    // start_scan takes Expr; we'll create a const true and a "no-cond" handling.
    // The C version passes NULL for the condition; we replicate that by using a
    // sentinel "always-true" boolean-constant expression.
    let cond = crate::expr::Expr {
        expr_type: crate::expr::ExprType::ExprConst,
        expr: crate::expr::ExprUnion::Cons(Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(true),
        }),
    };
    let _ = start_scan(rel, &mut scan, &cond);
    let mut record: Option<Record> = None;
    if create_record(&mut record, &rel.schema) == RC::Ok {
        if let Some(mut r) = record.take() {
            loop {
                let rc = next(&mut scan, &mut r);
                if rc != RC::Ok {
                    break;
                }
                result.push_str(&serialize_record(&r, &rel.schema));
                result.push('\n');
            }
        }
    }
    let _ = close_scan(&mut scan);
    result
}

pub fn serialize_schema(schema: &Schema) -> String {
    let mut result = String::new();
    result.push_str(&format!("Schema with <{}> attributes (", schema.num_attr));
    for i in 0..(schema.num_attr as usize) {
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
    for i in 0..(schema.key_size as usize) {
        if i != 0 {
            result.push_str(", ");
        }
        let attr_idx = schema.key_attrs[i] as usize;
        result.push_str(&schema.attr_names[attr_idx]);
    }
    result.push_str(")\n");
    result
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut result = String::new();
    result.push_str(&format!("[{}-{}] (", record.id.page, record.id.slot));
    for i in 0..(schema.num_attr as usize) {
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
    let bytes = record.data.as_bytes();
    let off = offset as usize;
    let idx = attr_num as usize;
    let attr_name = &schema.attr_names[idx];
    match schema.data_types[idx] {
        DataType::DtInt => {
            let mut buf = [0u8; 4];
            let take = (bytes.len() - off).min(4);
            buf[..take].copy_from_slice(&bytes[off..off + take]);
            let val = i32::from_ne_bytes(buf);
            format!("{}:{}", attr_name, val)
        }
        DataType::DtString => {
            let len = schema.type_length[idx] as usize;
            let end = (off + len).min(bytes.len());
            let raw = &bytes[off..end];
            // Mimic C's strncpy: terminate at the first null byte.
            let truncated: Vec<u8> = raw.iter().cloned().take_while(|&b| b != 0).collect();
            let s = String::from_utf8_lossy(&truncated);
            format!("{}:{}", attr_name, s)
        }
        DataType::DtFloat => {
            let mut buf = [0u8; 4];
            let take = (bytes.len() - off).min(4);
            buf[..take].copy_from_slice(&bytes[off..off + take]);
            let val = f32::from_ne_bytes(buf);
            // Mimic C's "%f" -> 6 decimal places
            format!("{}:{:.6}", attr_name, val)
        }
        DataType::DtBool => {
            let take = (bytes.len() - off).min(1);
            let val = if take > 0 { bytes[off] != 0 } else { false };
            format!("{}:{}", attr_name, if val { "TRUE" } else { "FALSE" })
        }
    }
}

pub fn serialize_value(val: &Value) -> String {
    match (&val.dt, &val.v) {
        (DataType::DtInt, ValueUnion::IntV(i)) => format!("{}", i),
        (DataType::DtFloat, ValueUnion::FloatV(f)) => format!("{:.6}", f),
        (DataType::DtString, ValueUnion::StringV(s)) => s.clone(),
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            (if *b { "true" } else { "false" }).to_string()
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
    let prefix = val.chars().next().unwrap();
    let rest = &val[1..];
    match prefix {
        'i' => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(parse_atoi(rest)),
        },
        'f' => Value {
            dt: DataType::DtFloat,
            v: ValueUnion::FloatV(parse_atof(rest)),
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

fn parse_atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut sign = 1i64;
    let mut chars = s.chars().peekable();
    if let Some(&c) = chars.peek() {
        if c == '-' {
            sign = -1;
            chars.next();
        } else if c == '+' {
            chars.next();
        }
    }
    let mut n: i64 = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            n = n.saturating_mul(10).saturating_add(d as i64);
        } else {
            break;
        }
    }
    (sign * n) as i32
}

fn parse_atof(s: &str) -> f32 {
    // Mimic C's atof: parse leading numeric prefix.
    let s = s.trim_start();
    let bytes = s.as_bytes();
    let mut idx = 0;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        idx += 1;
    }
    let mut saw_digit = false;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
        saw_digit = true;
    }
    if idx < bytes.len() && bytes[idx] == b'.' {
        idx += 1;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
            saw_digit = true;
        }
    }
    if idx < bytes.len() && (bytes[idx] == b'e' || bytes[idx] == b'E') {
        idx += 1;
        if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
            idx += 1;
        }
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
    }
    if !saw_digit {
        return 0.0;
    }
    s[..idx].parse::<f32>().unwrap_or(0.0)
}
