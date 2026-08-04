use crate::{
    dberror::RC,
    record_mgr::{get_attr_pos, get_num_tuples, next, start_scan, RM_ScanHandle},
    tables::{
        bytes_to_data, clone_schema, data_to_bytes, read_bool, read_f32, read_i32, DataType,
        Record, RM_TableData, Schema, Value, ValueUnion,
    },
};

pub struct VarString {
    pub buf: String,
    pub size: usize,
    pub bufsize: usize,
}

pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    *result = get_attr_pos(schema, attr_num);
    RC::Ok
}

pub fn serialize_table_info(rel: &RM_TableData) -> String {
    format!(
        "TABLE <{}> with <{}> tuples:\n{}",
        rel.name,
        get_num_tuples(rel),
        serialize_schema(&rel.schema)
    )
}

pub fn serialize_table_content(rel: &RM_TableData) -> String {
    let mut out = String::new();
    for (idx, name) in rel.schema.attr_names.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(name);
    }

    let mut scan = RM_ScanHandle {
        rel: RM_TableData {
            name: rel.name.clone(),
            schema: clone_schema(&rel.schema),
            mgmt_data: None,
        },
        mgmt_data: None,
    };
    let _ = start_scan(rel, &mut scan, &crate::expr::Expr {
        expr_type: crate::expr::ExprType::ExprConst,
        expr: crate::expr::ExprUnion::Cons(Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(true),
        }),
    });

    let mut record = Record {
        id: Default::default(),
        data: bytes_to_data(&vec![0; crate::record_mgr::get_record_size(&rel.schema) as usize]),
    };
    while next(&mut scan, &mut record) == RC::Ok {
        out.push_str(&serialize_record(&record, &rel.schema));
        out.push('\n');
    }
    let _ = crate::record_mgr::close_scan(&mut scan);
    out
}

pub fn serialize_schema(schema: &Schema) -> String {
    let mut out = format!("Schema with <{}> attributes (", schema.num_attr);
    for i in 0..schema.num_attr as usize {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&schema.attr_names[i]);
        out.push_str(": ");
        match schema.data_types[i] {
            DataType::DtInt => out.push_str("INT"),
            DataType::DtFloat => out.push_str("FLOAT"),
            DataType::DtString => out.push_str(&format!("STRING[{}]", schema.type_length[i])),
            DataType::DtBool => out.push_str("BOOL"),
        }
    }
    out.push_str(") with keys: (");
    for i in 0..schema.key_size as usize {
        if i > 0 {
            out.push_str(", ");
        }
        let key_idx = schema.key_attrs[i] as usize;
        if let Some(name) = schema.attr_names.get(key_idx) {
            out.push_str(name);
        }
    }
    out.push_str(")\n");
    out
}

pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut out = format!("[{}-{}] (", record.id.page, record.id.slot);
    for i in 0..schema.num_attr {
        out.push_str(&serialize_attr(record, schema, i));
        if i != 0 {
            out.push(',');
        }
    }
    out.push(')');
    out
}

pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let offset = get_attr_pos(schema, attr_num) as usize;
    let bytes = data_to_bytes(&record.data);
    match schema.data_types[attr_num as usize] {
        DataType::DtInt => format!(
            "{}:{}",
            schema.attr_names[attr_num as usize],
            read_i32(&bytes, offset)
        ),
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let text = String::from_utf8(
                bytes[offset..offset + len]
                    .iter()
                    .copied()
                    .take_while(|b| *b != 0)
                    .collect(),
            )
            .unwrap_or_default();
            format!("{}:{}", schema.attr_names[attr_num as usize], text)
        }
        DataType::DtFloat => format!(
            "{}:{}",
            schema.attr_names[attr_num as usize],
            read_f32(&bytes, offset)
        ),
        DataType::DtBool => format!(
            "{}:{}",
            schema.attr_names[attr_num as usize],
            if read_bool(&bytes, offset) { "TRUE" } else { "FALSE" }
        ),
    }
}

pub fn serialize_value(val: &Value) -> String {
    match &val.v {
        ValueUnion::IntV(v) => format!("{v}"),
        ValueUnion::FloatV(v) => format!("{v:.6}"),
        ValueUnion::StringV(v) => v.clone(),
        ValueUnion::BoolV(v) => {
            if *v {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
    }
}

pub fn string_to_value(val: &str) -> Value {
    let mut chars = val.chars();
    match chars.next() {
        Some('i') => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(chars.as_str().parse().unwrap_or(-1)),
        },
        Some('f') => Value {
            dt: DataType::DtFloat,
            v: ValueUnion::FloatV(chars.as_str().parse().unwrap_or(0.0)),
        },
        Some('s') => Value {
            dt: DataType::DtString,
            v: ValueUnion::StringV(chars.as_str().to_string()),
        },
        Some('b') => Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(matches!(chars.as_str().chars().next(), Some('t' | 'T' | '1'))),
        },
        _ => Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        },
    }
}
