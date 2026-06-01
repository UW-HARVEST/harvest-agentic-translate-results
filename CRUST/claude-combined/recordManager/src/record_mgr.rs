use crate::{dberror::RC, expr::Expr, tables::{Record, Schema, RM_TableData, RID, ValueUnion}};
use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle};
use crate::tables::{DataType, Value};
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
    RC::Ok
}

pub fn create_table(_name: &str, _schema: &Schema) -> RC {
    RC::Ok
}

pub fn open_table(_rel: &mut RM_TableData, _name: &str) -> RC {
    RC::Ok
}

pub fn close_table(_rel: &mut RM_TableData) -> RC {
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() {
        return RC::InvalidHeader;
    }
    match std::fs::remove_file(name) {
        Ok(_) => RC::Ok,
        Err(_) => RC::DestroyFailed,
    }
}

pub fn get_num_tuples(_rel: &RM_TableData) -> i32 {
    0
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

pub fn create_schema(num_attr: i32, attr_names: Vec<String>, data_types: Vec<DataType>, type_length: Vec<i32>, key_size: i32, keys: Vec<i32>) -> Schema {
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
    let data: String = "\0".repeat(size);
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
    let pos = get_attr_pos(schema, attr_num) as usize;
    let dt = schema.data_types[attr_num as usize].clone();
    let bytes = record.data.as_bytes();
    match dt {
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let end = std::cmp::min(pos + len, bytes.len());
            let s = String::from_utf8_lossy(&bytes[pos..end]).into_owned();
            *value = Value { dt: DataType::DtString, v: ValueUnion::StringV(s) };
        }
        DataType::DtInt => {
            let v = if pos + 4 <= bytes.len() {
                i32::from_ne_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]])
            } else { 0 };
            *value = Value { dt: DataType::DtInt, v: ValueUnion::IntV(v) };
        }
        DataType::DtFloat => {
            let v = if pos + 4 <= bytes.len() {
                f32::from_ne_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]])
            } else { 0.0 };
            *value = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(v) };
        }
        DataType::DtBool => {
            let v = if pos < bytes.len() { bytes[pos] != 0 } else { false };
            *value = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(v) };
        }
    }
    RC::Ok
}

pub fn set_attr(_record: &mut Record, _schema: &Schema, _attr_num: i32, _value: &Value) -> RC {
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut pos: i32 = 0;
    for i in 0..attr_num as usize {
        match schema.data_types[i] {
            DataType::DtString => pos += schema.type_length[i],
            DataType::DtInt => pos += std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => pos += std::mem::size_of::<f32>() as i32,
            DataType::DtBool => pos += std::mem::size_of::<bool>() as i32,
        }
    }
    pos
}
