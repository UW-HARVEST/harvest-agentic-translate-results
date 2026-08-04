use crate::{dberror::RC, tables::{RM_TableData, Schema, Record, Value}};
use crate::tables::{bytes_from_string, string_from_bytes, DataType, ValueUnion};
pub struct VarString {
pub buf: String,
pub size: usize,
pub bufsize: usize,
}
pub fn attr_offset(schema: &Schema, attr_num: i32, result: &mut i32) -> RC {
    let mut offset = 0;
    for index in 0..attr_num as usize {
        offset += match schema.data_types[index] {
            DataType::DtString => schema.type_length[index],
            DataType::DtInt => std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => std::mem::size_of::<f32>() as i32,
            DataType::DtBool => std::mem::size_of::<bool>() as i32,
        };
    }
    *result = offset;
    RC::Ok
}
pub fn serialize_table_info(rel: &RM_TableData) -> String {
    format!(
        "TABLE <{}> with <{}> tuples:\n{}",
        rel.name,
        crate::record_mgr::get_num_tuples(rel),
        serialize_schema(&rel.schema)
    )
}
pub fn serialize_table_content(rel: &RM_TableData) -> String {
    let mut result = String::new();
    for (index, name) in rel.schema.attr_names.iter().enumerate() {
        if index > 0 {
            result.push_str(", ");
        }
        result.push_str(name);
    }
    if let Some(state) = rel
        .mgmt_data
        .as_ref()
        .and_then(|value| value.downcast_ref::<std::rc::Rc<std::cell::RefCell<crate::record_mgr::TableState>>>())
    {
        let total = state.borrow().manager.total_tuples;
        let rec_size = state.borrow().manager.rec_size;
        let slots = ((crate::dberror::PAGE_SIZE as usize - 32) / (rec_size as usize + 2)) as i32;
        let mut seen = 0;
        let mut page = state.borrow().manager.first_data_page_num.max(0);
        let mut slot = 0;
        while seen < total {
            let rid = crate::tables::RID { page, slot };
            let mut record = crate::tables::Record {
                id: crate::tables::RID { page: -1, slot: -1 },
                data: string_from_bytes(vec![0_u8; rec_size as usize]),
            };
            if crate::record_mgr::get_record(rel, &rid, &mut record) == RC::Ok {
                result.push('\n');
                result.push_str(&serialize_record(&record, &rel.schema));
                seen += 1;
            }
            slot += 1;
            if slot >= slots {
                slot = 0;
                page += 1;
            }
        }
    }
    result
}
pub fn serialize_schema(schema: &Schema) -> String {
    let mut result = format!("Schema with <{}> attributes (", schema.num_attr);
    for index in 0..schema.num_attr as usize {
        if index > 0 {
            result.push_str(", ");
        }
        result.push_str(&schema.attr_names[index]);
        result.push_str(": ");
        match schema.data_types[index] {
            DataType::DtInt => result.push_str("INT"),
            DataType::DtFloat => result.push_str("FLOAT"),
            DataType::DtString => result.push_str(&format!("STRING[{}]", schema.type_length[index])),
            DataType::DtBool => result.push_str("BOOL"),
        }
    }
    result.push_str(") with keys: (");
    for (index, key) in schema.key_attrs.iter().enumerate() {
        if index > 0 {
            result.push_str(", ");
        }
        result.push_str(&schema.attr_names[*key as usize]);
    }
    result.push_str(")\n");
    result
}
pub fn serialize_record(record: &Record, schema: &Schema) -> String {
    let mut result = format!("[{}-{}] (", record.id.page, record.id.slot);
    for index in 0..schema.num_attr {
        result.push_str(&serialize_attr(record, schema, index));
        if index > 0 {
            result.push(',');
        }
    }
    result.push(')');
    result
}
pub fn serialize_attr(record: &Record, schema: &Schema, attr_num: i32) -> String {
    let mut offset = 0;
    let _ = attr_offset(schema, attr_num, &mut offset);
    let bytes = bytes_from_string(&record.data);
    let start = offset as usize;

    match schema.data_types[attr_num as usize] {
        DataType::DtInt => {
            let mut raw = [0_u8; 4];
            raw.copy_from_slice(&bytes[start..start + 4]);
            format!("{}:{}", schema.attr_names[attr_num as usize], i32::from_le_bytes(raw))
        }
        DataType::DtFloat => {
            let mut raw = [0_u8; 4];
            raw.copy_from_slice(&bytes[start..start + 4]);
            format!("{}:{:.6}", schema.attr_names[attr_num as usize], f32::from_le_bytes(raw))
        }
        DataType::DtBool => {
            let value = bytes.get(start).copied().unwrap_or(0) != 0;
            format!("{}:{}", schema.attr_names[attr_num as usize], if value { "TRUE" } else { "FALSE" })
        }
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let slice = &bytes[start..start + len];
            let end = slice.iter().position(|byte| *byte == 0).unwrap_or(slice.len());
            format!(
                "{}:{}",
                schema.attr_names[attr_num as usize],
                string_from_bytes(slice[..end].to_vec())
            )
        }
    }
}
pub fn serialize_value(val: &Value) -> String {
    match &val.v {
        ValueUnion::IntV(value) => value.to_string(),
        ValueUnion::FloatV(value) => format!("{value:.6}"),
        ValueUnion::StringV(value) => value.clone(),
        ValueUnion::BoolV(value) => {
            if *value { "true".to_string() } else { "false".to_string() }
        }
    }
}
pub fn string_to_value(val: &str) -> Value {
    match val.chars().next().unwrap_or('i') {
        'i' => Value { dt: DataType::DtInt, v: ValueUnion::IntV(val[1..].parse().unwrap_or(-1)) },
        'f' => Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(val[1..].parse().unwrap_or(0.0)) },
        's' => Value { dt: DataType::DtString, v: ValueUnion::StringV(val[1..].to_string()) },
        'b' => Value {
            dt: DataType::DtBool,
            v: ValueUnion::BoolV(val.as_bytes().get(1).copied() == Some(b't')),
        },
        _ => Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) },
    }
}
