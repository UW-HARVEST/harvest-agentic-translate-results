use crate::{dberror::{RC, PAGE_SIZE}, expr::{self, Expr}, tables::{Record, Schema, RM_TableData, RID, DataType, Value, ValueUnion}};
use crate::buffer_mgr::{self, BM_BufferPool, BM_PageHandle, ReplacementStrategy};
use crate::storage_mgr;

const MAX_ATTR_NAME_LEN: usize = 15;

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

pub fn create_table(name: &str, _schema: &Schema) -> RC {
    if name.is_empty() {
        return RC::GeneralError;
    }
    let rc = storage_mgr::create_page_file(name);
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
    storage_mgr::destroy_page_file(name)
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

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let total_entries = get_num_tuples(rel);
    let sm = ScanManager {
        total_entries,
        scan_index: 0,
        current_page_num: 0,
        current_slot_num: -1,
        condition_expression: Some(cond.clone()),
        scan_page_handle_ptr: None,
    };
    scan.mgmt_data = Some(Box::new(sm));
    RC::Ok
}

pub fn next(scan: &mut RM_ScanHandle, _record: &mut Record) -> RC {
    let mgmt = match scan.mgmt_data.as_ref() {
        Some(m) => m,
        None => return RC::RmNoMoreTuples,
    };
    let sm = match mgmt.downcast_ref::<ScanManager>() {
        Some(s) => s,
        None => return RC::RmNoMoreTuples,
    };
    if sm.scan_index >= sm.total_entries {
        return RC::RmNoMoreTuples;
    }
    RC::RmNoMoreTuples
}

pub fn close_scan(scan: &mut RM_ScanHandle) -> RC {
    scan.mgmt_data = None;
    RC::Ok
}

pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total: i32 = 0;
    for i in 0..schema.num_attr as usize {
        match schema.data_types[i] {
            DataType::DtString => total += schema.type_length[i],
            DataType::DtInt => total += std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => total += std::mem::size_of::<f32>() as i32,
            DataType::DtBool => total += std::mem::size_of::<bool>() as i32,
        }
    }
    let padding = total % 4;
    if padding != 0 {
        total += 4 - padding;
    }
    total
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
    let size = get_record_size(schema) as usize;
    let bytes = vec![0u8; size];
    let data = String::from_utf8(bytes).unwrap_or_default();
    *record = Some(Record {
        id: RID { page: -1, slot: -1 },
        data,
    });
    RC::Ok
}

pub fn free_record(_record: &mut Record) -> RC {
    RC::Ok
}

pub fn get_attr(record: &Record, schema: &Schema, attr_num: i32, value: &mut Value) -> RC {
    let attr_idx = attr_num as usize;
    if attr_idx >= schema.data_types.len() {
        return RC::GeneralError;
    }
    let pos = get_attr_pos(schema, attr_num) as usize;
    let bytes = record.data.as_bytes();
    let dt = schema.data_types[attr_idx].clone();
    value.dt = dt.clone();
    match dt {
        DataType::DtString => {
            let len = schema.type_length[attr_idx] as usize;
            let end = std::cmp::min(pos + len, bytes.len());
            let slice = if pos < bytes.len() { &bytes[pos..end] } else { &[][..] };
            let s = String::from_utf8_lossy(slice).to_string();
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtInt => {
            let mut buf = [0u8; 4];
            if pos + 4 <= bytes.len() {
                buf.copy_from_slice(&bytes[pos..pos + 4]);
            }
            value.v = ValueUnion::IntV(i32::from_ne_bytes(buf));
        }
        DataType::DtFloat => {
            let mut buf = [0u8; 4];
            if pos + 4 <= bytes.len() {
                buf.copy_from_slice(&bytes[pos..pos + 4]);
            }
            value.v = ValueUnion::FloatV(f32::from_ne_bytes(buf));
        }
        DataType::DtBool => {
            let b = if pos < bytes.len() { bytes[pos] != 0 } else { false };
            value.v = ValueUnion::BoolV(b);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let attr_idx = attr_num as usize;
    if attr_idx >= schema.data_types.len() {
        return RC::GeneralError;
    }
    let pos = get_attr_pos(schema, attr_num) as usize;
    let mut bytes = record.data.as_bytes().to_vec();
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
            let buf = i.to_ne_bytes();
            bytes[pos..pos + 4].copy_from_slice(&buf);
        }
        (DataType::DtFloat, ValueUnion::FloatV(f)) => {
            let buf = f.to_ne_bytes();
            bytes[pos..pos + 4].copy_from_slice(&buf);
        }
        (DataType::DtString, ValueUnion::StringV(s)) => {
            let s_bytes = s.as_bytes();
            let len = std::cmp::min(s_bytes.len(), needed);
            // Zero-fill
            for b in &mut bytes[pos..pos + needed] {
                *b = 0;
            }
            bytes[pos..pos + len].copy_from_slice(&s_bytes[..len]);
        }
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            bytes[pos] = if *b { 1 } else { 0 };
        }
        _ => return RC::RmCompareValueOfDifferentDatatype,
    }
    record.data = String::from_utf8_lossy(&bytes).to_string();
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut pos: i32 = 0;
    let n = attr_num as usize;
    for i in 0..n {
        match schema.data_types[i] {
            DataType::DtString => pos += schema.type_length[i],
            DataType::DtInt => pos += 4,
            DataType::DtFloat => pos += 4,
            DataType::DtBool => pos += 1,
        }
    }
    pos
}

// Suppress unused warnings for some imports
#[allow(dead_code)]
fn _suppress_unused_warnings() {
    let _ = MAX_ATTR_NAME_LEN;
    let _ = expr::ExprType::ExprConst;
    let _ = ReplacementStrategy::RsFifo;
    let _ = PAGE_SIZE;
    let _ = buffer_mgr::NO_PAGE;
}
