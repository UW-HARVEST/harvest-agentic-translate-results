use crate::{dberror::{RC, PAGE_SIZE}, expr::Expr, tables::{Record, Schema, RM_TableData, RID}};
use crate::buffer_mgr::{
    BM_BufferPool, BM_PageHandle, ReplacementStrategy,
    init_buffer_pool, shutdown_buffer_pool, pin_page, unpin_page, mark_dirty,
};
use crate::storage_mgr::{create_page_file, destroy_page_file};
use crate::tables::{DataType, Value, ValueUnion};
use std::cell::RefCell;
use std::rc::Rc;

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
    pub table_ref: Option<Rc<RefCell<TableManager>>>,
    pub schema: Option<Schema>,
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
const PAGE_HEADER_SIZE: usize = 1 + 7 * 4; // 1 char + 7 ints
// Round up to multiple matching C struct alignment. In C with default packing, PageHeader
// would be padded to 32 bytes (1 char + 3 padding + 7 ints = 32). Use 32 to match.
const PAGE_HEADER_SIZE_PADDED: usize = 32;

pub fn init_record_manager(_mgmt_data: Option<Box<dyn std::any::Any>>) -> RC {
    RC::Ok
}

pub fn shutdown_record_manager() -> RC {
    RC::Ok
}

fn data_type_disc(dt: &DataType) -> i32 {
    match dt {
        DataType::DtInt => 0,
        DataType::DtString => 1,
        DataType::DtFloat => 2,
        DataType::DtBool => 3,
    }
}

fn data_type_from_int(v: i32) -> DataType {
    match v {
        0 => DataType::DtInt,
        1 => DataType::DtString,
        2 => DataType::DtFloat,
        _ => DataType::DtBool,
    }
}

fn write_i32(buf: &mut Vec<u8>, off: usize, val: i32) {
    let bytes = val.to_ne_bytes();
    for i in 0..4 {
        buf[off + i] = bytes[i];
    }
}

fn read_i32(buf: &[u8], off: usize) -> i32 {
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&buf[off..off + 4]);
    i32::from_ne_bytes(arr)
}

fn string_to_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| c as u8).collect()
}

fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn prepare_table_header(buf: &mut Vec<u8>, tm: &mut TableManager, schema: &Schema) {
    tm.total_tuples = 0;
    tm.rec_size = get_record_size(schema);
    tm.first_free_page_num = 1;
    tm.first_free_slot_num = 0;
    tm.first_data_page_num = -1;

    let mut off = 0usize;
    write_i32(buf, off, tm.total_tuples); off += 4;
    write_i32(buf, off, tm.rec_size); off += 4;
    write_i32(buf, off, tm.first_free_page_num); off += 4;
    write_i32(buf, off, tm.first_free_slot_num); off += 4;
    write_i32(buf, off, tm.first_data_page_num); off += 4;
    write_i32(buf, off, schema.num_attr); off += 4;
    write_i32(buf, off, schema.key_size); off += 4;

    for i in 0..schema.num_attr as usize {
        let name = string_to_bytes(&schema.attr_names[i]);
        for k in 0..MAX_ATTR_NAME_LEN {
            buf[off + k] = if k < name.len() { name[k] } else { 0 };
        }
        off += MAX_ATTR_NAME_LEN;
        write_i32(buf, off, data_type_disc(&schema.data_types[i])); off += 4;
        write_i32(buf, off, schema.type_length[i]); off += 4;
    }
    for i in 0..schema.key_size as usize {
        write_i32(buf, off, schema.key_attrs[i]); off += 4;
    }
}

pub fn create_table(name: &str, schema: &Schema) -> RC {
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
        page_num: -1,
        data: String::new(),
    };
    let rc = pin_page(&mut bm, &mut page, 0);
    if rc != RC::Ok {
        return rc;
    }
    let mut buf = string_to_bytes(&page.data);
    if buf.len() < PAGE_SIZE as usize {
        buf.resize(PAGE_SIZE as usize, 0);
    }
    let mut tm = TableManager {
        total_tuples: 0,
        rec_size: 0,
        first_free_page_num: 0,
        first_free_slot_num: 0,
        first_data_page_num: 0,
        buffer_pool: None,
        page_handler: None,
    };
    prepare_table_header(&mut buf, &mut tm, schema);
    page.data = bytes_to_string(&buf);
    let rc = mark_dirty(&mut bm, &mut page);
    if rc != RC::Ok {
        return rc;
    }
    let rc = unpin_page(&mut bm, &mut page);
    if rc != RC::Ok {
        return rc;
    }
    shutdown_buffer_pool(&mut bm)
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
        page_num: -1,
        data: String::new(),
    };
    let rc = pin_page(&mut bm, &mut page, 0);
    if rc != RC::Ok {
        return rc;
    }
    let buf = string_to_bytes(&page.data);
    let mut off = 0;
    let total_tuples = read_i32(&buf, off); off += 4;
    let rec_size = read_i32(&buf, off); off += 4;
    let first_free_page_num = read_i32(&buf, off); off += 4;
    let first_free_slot_num = read_i32(&buf, off); off += 4;
    let first_data_page_num = read_i32(&buf, off); off += 4;
    let num_attr = read_i32(&buf, off); off += 4;
    let key_size = read_i32(&buf, off); off += 4;

    let mut attr_names: Vec<String> = Vec::with_capacity(num_attr as usize);
    let mut data_types: Vec<DataType> = Vec::with_capacity(num_attr as usize);
    let mut type_length: Vec<i32> = Vec::with_capacity(num_attr as usize);
    for _ in 0..num_attr as usize {
        let name_bytes = &buf[off..off + MAX_ATTR_NAME_LEN];
        let end = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_bytes.len());
        let name = String::from_utf8_lossy(&name_bytes[..end]).into_owned();
        attr_names.push(name);
        off += MAX_ATTR_NAME_LEN;
        let dt_int = read_i32(&buf, off); off += 4;
        data_types.push(data_type_from_int(dt_int));
        type_length.push(read_i32(&buf, off)); off += 4;
    }
    let mut key_attrs: Vec<i32> = Vec::with_capacity(key_size as usize);
    for _ in 0..key_size as usize {
        key_attrs.push(read_i32(&buf, off)); off += 4;
    }

    let _ = unpin_page(&mut bm, &mut page);

    let schema = Schema {
        num_attr,
        attr_names,
        data_types,
        type_length,
        key_attrs,
        key_size,
    };
    let tm = TableManager {
        total_tuples,
        rec_size,
        first_free_page_num,
        first_free_slot_num,
        first_data_page_num,
        buffer_pool: Some(bm),
        page_handler: Some(page),
    };
    rel.name = name.to_string();
    rel.schema = schema;
    rel.mgmt_data = Some(Box::new(Rc::new(RefCell::new(tm))));
    RC::Ok
}

pub fn close_table(rel: &mut RM_TableData) -> RC {
    let tm_box = match rel.mgmt_data.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let cell_box: Box<Rc<RefCell<TableManager>>> = match tm_box.downcast::<Rc<RefCell<TableManager>>>() {
        Ok(b) => b,
        Err(_) => return RC::Error,
    };
    let cell = *cell_box;
    let cell = match Rc::try_unwrap(cell) {
        Ok(c) => c,
        Err(_) => return RC::Error,
    };
    let mut tm = cell.into_inner();
    let mut bm = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = tm.page_handler.take().unwrap_or(BM_PageHandle { page_num: -1, data: String::new() });
    let rc = pin_page(&mut bm, &mut page, 0);
    if rc == RC::Ok {
        let mut buf = string_to_bytes(&page.data);
        if buf.len() < PAGE_SIZE as usize {
            buf.resize(PAGE_SIZE as usize, 0);
        }
        write_i32(&mut buf, 0, tm.total_tuples);
        write_i32(&mut buf, 4, tm.rec_size);
        write_i32(&mut buf, 8, tm.first_free_page_num);
        write_i32(&mut buf, 12, tm.first_free_slot_num);
        write_i32(&mut buf, 16, tm.first_data_page_num);
        page.data = bytes_to_string(&buf);
        let _ = mark_dirty(&mut bm, &mut page);
        let _ = unpin_page(&mut bm, &mut page);
    }
    shutdown_buffer_pool(&mut bm)
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() {
        return RC::InvalidHeader;
    }
    destroy_page_file(name)
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    rel.mgmt_data
        .as_ref()
        .and_then(|m| m.downcast_ref::<Rc<RefCell<TableManager>>>())
        .map(|c| c.borrow().total_tuples)
        .unwrap_or(-1)
}

fn slots_per_page(rec_size: i32) -> i32 {
    (PAGE_SIZE - PAGE_HEADER_SIZE_PADDED as i32) / (rec_size + 2)
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let tm_box = match rel.mgmt_data.as_ref() {
        Some(b) => b,
        None => return RC::Error,
    };
    let cell = match tm_box.downcast_ref::<Rc<RefCell<TableManager>>>() {
        Some(c) => c.clone(),
        None => return RC::Error,
    };
    let mut tm = cell.borrow_mut();
    let tm = &mut *tm;
    let bm = match tm.buffer_pool.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = tm.page_handler.take().unwrap_or(BM_PageHandle { page_num: -1, data: String::new() });

    let slots = slots_per_page(tm.rec_size);
    let target_page = tm.first_free_page_num;
    let rc = pin_page(bm, &mut page, target_page);
    if rc != RC::Ok {
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let mut buf = string_to_bytes(&page.data);
    if buf.len() < PAGE_SIZE as usize {
        buf.resize(PAGE_SIZE as usize, 0);
    }

    // PageHeader layout:
    // byte 0: pageIdentifier (char)
    // bytes 4..8: total_tuples
    // bytes 8..12: free_slot_cnt
    // bytes 12..16: next_free_slot_ind
    // bytes 16..20: prev_free_page_index
    // bytes 20..24: next_free_page_index
    // bytes 24..28: prev_data_page_index
    // bytes 28..32: next_data_page_index

    if buf[0] != b'Y' {
        buf[0] = b'Y';
        write_i32(&mut buf, 4, 0); // total_tuples
        write_i32(&mut buf, 8, slots - 1); // free_slot_cnt
        write_i32(&mut buf, 12, 1); // next_free_slot_ind
        write_i32(&mut buf, 16, -1); // prev_free_page_index
        write_i32(&mut buf, 20, page.page_num + 1); // next_free_page_index
        write_i32(&mut buf, 24, -1); // prev_data_page_index
        write_i32(&mut buf, 28, 1); // next_data_page_index
    } else {
        let total = read_i32(&buf, 4) + 1;
        let free = read_i32(&buf, 8) - 1;
        write_i32(&mut buf, 4, total);
        write_i32(&mut buf, 8, free);
        if free > 0 {
            let nfs = read_i32(&buf, 12) + 1;
            write_i32(&mut buf, 12, nfs);
        } else {
            let nfs = read_i32(&buf, 12);
            write_i32(&mut buf, 12, -nfs);
        }
    }

    let position = PAGE_HEADER_SIZE_PADDED + (tm.first_free_slot_num as usize) * (tm.rec_size as usize + 2);
    buf[position] = b'Y';
    let rec_bytes = string_to_bytes(&record.data);
    for k in 0..tm.rec_size as usize {
        buf[position + 1 + k] = rec_bytes.get(k).copied().unwrap_or(0);
    }
    buf[position + tm.rec_size as usize + 1] = b'|';

    let new_id_page = page.page_num;
    let new_id_slot = tm.first_free_slot_num;

    let free_after = read_i32(&buf, 8);
    if free_after == 0 {
        tm.first_free_page_num += 1;
        tm.first_free_slot_num = 0;
    } else {
        tm.first_free_slot_num += 1;
    }
    tm.total_tuples += 1;

    page.data = bytes_to_string(&buf);
    let _ = mark_dirty(bm, &mut page);
    let _ = unpin_page(bm, &mut page);
    tm.page_handler = Some(page);

    // Update record id by mutating: but the API takes &Record. The C code mutates it. We can't.
    // The id is set on the input record in C. We can't modify here in safe Rust signature.
    // Workaround: caller can pass &mut Record via separate API, but we keep signature. Skip.
    let _ = (new_id_page, new_id_slot);
    RC::Ok
}

/// Mutating variant: also updates record.id.page and record.id.slot.
pub fn insert_record_mut(rel: &mut RM_TableData, record: &mut Record) -> RC {
    // Re-implement so we can mutate record.id
    let tm_box = match rel.mgmt_data.as_ref() {
        Some(b) => b,
        None => return RC::Error,
    };
    let cell = match tm_box.downcast_ref::<Rc<RefCell<TableManager>>>() {
        Some(c) => c.clone(),
        None => return RC::Error,
    };
    let mut tm = cell.borrow_mut();
    let tm = &mut *tm;
    let bm = match tm.buffer_pool.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = tm.page_handler.take().unwrap_or(BM_PageHandle { page_num: -1, data: String::new() });

    let slots = slots_per_page(tm.rec_size);
    let target_page = tm.first_free_page_num;
    let rc = pin_page(bm, &mut page, target_page);
    if rc != RC::Ok {
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let mut buf = string_to_bytes(&page.data);
    if buf.len() < PAGE_SIZE as usize {
        buf.resize(PAGE_SIZE as usize, 0);
    }

    if buf[0] != b'Y' {
        buf[0] = b'Y';
        write_i32(&mut buf, 4, 0);
        write_i32(&mut buf, 8, slots - 1);
        write_i32(&mut buf, 12, 1);
        write_i32(&mut buf, 16, -1);
        write_i32(&mut buf, 20, page.page_num + 1);
        write_i32(&mut buf, 24, -1);
        write_i32(&mut buf, 28, 1);
    } else {
        let total = read_i32(&buf, 4) + 1;
        let free = read_i32(&buf, 8) - 1;
        write_i32(&mut buf, 4, total);
        write_i32(&mut buf, 8, free);
        if free > 0 {
            let nfs = read_i32(&buf, 12) + 1;
            write_i32(&mut buf, 12, nfs);
        } else {
            let nfs = read_i32(&buf, 12);
            write_i32(&mut buf, 12, -nfs);
        }
    }

    let position = PAGE_HEADER_SIZE_PADDED + (tm.first_free_slot_num as usize) * (tm.rec_size as usize + 2);
    buf[position] = b'Y';
    let rec_bytes = string_to_bytes(&record.data);
    for k in 0..tm.rec_size as usize {
        buf[position + 1 + k] = rec_bytes.get(k).copied().unwrap_or(0);
    }
    buf[position + tm.rec_size as usize + 1] = b'|';

    record.id.page = page.page_num;
    record.id.slot = tm.first_free_slot_num;

    let free_after = read_i32(&buf, 8);
    if free_after == 0 {
        tm.first_free_page_num += 1;
        tm.first_free_slot_num = 0;
    } else {
        tm.first_free_slot_num += 1;
    }
    tm.total_tuples += 1;

    page.data = bytes_to_string(&buf);
    let _ = mark_dirty(bm, &mut page);
    let _ = unpin_page(bm, &mut page);
    tm.page_handler = Some(page);
    RC::Ok
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let cell = match rel.mgmt_data.as_ref().and_then(|m| m.downcast_ref::<Rc<RefCell<TableManager>>>()) {
        Some(c) => c.clone(),
        None => return RC::Error,
    };
    let mut tm = cell.borrow_mut();
    let tm = &mut *tm;
    let slots = slots_per_page(tm.rec_size);
    if id.slot >= slots {
        return RC::RecordNotFound;
    }
    let bm = tm.buffer_pool.as_mut().unwrap();
    let mut page = tm.page_handler.take().unwrap_or(BM_PageHandle { page_num: -1, data: String::new() });
    let rc = pin_page(bm, &mut page, id.page);
    if rc != RC::Ok {
        tm.page_handler = Some(page);
        return rc;
    }
    let mut buf = string_to_bytes(&page.data);
    if buf.len() < PAGE_SIZE as usize {
        buf.resize(PAGE_SIZE as usize, 0);
    }
    let position = PAGE_HEADER_SIZE_PADDED + (id.slot as usize) * (tm.rec_size as usize + 2);
    if buf[position] != b'Y' {
        let _ = unpin_page(bm, &mut page);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    buf[position] = b'N';
    let total = read_i32(&buf, 4);
    let free = read_i32(&buf, 8);
    write_i32(&mut buf, 4, if total > 0 { total - 1 } else { 0 });
    write_i32(&mut buf, 8, free + 1);

    if tm.total_tuples > 0 {
        tm.total_tuples -= 1;
    }
    page.data = bytes_to_string(&buf);
    if mark_dirty(bm, &mut page) != RC::Ok {
        let _ = unpin_page(bm, &mut page);
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let rc = unpin_page(bm, &mut page);
    tm.page_handler = Some(page);
    rc
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let cell = match rel.mgmt_data.as_ref().and_then(|m| m.downcast_ref::<Rc<RefCell<TableManager>>>()) {
        Some(c) => c.clone(),
        None => return RC::Error,
    };
    let mut tm = cell.borrow_mut();
    let tm = &mut *tm;
    let slots = slots_per_page(tm.rec_size);
    if record.id.slot >= slots {
        return RC::RecordNotFound;
    }
    let bm = tm.buffer_pool.as_mut().unwrap();
    let mut page = tm.page_handler.take().unwrap_or(BM_PageHandle { page_num: -1, data: String::new() });
    let rc = pin_page(bm, &mut page, record.id.page);
    if rc != RC::Ok {
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let mut buf = string_to_bytes(&page.data);
    if buf.len() < PAGE_SIZE as usize {
        buf.resize(PAGE_SIZE as usize, 0);
    }
    let position = PAGE_HEADER_SIZE_PADDED + (record.id.slot as usize) * (tm.rec_size as usize + 2);
    if buf[position] != b'Y' {
        let _ = unpin_page(bm, &mut page);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    let rec_bytes = string_to_bytes(&record.data);
    for k in 0..tm.rec_size as usize {
        buf[position + 1 + k] = rec_bytes.get(k).copied().unwrap_or(0);
    }
    page.data = bytes_to_string(&buf);
    if mark_dirty(bm, &mut page) != RC::Ok {
        let _ = unpin_page(bm, &mut page);
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let rc = unpin_page(bm, &mut page);
    tm.page_handler = Some(page);
    if rc != RC::Ok {
        return RC::Error;
    }
    RC::Ok
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    let cell = match rel.mgmt_data.as_ref().and_then(|m| m.downcast_ref::<Rc<RefCell<TableManager>>>()) {
        Some(c) => c.clone(),
        None => return RC::Error,
    };
    let mut tm = cell.borrow_mut();
    let tm = &mut *tm;
    let slots = slots_per_page(tm.rec_size);
    if id.slot >= slots {
        return RC::RecordNotFound;
    }
    let bm = tm.buffer_pool.as_mut().unwrap();
    let mut page = tm.page_handler.take().unwrap_or(BM_PageHandle { page_num: -1, data: String::new() });
    let rc = pin_page(bm, &mut page, id.page);
    if rc != RC::Ok {
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let buf = string_to_bytes(&page.data);
    let position = PAGE_HEADER_SIZE_PADDED + (id.slot as usize) * (tm.rec_size as usize + 2);
    if buf.get(position).copied() != Some(b'Y') {
        let _ = unpin_page(bm, &mut page);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    let rec_bytes = &buf[position + 1..position + 1 + tm.rec_size as usize];
    record.data = bytes_to_string(rec_bytes);
    record.id = id.clone();
    let rc = unpin_page(bm, &mut page);
    tm.page_handler = Some(page);
    rc
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let cell = match rel.mgmt_data.as_ref().and_then(|m| m.downcast_ref::<Rc<RefCell<TableManager>>>()) {
        Some(c) => c.clone(),
        None => return RC::Error,
    };
    let tm = cell.borrow();
    let sm = ScanManager {
        total_entries: tm.total_tuples,
        current_page_num: tm.first_data_page_num,
        current_slot_num: -1,
        scan_index: 0,
        condition_expression: Some(cond.clone()),
        scan_page_handle_ptr: None,
        table_ref: Some(cell.clone()),
        schema: Some(rel.schema.clone()),
    };
    drop(tm);
    scan.mgmt_data = Some(Box::new(sm));
    scan.rel = clone_table_data(rel);
    RC::Ok
}

/// Like start_scan but with no condition.
pub fn start_scan_nocond(rel: &RM_TableData, scan: &mut RM_ScanHandle) -> RC {
    let cell = match rel.mgmt_data.as_ref().and_then(|m| m.downcast_ref::<Rc<RefCell<TableManager>>>()) {
        Some(c) => c.clone(),
        None => return RC::Error,
    };
    let tm = cell.borrow();
    let sm = ScanManager {
        total_entries: tm.total_tuples,
        current_page_num: tm.first_data_page_num,
        current_slot_num: -1,
        scan_index: 0,
        condition_expression: None,
        scan_page_handle_ptr: None,
        table_ref: Some(cell.clone()),
        schema: Some(rel.schema.clone()),
    };
    drop(tm);
    scan.mgmt_data = Some(Box::new(sm));
    scan.rel = clone_table_data(rel);
    RC::Ok
}

fn clone_table_data(rel: &RM_TableData) -> RM_TableData {
    RM_TableData {
        name: rel.name.clone(),
        schema: rel.schema.clone(),
        mgmt_data: None,
    }
}

pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    let sm_box = match scan.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let sm = match sm_box.downcast_mut::<ScanManager>() {
        Some(s) => s,
        None => return RC::Error,
    };
    let cell = match &sm.table_ref {
        Some(c) => c.clone(),
        None => return RC::Error,
    };
    let schema = match &sm.schema {
        Some(s) => s.clone(),
        None => return RC::Error,
    };
    let rec_size = cell.borrow().rec_size;
    let slots = slots_per_page(rec_size);
    if sm.scan_index >= sm.total_entries {
        return RC::RmNoMoreTuples;
    }
    loop {
        sm.current_slot_num += 1;
        if sm.current_slot_num >= slots {
            sm.current_page_num += 1;
            sm.current_slot_num = 0;
        }
        let rid = RID {
            page: sm.current_page_num,
            slot: sm.current_slot_num,
        };
        let rc = get_record_internal(&cell, &rid, record);
        if rc == RC::Ok {
            sm.scan_index += 1;
            let pass = if let Some(cond) = &sm.condition_expression {
                let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
                let _ = crate::expr::eval_expr(record, &schema, cond, &mut result);
                matches!(result.v, ValueUnion::BoolV(true))
            } else {
                true
            };
            if pass {
                return RC::Ok;
            }
        }
        if sm.scan_index >= sm.total_entries {
            return RC::RmNoMoreTuples;
        }
    }
}

fn get_record_internal(cell: &Rc<RefCell<TableManager>>, id: &RID, record: &mut Record) -> RC {
    let mut tm = cell.borrow_mut();
    let tm = &mut *tm;
    let slots = slots_per_page(tm.rec_size);
    if id.slot >= slots {
        return RC::RecordNotFound;
    }
    let bm = tm.buffer_pool.as_mut().unwrap();
    let mut page = tm.page_handler.take().unwrap_or(BM_PageHandle { page_num: -1, data: String::new() });
    let rc = pin_page(bm, &mut page, id.page);
    if rc != RC::Ok {
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let buf = string_to_bytes(&page.data);
    let position = PAGE_HEADER_SIZE_PADDED + (id.slot as usize) * (tm.rec_size as usize + 2);
    if buf.get(position).copied() != Some(b'Y') {
        let _ = unpin_page(bm, &mut page);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    let rec_bytes = &buf[position + 1..position + 1 + tm.rec_size as usize];
    record.data = bytes_to_string(rec_bytes);
    record.id = id.clone();
    let rc = unpin_page(bm, &mut page);
    tm.page_handler = Some(page);
    rc
}


pub fn close_scan(scan: &mut RM_ScanHandle) -> RC {
    if scan.mgmt_data.is_none() {
        return RC::RecordNotFound;
    }
    scan.mgmt_data = None;
    RC::Ok
}

pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total = 0i32;
    for i in 0..schema.num_attr as usize {
        match schema.data_types[i] {
            DataType::DtString => total += schema.type_length[i],
            DataType::DtInt => total += 4,
            DataType::DtFloat => total += 4,
            DataType::DtBool => total += 1,
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
    let rec_size = get_record_size(schema);
    let data: String = (0..rec_size).map(|_| '\0').collect();
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
    let attr_idx = attr_num as usize;
    let pos = get_attr_pos(schema, attr_num) as usize;
    let bytes = string_to_bytes(&record.data);
    let dt = schema.data_types[attr_idx].clone();
    match dt {
        DataType::DtString => {
            let len = schema.type_length[attr_idx] as usize;
            // C uses strncpy which doesn't guarantee null termination.
            // The C version allocates len+1 zeroed and strncpys len bytes; the resulting
            // string may have garbage if data has no null. Match behavior: include up to len.
            let end_idx = (pos + len).min(bytes.len());
            let raw = &bytes[pos..end_idx];
            // Find null terminator within raw if any (matches strncpy behavior into calloc'd buffer).
            let stop = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            let s = bytes_to_string(&raw[..stop]);
            value.dt = DataType::DtString;
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtInt => {
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&bytes[pos..pos + 4]);
            value.dt = DataType::DtInt;
            value.v = ValueUnion::IntV(i32::from_ne_bytes(arr));
        }
        DataType::DtFloat => {
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&bytes[pos..pos + 4]);
            value.dt = DataType::DtFloat;
            value.v = ValueUnion::FloatV(f32::from_ne_bytes(arr));
        }
        DataType::DtBool => {
            value.dt = DataType::DtBool;
            value.v = ValueUnion::BoolV(bytes[pos] != 0);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let attr_idx = attr_num as usize;
    let pos = get_attr_pos(schema, attr_num) as usize;
    let mut bytes = string_to_bytes(&record.data);
    let need_size = pos + match schema.data_types[attr_idx] {
        DataType::DtInt => 4,
        DataType::DtFloat => 4,
        DataType::DtString => schema.type_length[attr_idx] as usize,
        DataType::DtBool => 1,
    };
    if bytes.len() < need_size {
        bytes.resize(need_size, 0);
    }
    match (&schema.data_types[attr_idx], &value.v) {
        (DataType::DtInt, ValueUnion::IntV(v)) => {
            let arr = v.to_ne_bytes();
            for k in 0..4 {
                bytes[pos + k] = arr[k];
            }
        }
        (DataType::DtFloat, ValueUnion::FloatV(v)) => {
            let arr = v.to_ne_bytes();
            for k in 0..4 {
                bytes[pos + k] = arr[k];
            }
        }
        (DataType::DtString, ValueUnion::StringV(s)) => {
            let len = schema.type_length[attr_idx] as usize;
            let src = string_to_bytes(s);
            for k in 0..len {
                bytes[pos + k] = src.get(k).copied().unwrap_or(0);
            }
        }
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            bytes[pos] = if *b { 1 } else { 0 };
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
            DataType::DtInt => attr_pos += 4,
            DataType::DtFloat => attr_pos += 4,
            DataType::DtBool => attr_pos += 1,
        }
    }
    attr_pos
}
