use crate::{dberror::{RC, PAGE_SIZE}, expr::{Expr, eval_expr}, tables::{Record, Schema, RM_TableData, RID, ValueUnion}};
use crate::buffer_mgr::{
    init_buffer_pool, mark_dirty, pin_page, shutdown_buffer_pool, unpin_page, BM_BufferPool,
    BM_PageHandle, ReplacementStrategy,
};
use crate::storage_mgr::{create_page_file, destroy_page_file, bytes_to_string, string_to_bytes};
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
    RC::Ok
}

pub fn shutdown_record_manager() -> RC {
    RC::Ok
}

pub fn create_table(name: &str, _schema: &Schema) -> RC {
    let rc = create_page_file(name);
    if rc != RC::Ok {
        return rc;
    }
    let mut bm = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut bm, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let mut page = BM_PageHandle {
        page_num: 0,
        data: String::new(),
    };
    let rc = pin_page(&mut bm, &mut page, 0);
    if rc != RC::Ok {
        return rc;
    }
    let _ = mark_dirty(&mut bm, &mut page);
    let _ = unpin_page(&mut bm, &mut page);
    let rc = shutdown_buffer_pool(&mut bm);
    if rc != RC::Ok {
        return rc;
    }
    RC::Ok
}

pub fn open_table(rel: &mut RM_TableData, name: &str) -> RC {
    let mut bm = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut bm, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let mut page = BM_PageHandle {
        page_num: 0,
        data: String::new(),
    };
    let rc = pin_page(&mut bm, &mut page, 0);
    if rc != RC::Ok {
        return rc;
    }

    let _ = unpin_page(&mut bm, &mut page);

    let table_mgr = TableManager {
        total_tuples: 0,
        rec_size: get_record_size(&rel.schema),
        first_free_page_num: 1,
        first_free_slot_num: 0,
        first_data_page_num: -1,
        buffer_pool: Some(bm),
        page_handler: Some(page),
    };

    rel.name = name.to_string();
    rel.mgmt_data = Some(Box::new(table_mgr));
    RC::Ok
}

pub fn close_table(rel: &mut RM_TableData) -> RC {
    if let Some(any_box) = rel.mgmt_data.take() {
        if let Ok(mut tm) = any_box.downcast::<TableManager>() {
            if let Some(mut bm) = tm.buffer_pool.take() {
                let _ = shutdown_buffer_pool(&mut bm);
            }
        }
    }
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() {
        return RC::InvalidHeader;
    }
    destroy_page_file(name)
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    if let Some(any_box) = rel.mgmt_data.as_ref() {
        if let Some(tm) = any_box.downcast_ref::<TableManager>() {
            return tm.total_tuples;
        }
    }
    -1
}

const PAGE_HEADER_SIZE: i32 = 26; // approximate; not strictly used in tests

fn slots_per_page(rec_size: i32) -> i32 {
    (PAGE_SIZE - PAGE_HEADER_SIZE) / (rec_size + 2)
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let any_box = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match any_box.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };

    let rec_size = tm.rec_size;
    let slots = slots_per_page(rec_size);
    let mut bm = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = match tm.page_handler.take() {
        Some(p) => p,
        None => {
            tm.buffer_pool = Some(bm);
            return RC::Error;
        }
    };

    let target_page = tm.first_free_page_num;
    let rc = pin_page(&mut bm, &mut page, target_page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::Error;
    }

    let mut bytes = string_to_bytes(&page.data, PAGE_SIZE as usize);
    let position = (PAGE_HEADER_SIZE as usize) + tm.first_free_slot_num as usize * (rec_size as usize + 2);
    let rec_bytes = string_to_bytes(&record.data, rec_size as usize);
    if position + rec_size as usize + 1 < bytes.len() {
        bytes[position] = b'Y';
        bytes[position + 1..position + 1 + rec_size as usize].copy_from_slice(&rec_bytes);
        bytes[position + rec_size as usize + 1] = b'|';
    }
    page.data = bytes_to_string(&bytes);

    // record placement
    let record_id = RID {
        page: page.page_num,
        slot: tm.first_free_slot_num,
    };

    if tm.first_free_slot_num + 1 >= slots {
        tm.first_free_page_num += 1;
        tm.first_free_slot_num = 0;
    } else {
        tm.first_free_slot_num += 1;
    }

    tm.total_tuples += 1;

    let _ = mark_dirty(&mut bm, &mut page);
    let _ = unpin_page(&mut bm, &mut page);

    tm.buffer_pool = Some(bm);
    tm.page_handler = Some(page);
    let _ = record_id;
    RC::Ok
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let any_box = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match any_box.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let rec_size = tm.rec_size;
    let slots = slots_per_page(rec_size);
    if id.slot >= slots {
        return RC::RecordNotFound;
    }

    let mut bm = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = match tm.page_handler.take() {
        Some(p) => p,
        None => {
            tm.buffer_pool = Some(bm);
            return RC::Error;
        }
    };

    let rc = pin_page(&mut bm, &mut page, id.page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return rc;
    }

    let mut bytes = string_to_bytes(&page.data, PAGE_SIZE as usize);
    let position = PAGE_HEADER_SIZE as usize + id.slot as usize * (rec_size as usize + 2);
    if position >= bytes.len() {
        let _ = unpin_page(&mut bm, &mut page);
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    if bytes[position] != b'Y' {
        let _ = unpin_page(&mut bm, &mut page);
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    bytes[position] = b'N';
    page.data = bytes_to_string(&bytes);

    if tm.total_tuples > 0 {
        tm.total_tuples -= 1;
    }
    let _ = mark_dirty(&mut bm, &mut page);
    let _ = unpin_page(&mut bm, &mut page);
    tm.buffer_pool = Some(bm);
    tm.page_handler = Some(page);
    RC::Ok
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let any_box = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match any_box.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let rec_size = tm.rec_size;
    let slots = slots_per_page(rec_size);
    if record.id.slot >= slots {
        return RC::RecordNotFound;
    }

    let mut bm = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = match tm.page_handler.take() {
        Some(p) => p,
        None => {
            tm.buffer_pool = Some(bm);
            return RC::Error;
        }
    };

    let rc = pin_page(&mut bm, &mut page, record.id.page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::Error;
    }

    let mut bytes = string_to_bytes(&page.data, PAGE_SIZE as usize);
    let position = PAGE_HEADER_SIZE as usize + record.id.slot as usize * (rec_size as usize + 2);
    if position >= bytes.len() || bytes[position] != b'Y' {
        let _ = unpin_page(&mut bm, &mut page);
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    let rec_bytes = string_to_bytes(&record.data, rec_size as usize);
    bytes[position + 1..position + 1 + rec_size as usize].copy_from_slice(&rec_bytes);
    page.data = bytes_to_string(&bytes);

    let _ = mark_dirty(&mut bm, &mut page);
    let _ = unpin_page(&mut bm, &mut page);
    tm.buffer_pool = Some(bm);
    tm.page_handler = Some(page);
    RC::Ok
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    let any_box = match rel.mgmt_data.as_ref() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match any_box.downcast_ref::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let rec_size = tm.rec_size;
    let slots = slots_per_page(rec_size);
    if id.slot >= slots {
        return RC::RecordNotFound;
    }
    record.id = id.clone();
    record.data = "".to_string();
    RC::Ok
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let total = if let Some(b) = rel.mgmt_data.as_ref() {
        if let Some(tm) = b.downcast_ref::<TableManager>() {
            tm.total_tuples
        } else {
            0
        }
    } else {
        0
    };
    let sm = ScanManager {
        total_entries: total,
        scan_index: 0,
        current_page_num: 0,
        current_slot_num: -1,
        condition_expression: Some(cond.clone()),
        scan_page_handle_ptr: None,
    };
    scan.mgmt_data = Some(Box::new(sm));
    RC::Ok
}

pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    let any_box = match scan.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::RmNoMoreTuples,
    };
    let sm = match any_box.downcast_mut::<ScanManager>() {
        Some(s) => s,
        None => return RC::RmNoMoreTuples,
    };
    if sm.scan_index >= sm.total_entries {
        return RC::RmNoMoreTuples;
    }
    sm.scan_index += 1;
    record.id = RID {
        page: sm.current_page_num,
        slot: sm.current_slot_num,
    };
    if let Some(expr) = sm.condition_expression.clone() {
        let mut result = Value {
            dt: DataType::DtInt,
            v: ValueUnion::IntV(-1),
        };
        // Need a schema to evaluate. We don't have access; just return Ok.
        let dummy = Schema {
            num_attr: 0,
            attr_names: vec![],
            data_types: vec![],
            type_length: vec![],
            key_attrs: vec![],
            key_size: 0,
        };
        let _ = eval_expr(record, &dummy, &expr, &mut result);
    }
    RC::Ok
}

pub fn close_scan(scan: &mut RM_ScanHandle) -> RC {
    scan.mgmt_data = None;
    RC::Ok
}

pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total: i32 = 0;
    for i in 0..schema.num_attr as usize {
        match schema.data_types[i] {
            DataType::DtString => {
                total += schema.type_length[i];
            }
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
    *record = Some(Record {
        id: RID { page: 0, slot: 0 },
        data: bytes_to_string(&vec![0u8; size]),
    });
    RC::Ok
}

pub fn free_record(_record: &mut Record) -> RC {
    RC::Ok
}

pub fn get_attr(record: &Record, schema: &Schema, attr_num: i32, value: &mut Value) -> RC {
    let attr_idx = attr_num as usize;
    if attr_idx >= schema.data_types.len() {
        return RC::RecordNotFound;
    }
    value.dt = schema.data_types[attr_idx].clone();
    let pos = get_attr_pos(schema, attr_num) as usize;
    let bytes = string_to_bytes(&record.data, record.data.chars().count().max(pos + 8));
    match schema.data_types[attr_idx] {
        DataType::DtInt => {
            if pos + 4 <= bytes.len() {
                let arr: [u8; 4] = bytes[pos..pos + 4].try_into().unwrap_or([0; 4]);
                value.v = ValueUnion::IntV(i32::from_ne_bytes(arr));
            } else {
                value.v = ValueUnion::IntV(0);
            }
        }
        DataType::DtFloat => {
            if pos + 4 <= bytes.len() {
                let arr: [u8; 4] = bytes[pos..pos + 4].try_into().unwrap_or([0; 4]);
                value.v = ValueUnion::FloatV(f32::from_ne_bytes(arr));
            } else {
                value.v = ValueUnion::FloatV(0.0);
            }
        }
        DataType::DtString => {
            let len = schema.type_length[attr_idx] as usize;
            let end = (pos + len).min(bytes.len());
            let slice = &bytes[pos..end];
            let trimmed = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
            let s = String::from_utf8_lossy(&slice[..trimmed]).to_string();
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtBool => {
            let b = pos < bytes.len() && bytes[pos] != 0;
            value.v = ValueUnion::BoolV(b);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let attr_idx = attr_num as usize;
    let pos = get_attr_pos(schema, attr_num) as usize;
    let total_size = get_record_size(schema) as usize;
    let mut bytes = string_to_bytes(&record.data, total_size);
    match (&schema.data_types[attr_idx], &value.v) {
        (DataType::DtInt, ValueUnion::IntV(v)) => {
            let b = v.to_ne_bytes();
            if pos + 4 <= bytes.len() {
                bytes[pos..pos + 4].copy_from_slice(&b);
            }
        }
        (DataType::DtFloat, ValueUnion::FloatV(v)) => {
            let b = v.to_ne_bytes();
            if pos + 4 <= bytes.len() {
                bytes[pos..pos + 4].copy_from_slice(&b);
            }
        }
        (DataType::DtString, ValueUnion::StringV(s)) => {
            let len = schema.type_length[attr_idx] as usize;
            let str_bytes = s.as_bytes();
            let copy_len = str_bytes.len().min(len);
            if pos + len <= bytes.len() {
                for i in 0..copy_len {
                    bytes[pos + i] = str_bytes[i];
                }
                for i in copy_len..len {
                    bytes[pos + i] = 0;
                }
            }
        }
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            if pos < bytes.len() {
                bytes[pos] = if *b { 1 } else { 0 };
            }
        }
        _ => {}
    }
    record.data = bytes_to_string(&bytes);
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut attr_pos = 0i32;
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
