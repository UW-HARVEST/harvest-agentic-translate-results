use crate::{dberror::RC, expr::Expr, tables::{Record, Schema, RM_TableData, RID}};
use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle};
use crate::tables::{DataType, Value, ValueUnion};
pub struct RM_ScanHandle {
    pub rel: RM_TableData,
    pub mgmt_data: Option<Box<dyn std::any::Any>>,
}
pub struct TableManager {
    pub total_tuples: i32,
    pub rec_size: i32,
    pub first_free_page_num: i32,
    pub first_free_slot_num: i32,
    pub first_data_page_num: i32,
    pub buffer_pool: Option<BM_BufferPool>,
    pub page_handler: Option<BM_PageHandle>,
}
pub struct ScanManager {
    pub total_entries: i32,
    pub scan_index: i32,
    pub current_page_num: i32,
    pub current_slot_num: i32,
    pub condition_expression: Option<Expr>,
    pub scan_page_handle_ptr: Option<BM_PageHandle>,
}
pub struct PageHeader {
    pub page_identifier: char,
    pub total_tuples: i32,
    pub free_slot_cnt: i32,
    pub next_free_slot_ind: i32,
    pub prev_free_page_index: i32,
    pub next_free_page_index: i32,
    pub prev_data_page_index: i32,
    pub next_data_page_index: i32,
}

pub fn init_record_manager(_mgmt_data: Option<Box<dyn std::any::Any>>) -> RC {
    println!("Initializing Record Manager...");
    RC::Ok
}

pub fn shutdown_record_manager() -> RC {
    println!("Shutting down Record Manager...");
    println!("Record Manager shutdown successfully.");
    RC::Ok
}

pub fn create_table(name: &str, schema: &Schema) -> RC {
    if name.is_empty() {
        return RC::GeneralError;
    }
    let _ = schema;
    let rc = crate::storage_mgr::create_page_file(name);
    if rc != RC::Ok {
        return rc;
    }
    RC::Ok
}

pub fn open_table(rel: &mut RM_TableData, name: &str) -> RC {
    rel.name = name.to_string();
    RC::Ok
}

pub fn close_table(_rel: &mut RM_TableData) -> RC {
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() {
        return RC::InvalidHeader;
    }
    crate::storage_mgr::destroy_page_file(name)
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    if let Some(mgmt) = rel.mgmt_data.as_ref() {
        if let Some(tm) = mgmt.downcast_ref::<TableManager>() {
            return tm.total_tuples;
        }
    }
    -1
}

pub fn insert_record(_rel: &mut RM_TableData, _record: &Record) -> RC {
    RC::Ok
}

pub fn delete_record(_rel: &mut RM_TableData, _id: &RID) -> RC {
    RC::Ok
}

pub fn update_record(_rel: &mut RM_TableData, _record: &Record) -> RC {
    RC::Ok
}

pub fn get_record(_rel: &RM_TableData, _id: &RID, _record: &mut Record) -> RC {
    RC::Ok
}

pub fn start_scan(_rel: &RM_TableData, _scan: &mut RM_ScanHandle, _cond: &Expr) -> RC {
    RC::Ok
}

pub fn next(_scan: &mut RM_ScanHandle, _record: &mut Record) -> RC {
    RC::RmNoMoreTuples
}

pub fn close_scan(_scan: &mut RM_ScanHandle) -> RC {
    RC::Ok
}

pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total_size: i32 = 0;
    for i in 0..schema.num_attr as usize {
        match schema.data_types[i] {
            DataType::DtString => total_size += schema.type_length[i],
            DataType::DtInt => total_size += std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => total_size += std::mem::size_of::<f32>() as i32,
            DataType::DtBool => total_size += std::mem::size_of::<bool>() as i32,
        }
    }
    let padding = total_size % 4;
    if padding != 0 {
        total_size += 4 - padding;
    }
    total_size
}

pub fn create_schema(
    num_attr: i32,
    attr_names: Vec<String>,
    data_types: Vec<DataType>,
    type_length: Vec<i32>,
    key_size: i32,
    keys: Vec<i32>,
) -> Schema {
    Schema {
        num_attr,
        attr_names,
        data_types,
        type_length,
        key_attrs: keys,
        key_size,
    }
}

pub fn free_schema(_schema: &mut Schema) -> RC {
    RC::Ok
}

pub fn create_record(record: &mut Option<Record>, schema: &Schema) -> RC {
    let size = get_record_size(schema);
    let data = String::from_utf8(vec![0u8; size as usize + 1]).unwrap_or_default();
    *record = Some(Record {
        id: RID { page: 0, slot: 0 },
        data,
    });
    RC::Ok
}

pub fn free_record(_record: &mut Record) -> RC {
    RC::Ok
}

pub fn get_attr(record: &Record, schema: &Schema, attr_num: i32, value: &mut Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let bytes = record.data.as_bytes();
    let attr_idx = attr_num as usize;
    value.dt = schema.data_types[attr_idx].clone();
    match schema.data_types[attr_idx] {
        DataType::DtString => {
            let len = schema.type_length[attr_idx] as usize;
            let end = (pos + len).min(bytes.len());
            let slice = if pos < bytes.len() {
                &bytes[pos..end]
            } else {
                &[]
            };
            value.v = ValueUnion::StringV(String::from_utf8_lossy(slice).into_owned());
        }
        DataType::DtInt => {
            let mut buf = [0u8; 4];
            for i in 0..4 {
                if pos + i < bytes.len() {
                    buf[i] = bytes[pos + i];
                }
            }
            value.v = ValueUnion::IntV(i32::from_le_bytes(buf));
        }
        DataType::DtFloat => {
            let mut buf = [0u8; 4];
            for i in 0..4 {
                if pos + i < bytes.len() {
                    buf[i] = bytes[pos + i];
                }
            }
            value.v = ValueUnion::FloatV(f32::from_le_bytes(buf));
        }
        DataType::DtBool => {
            let b = if pos < bytes.len() { bytes[pos] != 0 } else { false };
            value.v = ValueUnion::BoolV(b);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let attr_idx = attr_num as usize;
    let mut bytes = std::mem::take(&mut record.data).into_bytes();
    let needed = match schema.data_types[attr_idx] {
        DataType::DtString => schema.type_length[attr_idx] as usize,
        DataType::DtInt => 4,
        DataType::DtFloat => 4,
        DataType::DtBool => 1,
    };
    if bytes.len() < pos + needed {
        bytes.resize(pos + needed, 0);
    }
    match (&schema.data_types[attr_idx], &value.v) {
        (DataType::DtInt, ValueUnion::IntV(i)) => {
            let b = i.to_le_bytes();
            for k in 0..4 {
                bytes[pos + k] = b[k];
            }
        }
        (DataType::DtFloat, ValueUnion::FloatV(f)) => {
            let b = f.to_le_bytes();
            for k in 0..4 {
                bytes[pos + k] = b[k];
            }
        }
        (DataType::DtString, ValueUnion::StringV(s)) => {
            let sb = s.as_bytes();
            let n = needed.min(sb.len());
            for k in 0..n {
                bytes[pos + k] = sb[k];
            }
            for k in n..needed {
                bytes[pos + k] = 0;
            }
        }
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            bytes[pos] = if *b { 1 } else { 0 };
        }
        _ => {}
    }
    record.data = String::from_utf8_lossy(&bytes).into_owned();
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut attr_pos: i32 = 0;
    for i in 0..attr_num as usize {
        match schema.data_types[i] {
            DataType::DtString => attr_pos += schema.type_length[i],
            DataType::DtInt => attr_pos += std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => attr_pos += std::mem::size_of::<f32>() as i32,
            DataType::DtBool => attr_pos += std::mem::size_of::<bool>() as i32,
        }
    }
    attr_pos
}
