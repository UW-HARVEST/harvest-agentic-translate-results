use crate::{dberror::RC, expr::Expr, tables::{Record, Schema, RM_TableData, RID, DataType, Value, ValueUnion}};
use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle};

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
    RC::Ok
}

pub fn shutdown_record_manager() -> RC {
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

pub fn delete_table(_name: &str) -> RC {
    RC::Ok
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    if let Some(mgmt) = &rel.mgmt_data {
        if let Some(tm) = mgmt.downcast_ref::<TableManager>() {
            return tm.total_tuples;
        }
    }
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
            DataType::DtString => {
                total += schema.type_length[i];
            }
            DataType::DtInt => {
                total += 4;
            }
            DataType::DtFloat => {
                total += 4;
            }
            DataType::DtBool => {
                total += 1;
            }
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
    let data: String = (0..size).map(|_| '\0').collect();
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
    if attr_num < 0 || attr_num as usize >= schema.data_types.len() {
        return RC::Error;
    }
    let pos = get_attr_pos(schema, attr_num) as usize;
    let n = attr_num as usize;
    let bytes = record.data.as_bytes();
    value.dt = schema.data_types[n].clone();
    match schema.data_types[n] {
        DataType::DtString => {
            let len = schema.type_length[n] as usize;
            let mut s = String::new();
            for j in 0..len {
                if pos + j >= bytes.len() {
                    break;
                }
                let b = bytes[pos + j];
                if b == 0 {
                    break;
                }
                s.push(b as char);
            }
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtInt => {
            let mut buf = [0u8; 4];
            for j in 0..4 {
                if pos + j < bytes.len() {
                    buf[j] = bytes[pos + j];
                }
            }
            value.v = ValueUnion::IntV(i32::from_le_bytes(buf));
        }
        DataType::DtFloat => {
            let mut buf = [0u8; 4];
            for j in 0..4 {
                if pos + j < bytes.len() {
                    buf[j] = bytes[pos + j];
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
    if attr_num < 0 || attr_num as usize >= schema.data_types.len() {
        return RC::Error;
    }
    let pos = get_attr_pos(schema, attr_num) as usize;
    let n = attr_num as usize;
    let mut bytes: Vec<u8> = record.data.bytes().collect();
    match schema.data_types[n] {
        DataType::DtString => {
            let len = schema.type_length[n] as usize;
            if let ValueUnion::StringV(s) = &value.v {
                let sb = s.as_bytes();
                while bytes.len() < pos + len {
                    bytes.push(0);
                }
                for j in 0..len {
                    bytes[pos + j] = if j < sb.len() { sb[j] } else { 0 };
                }
            }
        }
        DataType::DtInt => {
            if let ValueUnion::IntV(v) = value.v {
                let arr = v.to_le_bytes();
                while bytes.len() < pos + 4 {
                    bytes.push(0);
                }
                for j in 0..4 {
                    bytes[pos + j] = arr[j];
                }
            }
        }
        DataType::DtFloat => {
            if let ValueUnion::FloatV(v) = value.v {
                let arr = v.to_le_bytes();
                while bytes.len() < pos + 4 {
                    bytes.push(0);
                }
                for j in 0..4 {
                    bytes[pos + j] = arr[j];
                }
            }
        }
        DataType::DtBool => {
            if let ValueUnion::BoolV(b) = value.v {
                while bytes.len() <= pos {
                    bytes.push(0);
                }
                bytes[pos] = if b { 1 } else { 0 };
            }
        }
    }
    record.data = bytes.into_iter().map(|b| b as char).collect();
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut pos: i32 = 0;
    for i in 0..attr_num as usize {
        match schema.data_types[i] {
            DataType::DtString => {
                pos += schema.type_length[i];
            }
            DataType::DtInt => {
                pos += 4;
            }
            DataType::DtFloat => {
                pos += 4;
            }
            DataType::DtBool => {
                pos += 1;
            }
        }
    }
    pos
}
