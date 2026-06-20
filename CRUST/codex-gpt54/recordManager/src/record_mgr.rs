use crate::{dberror::RC, expr::Expr, tables::{Record, Schema, RM_TableData, RID}};
use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle};
use crate::tables::{bytes_from_string, fixed_len_bytes, string_from_bytes, DataType, Value, ValueUnion};
use crate::{buffer_mgr, expr, storage_mgr};
use std::cell::RefCell;
use std::rc::Rc;
pub struct RM_ScanHandle {
    pub rel: RM_TableData,
    pub mgmt_data: Option<Box<dyn std::any::Any>>,
}
pub struct TableManager{
    pub total_tuples: i32,
    pub rec_size : i32,
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
pub(crate) struct TableState {
    pub(crate) manager: TableManager,
}

struct ScanState {
    total_entries: i32,
    scan_index: i32,
    current_page_num: i32,
    current_slot_num: i32,
    condition_expression: Expr,
}

pub fn init_record_manager(mgmt_data: Option<Box<dyn std::any::Any>>) -> RC {
    let _ = mgmt_data;
    RC::Ok
}
pub fn shutdown_record_manager() -> RC {
    RC::Ok
}
pub fn create_table(name: &str, schema: &Schema) -> RC {
    let rc = storage_mgr::create_page_file(name);
    if rc != RC::Ok {
        return rc;
    }

    let mut buffer_pool = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: buffer_mgr::ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let mut page_handle = BM_PageHandle { page_num: 0, data: string_from_bytes(vec![0; crate::dberror::PAGE_SIZE as usize]) };
    let rc = buffer_mgr::init_buffer_pool(&mut buffer_pool, name, 3, buffer_mgr::ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let rc = buffer_mgr::pin_page(&mut buffer_pool, &mut page_handle, 0);
    if rc != RC::Ok {
        return rc;
    }

    let mut page_bytes = bytes_from_string(&page_handle.data);
    page_bytes.resize(crate::dberror::PAGE_SIZE as usize, 0);
    write_i32(&mut page_bytes, 0, 0);
    write_i32(&mut page_bytes, 4, get_record_size(schema));
    write_i32(&mut page_bytes, 8, 1);
    write_i32(&mut page_bytes, 12, 0);
    write_i32(&mut page_bytes, 16, -1);
    write_i32(&mut page_bytes, 20, schema.num_attr);
    write_i32(&mut page_bytes, 24, schema.key_size);
    let mut offset = 28;
    for index in 0..schema.num_attr as usize {
        page_bytes[offset..offset + 15].copy_from_slice(&fixed_len_bytes(&schema.attr_names[index], 15));
        offset += 15;
        write_i32(&mut page_bytes, offset, data_type_code(&schema.data_types[index]));
        offset += 4;
        write_i32(&mut page_bytes, offset, schema.type_length[index]);
        offset += 4;
    }
    for key in &schema.key_attrs {
        write_i32(&mut page_bytes, offset, *key);
        offset += 4;
    }
    page_handle.data = string_from_bytes(page_bytes);
    let rc = buffer_mgr::mark_dirty(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        return rc;
    }
    let rc = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        return rc;
    }
    buffer_mgr::shutdown_buffer_pool(&mut buffer_pool)
}
pub fn open_table(rel: &mut RM_TableData, name: &str) -> RC {
    let mut buffer_pool = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: buffer_mgr::ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let rc = buffer_mgr::init_buffer_pool(&mut buffer_pool, name, 3, buffer_mgr::ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let mut page_handle = BM_PageHandle { page_num: 0, data: String::new() };
    let rc = buffer_mgr::pin_page(&mut buffer_pool, &mut page_handle, 0);
    if rc != RC::Ok {
        return rc;
    }

    let page = bytes_from_string(&page_handle.data);
    let total_tuples = read_i32(&page, 0);
    let rec_size = read_i32(&page, 4);
    let first_free_page_num = read_i32(&page, 8);
    let first_free_slot_num = read_i32(&page, 12);
    let first_data_page_num = read_i32(&page, 16);
    let num_attr = read_i32(&page, 20);
    let key_size = read_i32(&page, 24);
    let mut offset = 28;
    let mut attr_names = Vec::with_capacity(num_attr as usize);
    let mut data_types = Vec::with_capacity(num_attr as usize);
    let mut type_length = Vec::with_capacity(num_attr as usize);
    for _ in 0..num_attr {
        let raw_name = &page[offset..offset + 15];
        let name_end = raw_name.iter().position(|byte| *byte == 0).unwrap_or(15);
        attr_names.push(string_from_bytes(raw_name[..name_end].to_vec()));
        offset += 15;
        data_types.push(match read_i32(&page, offset) {
            0 => DataType::DtInt,
            1 => DataType::DtString,
            2 => DataType::DtFloat,
            _ => DataType::DtBool,
        });
        offset += 4;
        type_length.push(read_i32(&page, offset));
        offset += 4;
    }
    let mut key_attrs = Vec::with_capacity(key_size as usize);
    for _ in 0..key_size {
        key_attrs.push(read_i32(&page, offset));
        offset += 4;
    }
    let rc = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        return rc;
    }

    rel.name = name.to_string();
    rel.schema = Schema {
        num_attr,
        attr_names,
        data_types,
        type_length,
        key_attrs,
        key_size,
    };
    let state = Rc::new(RefCell::new(TableState {
        manager: TableManager {
            total_tuples,
            rec_size,
            first_free_page_num,
            first_free_slot_num,
            first_data_page_num,
            buffer_pool: Some(buffer_pool),
            page_handler: Some(page_handle),
        },
    }));
    rel.mgmt_data = Some(Box::new(state));
    RC::Ok
}
pub fn close_table(rel: &mut RM_TableData) -> RC {
    let Some(state) = get_table_state(rel) else {
        return RC::Error;
    };
    {
        let mut state = state.borrow_mut();
        let total_tuples = state.manager.total_tuples;
        let rec_size = state.manager.rec_size;
        let first_free_page_num = state.manager.first_free_page_num;
        let first_free_slot_num = state.manager.first_free_slot_num;
        let first_data_page_num = state.manager.first_data_page_num;
        let Some(mut buffer_pool) = state.manager.buffer_pool.take() else {
            return RC::Error;
        };
        let mut page_handle = state.manager.page_handler.take().unwrap_or(BM_PageHandle { page_num: 0, data: String::new() });
        let rc = buffer_mgr::pin_page(&mut buffer_pool, &mut page_handle, 0);
        if rc != RC::Ok {
            state.manager.buffer_pool = Some(buffer_pool);
            state.manager.page_handler = Some(page_handle);
            return rc;
        }
        let mut page = bytes_from_string(&page_handle.data);
        page.resize(crate::dberror::PAGE_SIZE as usize, 0);
        write_i32(&mut page, 0, total_tuples);
        write_i32(&mut page, 4, rec_size);
        write_i32(&mut page, 8, first_free_page_num);
        write_i32(&mut page, 12, first_free_slot_num);
        write_i32(&mut page, 16, first_data_page_num);
        page_handle.data = string_from_bytes(page);
        let dirty_rc = buffer_mgr::mark_dirty(&mut buffer_pool, &mut page_handle);
        if dirty_rc != RC::Ok {
            state.manager.buffer_pool = Some(buffer_pool);
            state.manager.page_handler = Some(page_handle);
            return dirty_rc;
        }
        let unpin_rc = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
        if unpin_rc != RC::Ok {
            state.manager.buffer_pool = Some(buffer_pool);
            state.manager.page_handler = Some(page_handle);
            return unpin_rc;
        }
        let shutdown_rc = buffer_mgr::shutdown_buffer_pool(&mut buffer_pool);
        if shutdown_rc != RC::Ok {
            state.manager.buffer_pool = Some(buffer_pool);
            state.manager.page_handler = Some(page_handle);
            return shutdown_rc;
        }
    }
    rel.mgmt_data = None;
    RC::Ok
}
pub fn delete_table(name: &str) -> RC {
    storage_mgr::destroy_page_file(name)
}
pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    get_table_state(rel)
        .map(|state| state.borrow().manager.total_tuples)
        .unwrap_or(-1)
}
pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let Some(state) = get_table_state(rel) else {
        return RC::Error;
    };
    let mut state = state.borrow_mut();
    let slots = slots_per_page(state.manager.rec_size);
    let page_num = state.manager.first_free_page_num;
    let rec_size = state.manager.rec_size;
    let Some(mut buffer_pool) = state.manager.buffer_pool.take() else {
        return RC::Error;
    };
    let mut page_handle = state.manager.page_handler.take().unwrap_or(BM_PageHandle { page_num, data: String::new() });
    let rc = buffer_mgr::pin_page(&mut buffer_pool, &mut page_handle, page_num);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    let mut page = bytes_from_string(&page_handle.data);
    page.resize(crate::dberror::PAGE_SIZE as usize, 0);
    let mut header = decode_page_header(&page);
    if header.page_identifier != 'Y' {
        header.page_identifier = 'Y';
        header.total_tuples = 0;
        header.free_slot_cnt = slots - 1;
        header.next_free_slot_ind = 1;
        header.prev_free_page_index = -1;
        header.next_free_page_index = page_num + 1;
        header.prev_data_page_index = -1;
        header.next_data_page_index = 1;
    } else {
        header.total_tuples += 1;
        header.free_slot_cnt -= 1;
        if header.free_slot_cnt > 0 {
            header.next_free_slot_ind += 1;
        } else {
            header.next_free_slot_ind = -header.next_free_slot_ind;
        }
    }
    encode_page_header(&mut page, &header);
    let slot = state.manager.first_free_slot_num;
    let record_offset = page_header_size() + slot as usize * (rec_size as usize + 2);
    page[record_offset] = b'Y';
    let record_bytes = record_data_bytes(record, rec_size as usize);
    page[record_offset + 1..record_offset + 1 + rec_size as usize].copy_from_slice(&record_bytes);
    page[record_offset + 1 + rec_size as usize] = b'|';

    if header.free_slot_cnt == 0 {
        state.manager.first_free_page_num += 1;
        state.manager.first_free_slot_num = 0;
    } else {
        state.manager.first_free_slot_num += 1;
    }
    state.manager.total_tuples += 1;
    page_handle.page_num = page_num;
    page_handle.data = string_from_bytes(page);
    let rc = buffer_mgr::mark_dirty(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    let rc = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    state.manager.buffer_pool = Some(buffer_pool);
    state.manager.page_handler = Some(page_handle);
    RC::Ok
}
pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let Some(state) = get_table_state(rel) else {
        return RC::Error;
    };
    let mut state = state.borrow_mut();
    if id.slot >= slots_per_page(state.manager.rec_size) {
        return RC::RecordNotFound;
    }
    let rec_size = state.manager.rec_size;
    let Some(mut buffer_pool) = state.manager.buffer_pool.take() else {
        return RC::Error;
    };
    let mut page_handle = state.manager.page_handler.take().unwrap_or(BM_PageHandle { page_num: id.page, data: String::new() });
    let rc = buffer_mgr::pin_page(&mut buffer_pool, &mut page_handle, id.page);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    let mut page = bytes_from_string(&page_handle.data);
    let record_offset = page_header_size() + id.slot as usize * (rec_size as usize + 2);
    if page.get(record_offset).copied() != Some(b'Y') {
        let _ = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return RC::RecordNotFound;
    }
    page[record_offset] = b'N';
    let mut header = decode_page_header(&page);
    header.total_tuples = (header.total_tuples - 1).max(0);
    header.free_slot_cnt += 1;
    encode_page_header(&mut page, &header);
    state.manager.total_tuples = (state.manager.total_tuples - 1).max(0);
    page_handle.data = string_from_bytes(page);
    let rc = buffer_mgr::mark_dirty(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    let rc = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    state.manager.buffer_pool = Some(buffer_pool);
    state.manager.page_handler = Some(page_handle);
    RC::Ok
}
pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let Some(state) = get_table_state(rel) else {
        return RC::Error;
    };
    let mut state = state.borrow_mut();
    if record.id.slot >= slots_per_page(state.manager.rec_size) {
        return RC::RecordNotFound;
    }
    let rec_size = state.manager.rec_size;
    let Some(mut buffer_pool) = state.manager.buffer_pool.take() else {
        return RC::Error;
    };
    let mut page_handle = state.manager.page_handler.take().unwrap_or(BM_PageHandle { page_num: record.id.page, data: String::new() });
    let rc = buffer_mgr::pin_page(&mut buffer_pool, &mut page_handle, record.id.page);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    let mut page = bytes_from_string(&page_handle.data);
    let record_offset = page_header_size() + record.id.slot as usize * (rec_size as usize + 2);
    if page.get(record_offset).copied() != Some(b'Y') {
        let _ = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return RC::RecordNotFound;
    }
    let record_bytes = record_data_bytes(record, rec_size as usize);
    page[record_offset + 1..record_offset + 1 + rec_size as usize].copy_from_slice(&record_bytes);
    page_handle.data = string_from_bytes(page);
    let rc = buffer_mgr::mark_dirty(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    let rc = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    state.manager.buffer_pool = Some(buffer_pool);
    state.manager.page_handler = Some(page_handle);
    RC::Ok
}
pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    let Some(state) = get_table_state(rel) else {
        return RC::Error;
    };
    let mut state = state.borrow_mut();
    if id.slot >= slots_per_page(state.manager.rec_size) {
        return RC::RecordNotFound;
    }
    let rec_size = state.manager.rec_size;
    let Some(mut buffer_pool) = state.manager.buffer_pool.take() else {
        return RC::Error;
    };
    let mut page_handle = state.manager.page_handler.take().unwrap_or(BM_PageHandle { page_num: id.page, data: String::new() });
    let rc = buffer_mgr::pin_page(&mut buffer_pool, &mut page_handle, id.page);
    if rc != RC::Ok {
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return rc;
    }
    let page = bytes_from_string(&page_handle.data);
    let record_offset = page_header_size() + id.slot as usize * (rec_size as usize + 2);
    if page.get(record_offset).copied() != Some(b'Y') {
        let _ = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
        state.manager.buffer_pool = Some(buffer_pool);
        state.manager.page_handler = Some(page_handle);
        return RC::RecordNotFound;
    }
    record.id = id.clone();
    record.data = string_from_bytes(page[record_offset + 1..record_offset + 1 + rec_size as usize].to_vec());
    let rc = buffer_mgr::unpin_page(&mut buffer_pool, &mut page_handle);
    state.manager.buffer_pool = Some(buffer_pool);
    state.manager.page_handler = Some(page_handle);
    rc
}
pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let total_entries = get_num_tuples(rel);
    scan.rel = clone_table_handle(rel);
    scan.mgmt_data = Some(Box::new(ScanState {
        total_entries,
        scan_index: 0,
        current_page_num: get_table_state(rel).map(|state| state.borrow().manager.first_data_page_num).unwrap_or(-1),
        current_slot_num: -1,
        condition_expression: cond.clone(),
    }));
    RC::Ok
}
pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    let Some(scan_state) = scan.mgmt_data.as_mut().and_then(|state| state.downcast_mut::<ScanState>()) else {
        return RC::RecordNotFound;
    };
    let Some(table_state) = get_table_state(&scan.rel) else {
        return RC::Error;
    };
    let rec_size = table_state.borrow().manager.rec_size;
    let slots = slots_per_page(rec_size);
    if scan_state.scan_index >= scan_state.total_entries {
        return RC::RmNoMoreTuples;
    }

    loop {
        scan_state.current_slot_num += 1;
        if scan_state.current_slot_num >= slots {
            scan_state.current_page_num += 1;
            scan_state.current_slot_num = 0;
        }
        let rid = RID { page: scan_state.current_page_num, slot: scan_state.current_slot_num };
        let rc = get_record(&scan.rel, &rid, record);
        if rc == RC::Ok {
            scan_state.scan_index += 1;
            if matches!(scan_state.condition_expression.expr_type, expr::ExprType::ExprConst)
                && !matches!(scan_state.condition_expression.expr, expr::ExprUnion::Op(_))
            {
                return RC::Ok;
            }
            let mut value = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
            let eval_rc = expr::eval_expr(record, &scan.rel.schema, &scan_state.condition_expression, &mut value);
            if eval_rc != RC::Ok {
                return eval_rc;
            }
            if let ValueUnion::BoolV(true) = value.v {
                return RC::Ok;
            }
        }
        if scan_state.scan_index >= scan_state.total_entries {
            return RC::RmNoMoreTuples;
        }
    }
}
pub fn close_scan(scan: &mut RM_ScanHandle) -> RC {
    scan.mgmt_data = None;
    RC::Ok
}
pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total = 0;
    for index in 0..schema.num_attr as usize {
        total += match schema.data_types[index] {
            DataType::DtString => schema.type_length[index],
            DataType::DtInt => std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => std::mem::size_of::<f32>() as i32,
            DataType::DtBool => std::mem::size_of::<bool>() as i32,
        };
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
pub fn free_schema(schema: &mut Schema) -> RC {
    schema.attr_names.clear();
    schema.data_types.clear();
    schema.type_length.clear();
    schema.key_attrs.clear();
    RC::Ok
}
pub fn create_record(record: &mut Option<Record>, schema: &Schema) -> RC {
    *record = Some(Record {
        id: RID { page: -1, slot: -1 },
        data: string_from_bytes(vec![0_u8; get_record_size(schema) as usize + 1]),
    });
    RC::Ok
}
pub fn free_record(record: &mut Record) -> RC {
    record.data.clear();
    RC::Ok
}
pub fn get_attr(record: &Record, schema: &Schema, attr_num: i32, value: &mut Value) -> RC {
    let position = get_attr_pos(schema, attr_num) as usize;
    let bytes = bytes_from_string(&record.data);
    value.dt = schema.data_types[attr_num as usize].clone();
    value.v = match schema.data_types[attr_num as usize] {
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let slice = &bytes[position..position + len];
            let end = slice.iter().position(|byte| *byte == 0).unwrap_or(slice.len());
            ValueUnion::StringV(string_from_bytes(slice[..end].to_vec()))
        }
        DataType::DtInt => {
            let mut raw = [0_u8; 4];
            raw.copy_from_slice(&bytes[position..position + 4]);
            ValueUnion::IntV(i32::from_le_bytes(raw))
        }
        DataType::DtFloat => {
            let mut raw = [0_u8; 4];
            raw.copy_from_slice(&bytes[position..position + 4]);
            ValueUnion::FloatV(f32::from_le_bytes(raw))
        }
        DataType::DtBool => ValueUnion::BoolV(bytes.get(position).copied().unwrap_or(0) != 0),
    };
    RC::Ok
}
pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let position = get_attr_pos(schema, attr_num) as usize;
    let rec_size = get_record_size(schema) as usize;
    let mut bytes = bytes_from_string(&record.data);
    bytes.resize(rec_size + 1, 0);
    match (&schema.data_types[attr_num as usize], &value.v) {
        (DataType::DtInt, ValueUnion::IntV(val)) => {
            bytes[position..position + 4].copy_from_slice(&val.to_le_bytes());
        }
        (DataType::DtFloat, ValueUnion::FloatV(val)) => {
            bytes[position..position + 4].copy_from_slice(&val.to_le_bytes());
        }
        (DataType::DtString, ValueUnion::StringV(val)) => {
            let len = schema.type_length[attr_num as usize] as usize;
            bytes[position..position + len].copy_from_slice(&fixed_len_bytes(val, len));
        }
        (DataType::DtBool, ValueUnion::BoolV(val)) => {
            bytes[position] = u8::from(*val);
        }
        _ => return RC::RmCompareValueOfDifferentDatatype,
    }
    record.data = string_from_bytes(bytes);
    RC::Ok
}
pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut pos = 0;
    for index in 0..attr_num as usize {
        pos += match schema.data_types[index] {
            DataType::DtString => schema.type_length[index],
            DataType::DtInt => std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => std::mem::size_of::<f32>() as i32,
            DataType::DtBool => std::mem::size_of::<bool>() as i32,
        };
    }
    pos
}

fn get_table_state(rel: &RM_TableData) -> Option<&Rc<RefCell<TableState>>> {
    rel.mgmt_data.as_ref()?.downcast_ref::<Rc<RefCell<TableState>>>()
}

fn clone_table_handle(rel: &RM_TableData) -> RM_TableData {
    RM_TableData {
        name: rel.name.clone(),
        schema: rel.schema.clone(),
        mgmt_data: get_table_state(rel)
            .map(|state| Box::new(state.clone()) as Box<dyn std::any::Any>),
    }
}

fn write_i32(page: &mut [u8], offset: usize, value: i32) {
    page[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn read_i32(page: &[u8], offset: usize) -> i32 {
    let mut raw = [0_u8; 4];
    raw.copy_from_slice(&page[offset..offset + 4]);
    i32::from_le_bytes(raw)
}

fn page_header_size() -> usize {
    32
}

fn slots_per_page(record_size: i32) -> i32 {
    ((crate::dberror::PAGE_SIZE as usize - page_header_size()) / (record_size as usize + 2)) as i32
}

fn decode_page_header(page: &[u8]) -> PageHeader {
    PageHeader {
        page_identifier: page.first().copied().unwrap_or(0) as char,
        total_tuples: read_i32(page, 4),
        free_slot_cnt: read_i32(page, 8),
        next_free_slot_ind: read_i32(page, 12),
        prev_free_page_index: read_i32(page, 16),
        next_free_page_index: read_i32(page, 20),
        prev_data_page_index: read_i32(page, 24),
        next_data_page_index: read_i32(page, 28),
    }
}

fn encode_page_header(page: &mut [u8], header: &PageHeader) {
    page[0] = header.page_identifier as u8;
    page[1..4].fill(0);
    write_i32(page, 4, header.total_tuples);
    write_i32(page, 8, header.free_slot_cnt);
    write_i32(page, 12, header.next_free_slot_ind);
    write_i32(page, 16, header.prev_free_page_index);
    write_i32(page, 20, header.next_free_page_index);
    write_i32(page, 24, header.prev_data_page_index);
    write_i32(page, 28, header.next_data_page_index);
}

fn record_data_bytes(record: &Record, rec_size: usize) -> Vec<u8> {
    let mut bytes = bytes_from_string(&record.data);
    bytes.resize(rec_size + 1, 0);
    bytes[..rec_size].to_vec()
}

fn data_type_code(data_type: &DataType) -> i32 {
    match data_type {
        DataType::DtInt => 0,
        DataType::DtString => 1,
        DataType::DtFloat => 2,
        DataType::DtBool => 3,
    }
}
