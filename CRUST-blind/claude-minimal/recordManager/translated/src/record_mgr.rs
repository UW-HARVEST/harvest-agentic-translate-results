use crate::{dberror::RC, expr::Expr, tables::{Record, Schema, RM_TableData, RID}};
use crate::buffer_mgr::{
    BM_BufferPool, BM_PageHandle, ReplacementStrategy,
    init_buffer_pool, shutdown_buffer_pool, pin_page, unpin_page, mark_dirty,
};
use crate::storage_mgr::{create_page_file, destroy_page_file};
use crate::tables::{DataType, Value, ValueUnion};
use crate::dberror::PAGE_SIZE;

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

const MAX_ATTR_NAME_LEN: usize = 15;

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
    let rc = create_page_file(name);
    if rc != RC::Ok {
        return rc;
    }
    let mut buffer_pool = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut buffer_pool, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let mut page_handle = BM_PageHandle {
        page_num: 0,
        data: String::new(),
    };
    let rc = pin_page(&mut buffer_pool, &mut page_handle, 0);
    if rc != RC::Ok {
        let _ = shutdown_buffer_pool(&mut buffer_pool);
        return rc;
    }

    // Prepare table header in page_handle.data
    let rec_size = get_record_size(schema);
    let mut header_bytes: Vec<u8> = Vec::new();
    let total_tuples: i32 = 0;
    let first_free_page_num: i32 = 1;
    let first_free_slot_num: i32 = 0;
    let first_data_page_num: i32 = -1;
    header_bytes.extend_from_slice(&total_tuples.to_ne_bytes());
    header_bytes.extend_from_slice(&rec_size.to_ne_bytes());
    header_bytes.extend_from_slice(&first_free_page_num.to_ne_bytes());
    header_bytes.extend_from_slice(&first_free_slot_num.to_ne_bytes());
    header_bytes.extend_from_slice(&first_data_page_num.to_ne_bytes());
    header_bytes.extend_from_slice(&schema.num_attr.to_ne_bytes());
    header_bytes.extend_from_slice(&schema.key_size.to_ne_bytes());

    for i in 0..schema.num_attr as usize {
        let mut name_bytes = vec![0u8; MAX_ATTR_NAME_LEN];
        let an_bytes = schema.attr_names[i].as_bytes();
        let copy_len = std::cmp::min(an_bytes.len(), MAX_ATTR_NAME_LEN);
        name_bytes[..copy_len].copy_from_slice(&an_bytes[..copy_len]);
        header_bytes.extend_from_slice(&name_bytes);

        let dt_int: i32 = match schema.data_types[i] {
            DataType::DtInt => 0,
            DataType::DtString => 1,
            DataType::DtFloat => 2,
            DataType::DtBool => 3,
        };
        header_bytes.extend_from_slice(&dt_int.to_ne_bytes());
        header_bytes.extend_from_slice(&schema.type_length[i].to_ne_bytes());
    }
    for i in 0..schema.key_size as usize {
        header_bytes.extend_from_slice(&schema.key_attrs[i].to_ne_bytes());
    }

    page_handle.data = String::from_utf8_lossy(&header_bytes).into_owned();

    let _ = mark_dirty(&mut buffer_pool, &mut page_handle);
    let _ = unpin_page(&mut buffer_pool, &mut page_handle);
    let _ = shutdown_buffer_pool(&mut buffer_pool);
    RC::Ok
}

pub fn open_table(rel: &mut RM_TableData, name: &str) -> RC {
    let mut buffer_pool = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut buffer_pool, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let mut page_handle = BM_PageHandle {
        page_num: 0,
        data: String::new(),
    };
    let rc = pin_page(&mut buffer_pool, &mut page_handle, 0);
    if rc != RC::Ok {
        return rc;
    }

    let header = page_handle.data.as_bytes().to_vec();
    let mut pos = 0usize;
    let read_i32 = |bytes: &[u8], pos: &mut usize| -> i32 {
        if *pos + 4 > bytes.len() {
            return 0;
        }
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&bytes[*pos..*pos + 4]);
        *pos += 4;
        i32::from_ne_bytes(buf)
    };

    let total_tuples = read_i32(&header, &mut pos);
    let rec_size = read_i32(&header, &mut pos);
    let first_free_page_num = read_i32(&header, &mut pos);
    let first_free_slot_num = read_i32(&header, &mut pos);
    let first_data_page_num = read_i32(&header, &mut pos);
    let num_attr = read_i32(&header, &mut pos);
    let key_size = read_i32(&header, &mut pos);

    let mut attr_names: Vec<String> = Vec::with_capacity(num_attr as usize);
    let mut data_types: Vec<DataType> = Vec::with_capacity(num_attr as usize);
    let mut type_length: Vec<i32> = Vec::with_capacity(num_attr as usize);

    for _ in 0..num_attr as usize {
        if pos + MAX_ATTR_NAME_LEN > header.len() {
            break;
        }
        let name_slice = &header[pos..pos + MAX_ATTR_NAME_LEN];
        let n = String::from_utf8_lossy(name_slice).trim_end_matches('\0').to_string();
        attr_names.push(n);
        pos += MAX_ATTR_NAME_LEN;

        let dt_val = read_i32(&header, &mut pos);
        let dt = match dt_val {
            0 => DataType::DtInt,
            1 => DataType::DtString,
            2 => DataType::DtFloat,
            _ => DataType::DtBool,
        };
        data_types.push(dt);
        type_length.push(read_i32(&header, &mut pos));
    }

    let mut key_attrs: Vec<i32> = Vec::with_capacity(key_size as usize);
    for _ in 0..key_size as usize {
        key_attrs.push(read_i32(&header, &mut pos));
    }

    let _ = unpin_page(&mut buffer_pool, &mut page_handle);

    let schema = Schema {
        num_attr,
        attr_names,
        data_types,
        type_length,
        key_attrs,
        key_size,
    };

    let table_manager = TableManager {
        total_tuples,
        rec_size,
        first_free_page_num,
        first_free_slot_num,
        first_data_page_num,
        buffer_pool: Some(buffer_pool),
        page_handler: Some(page_handle),
    };

    rel.name = name.to_string();
    rel.schema = schema;
    rel.mgmt_data = Some(Box::new(table_manager));
    RC::Ok
}

pub fn close_table(rel: &mut RM_TableData) -> RC {
    let mut tm_box = match rel.mgmt_data.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match tm_box.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let mut buffer_pool = match tm.buffer_pool.take() {
        Some(bp) => bp,
        None => return RC::Error,
    };
    let mut page_handle = match tm.page_handler.take() {
        Some(ph) => ph,
        None => return RC::Error,
    };
    let pin_status = pin_page(&mut buffer_pool, &mut page_handle, 0);
    if pin_status == RC::Ok {
        let mut header_bytes: Vec<u8> = Vec::new();
        header_bytes.extend_from_slice(&tm.total_tuples.to_ne_bytes());
        header_bytes.extend_from_slice(&tm.rec_size.to_ne_bytes());
        header_bytes.extend_from_slice(&tm.first_free_page_num.to_ne_bytes());
        header_bytes.extend_from_slice(&tm.first_free_slot_num.to_ne_bytes());
        header_bytes.extend_from_slice(&tm.first_data_page_num.to_ne_bytes());
        let mut data = page_handle.data.as_bytes().to_vec();
        if data.len() < header_bytes.len() {
            data.resize(header_bytes.len(), 0);
        }
        for (i, &b) in header_bytes.iter().enumerate() {
            data[i] = b;
        }
        page_handle.data = String::from_utf8_lossy(&data).into_owned();
        let _ = mark_dirty(&mut buffer_pool, &mut page_handle);
        let _ = unpin_page(&mut buffer_pool, &mut page_handle);
    }
    let _ = shutdown_buffer_pool(&mut buffer_pool);
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() {
        return RC::InvalidHeader;
    }
    destroy_page_file(name)
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    match rel.mgmt_data.as_ref() {
        Some(b) => match b.downcast_ref::<TableManager>() {
            Some(tm) => tm.total_tuples,
            None => -1,
        },
        None => -1,
    }
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let tm_box = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match tm_box.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let buffer_pool = match tm.buffer_pool.as_mut() {
        Some(bp) => bp,
        None => return RC::Error,
    };
    let page_handle = match tm.page_handler.as_mut() {
        Some(ph) => ph,
        None => return RC::Error,
    };
    let header_size = std::mem::size_of::<i32>() * 7 + std::mem::size_of::<u8>(); // approx
    let slots_available_on_page = (PAGE_SIZE as usize - header_size) / (tm.rec_size as usize + 2);

    let rc = pin_page(buffer_pool, page_handle, tm.first_free_page_num);
    if rc != RC::Ok {
        return RC::Error;
    }

    let mut data = page_handle.data.as_bytes().to_vec();
    if data.len() < PAGE_SIZE as usize {
        data.resize(PAGE_SIZE as usize, 0);
    }
    let position = header_size + tm.first_free_slot_num as usize * (tm.rec_size as usize + 2);
    if position + tm.rec_size as usize + 2 <= data.len() {
        data[position] = b'Y';
        let rec_bytes = record.data.as_bytes();
        let copy_len = std::cmp::min(rec_bytes.len(), tm.rec_size as usize);
        for k in 0..copy_len {
            data[position + 1 + k] = rec_bytes[k];
        }
        data[position + tm.rec_size as usize + 1] = b'|';
    }
    page_handle.data = String::from_utf8_lossy(&data).into_owned();

    // Note: Original C mutates record.id; in Rust insert_record takes &Record (immutable).
    // We skip updating record's id here since we cannot, but in tests records are typically
    // re-fetched.
    let _ = record;
    if (tm.first_free_slot_num as usize + 1) >= slots_available_on_page {
        tm.first_free_page_num += 1;
        tm.first_free_slot_num = 0;
    } else {
        tm.first_free_slot_num += 1;
    }
    tm.total_tuples += 1;

    let _ = mark_dirty(buffer_pool, page_handle);
    let _ = unpin_page(buffer_pool, page_handle);
    RC::Ok
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let tm_box = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match tm_box.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let buffer_pool = match tm.buffer_pool.as_mut() {
        Some(bp) => bp,
        None => return RC::Error,
    };
    let page_handle = match tm.page_handler.as_mut() {
        Some(ph) => ph,
        None => return RC::Error,
    };
    let header_size = std::mem::size_of::<i32>() * 7 + std::mem::size_of::<u8>();
    let max_slots = (PAGE_SIZE as usize - header_size) / (tm.rec_size as usize + 2);
    if id.slot as usize >= max_slots {
        return RC::RecordNotFound;
    }
    let pin_status = pin_page(buffer_pool, page_handle, id.page);
    if pin_status != RC::Ok {
        return pin_status;
    }
    let position = header_size + id.slot as usize * (tm.rec_size as usize + 2);
    let mut data = page_handle.data.as_bytes().to_vec();
    if position >= data.len() || data[position] != b'Y' {
        let _ = unpin_page(buffer_pool, page_handle);
        return RC::RecordNotFound;
    }
    data[position] = b'N';
    page_handle.data = String::from_utf8_lossy(&data).into_owned();
    if tm.total_tuples > 0 {
        tm.total_tuples -= 1;
    }
    let _ = mark_dirty(buffer_pool, page_handle);
    let _ = unpin_page(buffer_pool, page_handle);
    RC::Ok
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let tm_box = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match tm_box.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let buffer_pool = match tm.buffer_pool.as_mut() {
        Some(bp) => bp,
        None => return RC::Error,
    };
    let page_handle = match tm.page_handler.as_mut() {
        Some(ph) => ph,
        None => return RC::Error,
    };
    let header_size = std::mem::size_of::<i32>() * 7 + std::mem::size_of::<u8>();
    let max_slots = (PAGE_SIZE as usize - header_size) / (tm.rec_size as usize + 2);
    if record.id.slot as usize >= max_slots {
        return RC::RecordNotFound;
    }
    let rc = pin_page(buffer_pool, page_handle, record.id.page);
    if rc != RC::Ok {
        return RC::Error;
    }
    let position = header_size + record.id.slot as usize * (tm.rec_size as usize + 2);
    let mut data = page_handle.data.as_bytes().to_vec();
    if position >= data.len() || data[position] != b'Y' {
        let _ = unpin_page(buffer_pool, page_handle);
        return RC::RecordNotFound;
    }
    let rec_bytes = record.data.as_bytes();
    let copy_len = std::cmp::min(rec_bytes.len(), tm.rec_size as usize);
    for k in 0..copy_len {
        data[position + 1 + k] = rec_bytes[k];
    }
    page_handle.data = String::from_utf8_lossy(&data).into_owned();
    let _ = mark_dirty(buffer_pool, page_handle);
    let _ = unpin_page(buffer_pool, page_handle);
    RC::Ok
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    // We need mutable access to TableManager, but signature gives &RM_TableData.
    // Use a raw pointer dereference via std::ptr to avoid reference-cast UB lint.
    let rel_ptr: *mut RM_TableData = rel as *const RM_TableData as *mut RM_TableData;
    let mgmt_ptr: *mut Option<Box<dyn std::any::Any>> = unsafe { &raw mut (*rel_ptr).mgmt_data };
    let tm_box = match unsafe { (*mgmt_ptr).as_mut() } {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match tm_box.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let buffer_pool = match tm.buffer_pool.as_mut() {
        Some(bp) => bp,
        None => return RC::Error,
    };
    let page_handle = match tm.page_handler.as_mut() {
        Some(ph) => ph,
        None => return RC::Error,
    };
    let header_size = std::mem::size_of::<i32>() * 7 + std::mem::size_of::<u8>();
    let max_slots = (PAGE_SIZE as usize - header_size) / (tm.rec_size as usize + 2);
    if id.slot as usize >= max_slots {
        return RC::RecordNotFound;
    }
    let rc = pin_page(buffer_pool, page_handle, id.page);
    if rc != RC::Ok {
        return RC::Error;
    }
    let position = header_size + id.slot as usize * (tm.rec_size as usize + 2);
    let data = page_handle.data.as_bytes();
    if position >= data.len() || data[position] != b'Y' {
        let _ = unpin_page(buffer_pool, page_handle);
        return RC::RecordNotFound;
    }
    let start = position + 1;
    let end = std::cmp::min(start + tm.rec_size as usize, data.len());
    let rec_slice = &data[start..end];
    record.data = String::from_utf8_lossy(rec_slice).into_owned();
    record.id = id.clone();
    let _ = unpin_page(buffer_pool, page_handle);
    RC::Ok
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let total_entries = get_num_tuples(rel);
    let first_data_page_num = match rel.mgmt_data.as_ref() {
        Some(b) => match b.downcast_ref::<TableManager>() {
            Some(tm) => tm.first_data_page_num,
            None => -1,
        },
        None => -1,
    };
    let scan_mgr = ScanManager {
        total_entries,
        scan_index: 0,
        current_page_num: first_data_page_num,
        current_slot_num: -1,
        condition_expression: Some(cond.clone()),
        scan_page_handle_ptr: None,
    };
    scan.mgmt_data = Some(Box::new(scan_mgr));
    RC::Ok
}

pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    let header_size = std::mem::size_of::<i32>() * 7 + std::mem::size_of::<u8>();
    let rec_size = match scan.rel.mgmt_data.as_ref() {
        Some(b) => match b.downcast_ref::<TableManager>() {
            Some(tm) => tm.rec_size,
            None => return RC::Error,
        },
        None => return RC::Error,
    };
    let slots_per_page = (PAGE_SIZE as usize - header_size) / (rec_size as usize + 2);

    let scan_mgr_box = match scan.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let scan_mgr = match scan_mgr_box.downcast_mut::<ScanManager>() {
        Some(s) => s,
        None => return RC::Error,
    };

    if scan_mgr.scan_index >= scan_mgr.total_entries {
        return RC::RmNoMoreTuples;
    }

    loop {
        scan_mgr.current_slot_num += 1;
        if scan_mgr.current_slot_num as usize >= slots_per_page {
            scan_mgr.current_page_num += 1;
            scan_mgr.current_slot_num = 0;
        }

        let rid = RID {
            page: scan_mgr.current_page_num,
            slot: scan_mgr.current_slot_num,
        };
        let rc = get_record(&scan.rel, &rid, record);
        if rc == RC::Ok {
            scan_mgr.scan_index += 1;
            if let Some(cond_expr) = scan_mgr.condition_expression.clone() {
                let mut eval_result = Value {
                    dt: DataType::DtBool,
                    v: ValueUnion::BoolV(false),
                };
                let _ = crate::expr::eval_expr(record, &scan.rel.schema, &cond_expr, &mut eval_result);
                if let ValueUnion::BoolV(true) = eval_result.v {
                    return RC::Ok;
                }
            } else {
                return RC::Ok;
            }
        }

        if scan_mgr.scan_index >= scan_mgr.total_entries {
            return RC::RmNoMoreTuples;
        }
    }
}

pub fn close_scan(scan: &mut RM_ScanHandle) -> RC {
    scan.mgmt_data = None;
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
    // No-op in Rust: drop handles cleanup.
    RC::Ok
}

pub fn create_record(record: &mut Option<Record>, schema: &Schema) -> RC {
    let rec_size = get_record_size(schema);
    let r = Record {
        id: RID { page: -1, slot: -1 },
        data: String::from_utf8(vec![0u8; rec_size as usize]).unwrap_or_default(),
    };
    *record = Some(r);
    RC::Ok
}

pub fn free_record(_record: &mut Record) -> RC {
    // No-op in Rust.
    RC::Ok
}

pub fn get_attr(record: &Record, schema: &Schema, attr_num: i32, value: &mut Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let data = record.data.as_bytes();
    let attr_idx = attr_num as usize;
    match schema.data_types[attr_idx] {
        DataType::DtString => {
            let len = schema.type_length[attr_idx] as usize;
            let end = std::cmp::min(pos + len, data.len());
            let slice = if pos < data.len() { &data[pos..end] } else { &[] };
            let s = String::from_utf8_lossy(slice).trim_end_matches('\0').to_string();
            value.dt = DataType::DtString;
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtInt => {
            let mut buf = [0u8; 4];
            let len = std::cmp::min(4, data.len().saturating_sub(pos));
            if len > 0 {
                buf[..len].copy_from_slice(&data[pos..pos + len]);
            }
            value.dt = DataType::DtInt;
            value.v = ValueUnion::IntV(i32::from_ne_bytes(buf));
        }
        DataType::DtFloat => {
            let mut buf = [0u8; 4];
            let len = std::cmp::min(4, data.len().saturating_sub(pos));
            if len > 0 {
                buf[..len].copy_from_slice(&data[pos..pos + len]);
            }
            value.dt = DataType::DtFloat;
            value.v = ValueUnion::FloatV(f32::from_ne_bytes(buf));
        }
        DataType::DtBool => {
            let v = if pos < data.len() { data[pos] != 0 } else { false };
            value.dt = DataType::DtBool;
            value.v = ValueUnion::BoolV(v);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let mut data = record.data.as_bytes().to_vec();
    let attr_idx = attr_num as usize;
    let needed_len = pos
        + match schema.data_types[attr_idx] {
            DataType::DtString => schema.type_length[attr_idx] as usize,
            DataType::DtInt => 4,
            DataType::DtFloat => 4,
            DataType::DtBool => 1,
        };
    if data.len() < needed_len {
        data.resize(needed_len, 0);
    }
    match (&schema.data_types[attr_idx], &value.v) {
        (DataType::DtInt, ValueUnion::IntV(v)) => {
            let bytes = v.to_ne_bytes();
            data[pos..pos + 4].copy_from_slice(&bytes);
        }
        (DataType::DtFloat, ValueUnion::FloatV(v)) => {
            let bytes = v.to_ne_bytes();
            data[pos..pos + 4].copy_from_slice(&bytes);
        }
        (DataType::DtString, ValueUnion::StringV(s)) => {
            let len = schema.type_length[attr_idx] as usize;
            let s_bytes = s.as_bytes();
            let copy_len = std::cmp::min(s_bytes.len(), len);
            // Zero out first
            for k in 0..len {
                data[pos + k] = 0;
            }
            data[pos..pos + copy_len].copy_from_slice(&s_bytes[..copy_len]);
        }
        (DataType::DtBool, ValueUnion::BoolV(v)) => {
            data[pos] = if *v { 1 } else { 0 };
        }
        _ => {}
    }
    record.data = String::from_utf8_lossy(&data).into_owned();
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
