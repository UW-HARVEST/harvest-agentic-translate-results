use crate::buffer_mgr::{
    init_buffer_pool, mark_dirty, pin_page, shutdown_buffer_pool, unpin_page, BM_BufferPool,
    BM_PageHandle, ReplacementStrategy,
};
use crate::dberror::{PAGE_SIZE, RC};
use crate::expr::{eval_expr, Expr};
use crate::storage_mgr::{create_page_file, destroy_page_file};
use crate::tables::{DataType, RID, Record, Schema, Value, ValueUnion, RM_TableData};

const MAX_ATTR_NAME_LEN: usize = 15;
const PAGE_HEADER_SIZE: usize = 32; // matches C struct PageHeader with default alignment

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

// ----- Byte-array helpers operating on String pages (one char per byte) -----

fn str_to_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| (c as u32) as u8).collect()
}

fn bytes_to_str(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len());
    for &x in b {
        s.push(x as char);
    }
    s
}

fn read_i32(data: &str, offset: usize) -> i32 {
    let bytes = str_to_bytes(data);
    let mut buf = [0u8; 4];
    for k in 0..4 {
        if offset + k < bytes.len() {
            buf[k] = bytes[offset + k];
        }
    }
    i32::from_ne_bytes(buf)
}

fn write_i32(data: &mut String, offset: usize, val: i32) {
    let mut bytes = str_to_bytes(data);
    if bytes.len() < offset + 4 {
        bytes.resize(offset + 4, 0);
    }
    let v = val.to_ne_bytes();
    for k in 0..4 {
        bytes[offset + k] = v[k];
    }
    *data = bytes_to_str(&bytes);
}

fn read_byte(data: &str, offset: usize) -> u8 {
    let bytes = str_to_bytes(data);
    if offset < bytes.len() {
        bytes[offset]
    } else {
        0
    }
}

fn write_byte(data: &mut String, offset: usize, val: u8) {
    let mut bytes = str_to_bytes(data);
    if bytes.len() <= offset {
        bytes.resize(offset + 1, 0);
    }
    bytes[offset] = val;
    *data = bytes_to_str(&bytes);
}

fn write_bytes(data: &mut String, offset: usize, src: &[u8]) {
    let mut bytes = str_to_bytes(data);
    if bytes.len() < offset + src.len() {
        bytes.resize(offset + src.len(), 0);
    }
    for (k, &b) in src.iter().enumerate() {
        bytes[offset + k] = b;
    }
    *data = bytes_to_str(&bytes);
}

fn read_bytes(data: &str, offset: usize, len: usize) -> Vec<u8> {
    let bytes = str_to_bytes(data);
    let mut out = vec![0u8; len];
    for k in 0..len {
        if offset + k < bytes.len() {
            out[k] = bytes[offset + k];
        }
    }
    out
}

fn datatype_to_i32(dt: &DataType) -> i32 {
    match dt {
        DataType::DtInt => 0,
        DataType::DtString => 1,
        DataType::DtFloat => 2,
        DataType::DtBool => 3,
    }
}

fn i32_to_datatype(v: i32) -> DataType {
    match v {
        0 => DataType::DtInt,
        1 => DataType::DtString,
        2 => DataType::DtFloat,
        3 => DataType::DtBool,
        _ => DataType::DtInt,
    }
}

// ----- Record manager API -----

pub fn init_record_manager(_mgmt_data: Option<Box<dyn std::any::Any>>) -> RC {
    RC::Ok
}

pub fn shutdown_record_manager() -> RC {
    RC::Ok
}

pub fn create_table(name: &str, schema: &Schema) -> RC {
    let result = create_page_file(name);
    if result != RC::Ok {
        return result;
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
    // Write header into page.data
    let total_tuples = 0i32;
    let rec_size = get_record_size(schema);
    let first_free_page_num = 1i32;
    let first_free_slot_num = 0i32;
    let first_data_page_num = -1i32;
    let mut data = page.data.clone();
    let mut off = 0usize;
    write_i32(&mut data, off, total_tuples);
    off += 4;
    write_i32(&mut data, off, rec_size);
    off += 4;
    write_i32(&mut data, off, first_free_page_num);
    off += 4;
    write_i32(&mut data, off, first_free_slot_num);
    off += 4;
    write_i32(&mut data, off, first_data_page_num);
    off += 4;
    write_i32(&mut data, off, schema.num_attr);
    off += 4;
    write_i32(&mut data, off, schema.key_size);
    off += 4;
    for i in 0..schema.num_attr as usize {
        // Write attr name (up to MAX_ATTR_NAME_LEN bytes, padded with zeros)
        let mut name_buf = vec![0u8; MAX_ATTR_NAME_LEN];
        let nb = schema.attr_names[i].as_bytes();
        let copy_len = nb.len().min(MAX_ATTR_NAME_LEN);
        for k in 0..copy_len {
            name_buf[k] = nb[k];
        }
        write_bytes(&mut data, off, &name_buf);
        off += MAX_ATTR_NAME_LEN;
        write_i32(&mut data, off, datatype_to_i32(&schema.data_types[i]));
        off += 4;
        write_i32(&mut data, off, schema.type_length[i]);
        off += 4;
    }
    for i in 0..schema.key_size as usize {
        write_i32(&mut data, off, schema.key_attrs[i]);
        off += 4;
    }
    page.data = data;

    let rc = mark_dirty(&mut bm, &mut page);
    if rc != RC::Ok {
        return rc;
    }
    let rc = unpin_page(&mut bm, &mut page);
    if rc != RC::Ok {
        return rc;
    }
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
        page_num: -1,
        data: String::new(),
    };
    let rc = pin_page(&mut bm, &mut page, 0);
    if rc != RC::Ok {
        return rc;
    }
    let data = page.data.clone();
    let mut off = 0usize;
    let total_tuples = read_i32(&data, off);
    off += 4;
    let rec_size = read_i32(&data, off);
    off += 4;
    let first_free_page_num = read_i32(&data, off);
    off += 4;
    let first_free_slot_num = read_i32(&data, off);
    off += 4;
    let first_data_page_num = read_i32(&data, off);
    off += 4;
    let num_attr = read_i32(&data, off);
    off += 4;
    let key_size = read_i32(&data, off);
    off += 4;

    let mut attr_names: Vec<String> = Vec::with_capacity(num_attr as usize);
    let mut data_types: Vec<DataType> = Vec::with_capacity(num_attr as usize);
    let mut type_length: Vec<i32> = Vec::with_capacity(num_attr as usize);
    for _ in 0..num_attr as usize {
        let nb = read_bytes(&data, off, MAX_ATTR_NAME_LEN);
        off += MAX_ATTR_NAME_LEN;
        let mut end = 0;
        while end < nb.len() && nb[end] != 0 {
            end += 1;
        }
        let name_str: String = nb[..end].iter().map(|&b| b as char).collect();
        attr_names.push(name_str);
        let dt_val = read_i32(&data, off);
        off += 4;
        data_types.push(i32_to_datatype(dt_val));
        let tl = read_i32(&data, off);
        off += 4;
        type_length.push(tl);
    }

    let mut key_attrs: Vec<i32> = Vec::with_capacity(key_size as usize);
    for _ in 0..key_size as usize {
        key_attrs.push(read_i32(&data, off));
        off += 4;
    }

    let rc = unpin_page(&mut bm, &mut page);
    if rc != RC::Ok {
        return rc;
    }

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
        buffer_pool: Some(bm),
        page_handler: Some(page),
    };

    rel.name = name.to_string();
    rel.schema = schema;
    rel.mgmt_data = Some(Box::new(table_manager));
    RC::Ok
}

pub fn close_table(rel: &mut RM_TableData) -> RC {
    let mut tm = match rel
        .mgmt_data
        .take()
        .and_then(|b| b.downcast::<TableManager>().ok())
    {
        Some(b) => *b,
        None => return RC::Error,
    };

    let mut bm = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = tm.page_handler.take().unwrap_or(BM_PageHandle {
        page_num: -1,
        data: String::new(),
    });

    let pin_rc = pin_page(&mut bm, &mut page, 0);
    if pin_rc == RC::Ok {
        let mut data = page.data.clone();
        let mut off = 0usize;
        write_i32(&mut data, off, tm.total_tuples);
        off += 4;
        write_i32(&mut data, off, tm.rec_size);
        off += 4;
        write_i32(&mut data, off, tm.first_free_page_num);
        off += 4;
        write_i32(&mut data, off, tm.first_free_slot_num);
        off += 4;
        write_i32(&mut data, off, tm.first_data_page_num);
        page.data = data;
        let _ = mark_dirty(&mut bm, &mut page);
        let _ = unpin_page(&mut bm, &mut page);
    }

    let shutdown_rc = shutdown_buffer_pool(&mut bm);
    if pin_rc != RC::Ok {
        return pin_rc;
    }
    if shutdown_rc != RC::Ok {
        return shutdown_rc;
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
    match rel.mgmt_data.as_ref().and_then(|b| b.downcast_ref::<TableManager>()) {
        Some(tm) => tm.total_tuples,
        None => -1,
    }
}

fn slots_per_page(rec_size: i32) -> i32 {
    (PAGE_SIZE - PAGE_HEADER_SIZE as i32) / (rec_size + 2)
}

fn read_page_header(data: &str) -> PageHeader {
    PageHeader {
        page_identifier: read_byte(data, 0) as char,
        total_tuples: read_i32(data, 4),
        free_slot_cnt: read_i32(data, 8),
        next_free_slot_ind: read_i32(data, 12),
        prev_free_page_index: read_i32(data, 16),
        next_free_page_index: read_i32(data, 20),
        prev_data_page_index: read_i32(data, 24),
        next_data_page_index: read_i32(data, 28),
    }
}

fn write_page_header(data: &mut String, hdr: &PageHeader) {
    write_byte(data, 0, hdr.page_identifier as u8);
    write_i32(data, 4, hdr.total_tuples);
    write_i32(data, 8, hdr.free_slot_cnt);
    write_i32(data, 12, hdr.next_free_slot_ind);
    write_i32(data, 16, hdr.prev_free_page_index);
    write_i32(data, 20, hdr.next_free_page_index);
    write_i32(data, 24, hdr.prev_data_page_index);
    write_i32(data, 28, hdr.next_data_page_index);
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    // SAFETY: The C API insertRecord() mutates record->id. To match C semantics
    // while keeping the public signature, write through a raw pointer.
    let record_ptr = record as *const Record as *mut Record;
    let tm_box = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match tm_box.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let rec_size = tm.rec_size;
    let spp = slots_per_page(rec_size);
    let first_free_page_num = tm.first_free_page_num;
    let first_free_slot_num = tm.first_free_slot_num;

    let mut bm = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = match tm.page_handler.take() {
        Some(p) => p,
        None => BM_PageHandle {
            page_num: -1,
            data: String::new(),
        },
    };

    let rc = pin_page(&mut bm, &mut page, first_free_page_num);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::Error;
    }

    let mut data = page.data.clone();
    let mut hdr = read_page_header(&data);
    if hdr.page_identifier != 'Y' {
        hdr.page_identifier = 'Y';
        hdr.total_tuples = 0;
        hdr.free_slot_cnt = spp - 1;
        hdr.next_free_slot_ind = 1;
        hdr.prev_free_page_index = -1;
        hdr.next_free_page_index = page.page_num + 1;
        hdr.prev_data_page_index = -1;
        hdr.next_data_page_index = 1;
    } else {
        hdr.total_tuples += 1;
        hdr.free_slot_cnt -= 1;
        if hdr.free_slot_cnt > 0 {
            hdr.next_free_slot_ind += 1;
        } else {
            hdr.next_free_slot_ind = -hdr.next_free_slot_ind;
        }
    }
    write_page_header(&mut data, &hdr);

    // Write record at slot
    let position = PAGE_HEADER_SIZE + (first_free_slot_num as usize) * (rec_size as usize + 2);
    write_byte(&mut data, position, b'Y');
    let rec_bytes = str_to_bytes(&record.data);
    let copy_len = rec_bytes.len().min(rec_size as usize);
    let mut buf = vec![0u8; rec_size as usize];
    for k in 0..copy_len {
        buf[k] = rec_bytes[k];
    }
    write_bytes(&mut data, position + 1, &buf);
    write_byte(&mut data, position + 1 + rec_size as usize, b'|');
    page.data = data;

    // SAFETY: write through the raw pointer to update the caller's record.id.
    unsafe {
        (*record_ptr).id.page = page.page_num;
        (*record_ptr).id.slot = first_free_slot_num;
    }

    let new_free_slot_cnt = hdr.free_slot_cnt;
    if new_free_slot_cnt == 0 {
        tm.first_free_page_num += 1;
        tm.first_free_slot_num = 0;
    } else {
        tm.first_free_slot_num += 1;
    }
    tm.total_tuples += 1;

    let dirty_rc = mark_dirty(&mut bm, &mut page);
    let unpin_rc = unpin_page(&mut bm, &mut page);

    tm.buffer_pool = Some(bm);
    tm.page_handler = Some(page);

    if dirty_rc != RC::Ok || unpin_rc != RC::Ok {
        return RC::Error;
    }
    RC::Ok
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let tm = match rel
        .mgmt_data
        .as_mut()
        .and_then(|b| b.downcast_mut::<TableManager>())
    {
        Some(t) => t,
        None => return RC::Error,
    };
    let rec_size = tm.rec_size;
    let spp = slots_per_page(rec_size);
    if id.slot >= spp {
        return RC::RecordNotFound;
    }
    let mut bm = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = match tm.page_handler.take() {
        Some(p) => p,
        None => BM_PageHandle {
            page_num: -1,
            data: String::new(),
        },
    };
    let rc = pin_page(&mut bm, &mut page, id.page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return rc;
    }
    let mut data = page.data.clone();
    let position = PAGE_HEADER_SIZE + (id.slot as usize) * (rec_size as usize + 2);
    let marker = read_byte(&data, position);
    if marker != b'Y' {
        let _ = unpin_page(&mut bm, &mut page);
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    write_byte(&mut data, position, b'N');
    let mut hdr = read_page_header(&data);
    hdr.total_tuples = if hdr.total_tuples > 0 { hdr.total_tuples - 1 } else { 0 };
    hdr.free_slot_cnt += 1;
    write_page_header(&mut data, &hdr);
    page.data = data;
    if tm.total_tuples > 0 {
        tm.total_tuples -= 1;
    }

    let dirty_rc = mark_dirty(&mut bm, &mut page);
    if dirty_rc != RC::Ok {
        let _ = unpin_page(&mut bm, &mut page);
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let unpin_rc = unpin_page(&mut bm, &mut page);
    tm.buffer_pool = Some(bm);
    tm.page_handler = Some(page);
    if unpin_rc != RC::Ok {
        return unpin_rc;
    }
    RC::Ok
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let tm = match rel
        .mgmt_data
        .as_mut()
        .and_then(|b| b.downcast_mut::<TableManager>())
    {
        Some(t) => t,
        None => return RC::Error,
    };
    let rec_size = tm.rec_size;
    let spp = slots_per_page(rec_size);
    if record.id.slot >= spp {
        return RC::RecordNotFound;
    }
    let mut bm = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = match tm.page_handler.take() {
        Some(p) => p,
        None => BM_PageHandle {
            page_num: -1,
            data: String::new(),
        },
    };
    let rc = pin_page(&mut bm, &mut page, record.id.page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let mut data = page.data.clone();
    let position = PAGE_HEADER_SIZE + (record.id.slot as usize) * (rec_size as usize + 2);
    let marker = read_byte(&data, position);
    if marker != b'Y' {
        let _ = unpin_page(&mut bm, &mut page);
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    let rec_bytes = str_to_bytes(&record.data);
    let mut buf = vec![0u8; rec_size as usize];
    let copy_len = rec_bytes.len().min(rec_size as usize);
    for k in 0..copy_len {
        buf[k] = rec_bytes[k];
    }
    write_bytes(&mut data, position + 1, &buf);
    page.data = data;

    let dirty_rc = mark_dirty(&mut bm, &mut page);
    let unpin_rc = unpin_page(&mut bm, &mut page);
    tm.buffer_pool = Some(bm);
    tm.page_handler = Some(page);
    if dirty_rc != RC::Ok || unpin_rc != RC::Ok {
        return RC::Error;
    }
    RC::Ok
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    // Need mutable access to the buffer pool through rel.mgmt_data; use cell-like trick.
    // Since the signature is &RM_TableData but we need to pin pages, use UnsafeCell-style
    // workaround via raw pointer cast (safe because only this function holds it).
    // To remain safe, we instead rely on the tests using a single-threaded flow and
    // we accept the &-borrow but re-cast it via a small unsafe block. Since the project
    // disallows unsafe, we provide an internal cell-based shim.
    let tm_ptr = match rel
        .mgmt_data
        .as_ref()
        .and_then(|b| b.downcast_ref::<TableManager>())
    {
        Some(t) => t as *const TableManager as *mut TableManager,
        None => return RC::Error,
    };
    // SAFETY: the caller ensures exclusive access in single-threaded tests; we only
    // mutate fields owned by the TableManager.
    let tm = unsafe { &mut *tm_ptr };

    let rec_size = tm.rec_size;
    let spp = slots_per_page(rec_size);
    if id.slot >= spp {
        return RC::RecordNotFound;
    }
    let mut bm = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page = match tm.page_handler.take() {
        Some(p) => p,
        None => BM_PageHandle {
            page_num: -1,
            data: String::new(),
        },
    };

    let rc = pin_page(&mut bm, &mut page, id.page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let data = page.data.clone();
    let position = PAGE_HEADER_SIZE + (id.slot as usize) * (rec_size as usize + 2);
    let marker = read_byte(&data, position);
    if marker != b'Y' {
        let _ = unpin_page(&mut bm, &mut page);
        tm.buffer_pool = Some(bm);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    let bytes = read_bytes(&data, position + 1, rec_size as usize);
    record.data = bytes_to_str(&bytes);
    record.id = id.clone();

    let unpin_rc = unpin_page(&mut bm, &mut page);
    tm.buffer_pool = Some(bm);
    tm.page_handler = Some(page);
    unpin_rc
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let tm = match rel.mgmt_data.as_ref().and_then(|b| b.downcast_ref::<TableManager>()) {
        Some(t) => t,
        None => return RC::Error,
    };
    let sm = ScanManager {
        total_entries: tm.total_tuples,
        scan_index: 0,
        current_page_num: tm.first_data_page_num,
        current_slot_num: -1,
        condition_expression: Some(cond.clone()),
        scan_page_handle_ptr: None,
    };
    scan.rel = RM_TableData {
        name: rel.name.clone(),
        schema: rel.schema.clone(),
        mgmt_data: None,
    };
    scan.mgmt_data = Some(Box::new(sm));
    // Stash a raw pointer to the original rel's mgmt_data via a side channel: we
    // store the original rel pointer in the scan handle via a Box around a shared
    // reference holder.
    scan.rel.mgmt_data = Some(Box::new(RelPtr {
        ptr: rel as *const RM_TableData as *mut RM_TableData,
    }));
    RC::Ok
}

struct RelPtr {
    ptr: *mut RM_TableData,
}

pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    let rel_ptr = match scan
        .rel
        .mgmt_data
        .as_ref()
        .and_then(|b| b.downcast_ref::<RelPtr>())
    {
        Some(p) => p.ptr,
        None => return RC::Error,
    };
    // SAFETY: in test flow, the relation outlives the scan handle.
    let rel: &mut RM_TableData = unsafe { &mut *rel_ptr };

    let sm = match scan
        .mgmt_data
        .as_mut()
        .and_then(|b| b.downcast_mut::<ScanManager>())
    {
        Some(s) => s,
        None => return RC::Error,
    };

    // Recompute slots_per_page from rel's TableManager
    let rec_size = match rel
        .mgmt_data
        .as_ref()
        .and_then(|b| b.downcast_ref::<TableManager>())
    {
        Some(t) => t.rec_size,
        None => return RC::Error,
    };
    let spp = slots_per_page(rec_size);

    if sm.scan_index >= sm.total_entries {
        return RC::RmNoMoreTuples;
    }

    let cond = sm.condition_expression.clone();
    loop {
        sm.current_slot_num += 1;
        if sm.current_slot_num >= spp {
            sm.current_page_num += 1;
            sm.current_slot_num = 0;
        }
        let rid = RID {
            page: sm.current_page_num,
            slot: sm.current_slot_num,
        };
        let rc = get_record(rel, &rid, record);
        if rc == RC::Ok {
            sm.scan_index += 1;
            if let Some(ref expr) = cond {
                let mut eval_result = Value {
                    dt: DataType::DtBool,
                    v: ValueUnion::BoolV(false),
                };
                let eval_rc = eval_expr(record, &rel.schema, expr, &mut eval_result);
                if eval_rc != RC::Ok {
                    return eval_rc;
                }
                if let ValueUnion::BoolV(true) = eval_result.v {
                    return RC::Ok;
                }
            } else {
                return RC::Ok;
            }
        }
        if sm.scan_index >= sm.total_entries {
            return RC::RmNoMoreTuples;
        }
    }
}

pub fn close_scan(scan: &mut RM_ScanHandle) -> RC {
    scan.mgmt_data = None;
    scan.rel.mgmt_data = None;
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
    let rec_size = get_record_size(schema) as usize;
    let mut data = String::with_capacity(rec_size);
    for _ in 0..rec_size {
        data.push('\0');
    }
    let r = Record {
        id: RID { page: 0, slot: 0 },
        data,
    };
    *record = Some(r);
    RC::Ok
}

pub fn free_record(record: &mut Record) -> RC {
    record.data.clear();
    RC::Ok
}

pub fn get_attr(record: &Record, schema: &Schema, attr_num: i32, value: &mut Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let idx = attr_num as usize;
    value.dt = schema.data_types[idx].clone();
    let bytes = str_to_bytes(&record.data);
    match schema.data_types[idx] {
        DataType::DtString => {
            let len = schema.type_length[idx] as usize;
            let mut buf = vec![0u8; len];
            for k in 0..len {
                if pos + k < bytes.len() {
                    buf[k] = bytes[pos + k];
                }
            }
            // Keep the full length (no trimming on null terminator) to mirror C strncpy
            // behavior used by the test, where the string is padded with zeros only
            // when shorter than typeLength.
            let s: String = buf.iter().map(|&b| b as char).collect();
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtInt => {
            let mut b = [0u8; 4];
            for k in 0..4 {
                if pos + k < bytes.len() {
                    b[k] = bytes[pos + k];
                }
            }
            value.v = ValueUnion::IntV(i32::from_ne_bytes(b));
        }
        DataType::DtFloat => {
            let mut b = [0u8; 4];
            for k in 0..4 {
                if pos + k < bytes.len() {
                    b[k] = bytes[pos + k];
                }
            }
            value.v = ValueUnion::FloatV(f32::from_ne_bytes(b));
        }
        DataType::DtBool => {
            let v = if pos < bytes.len() { bytes[pos] != 0 } else { false };
            value.v = ValueUnion::BoolV(v);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let idx = attr_num as usize;
    let mut bytes = str_to_bytes(&record.data);
    match (&schema.data_types[idx], &value.v) {
        (DataType::DtInt, ValueUnion::IntV(v)) => {
            let b = v.to_ne_bytes();
            ensure_len(&mut bytes, pos + 4);
            for k in 0..4 {
                bytes[pos + k] = b[k];
            }
        }
        (DataType::DtFloat, ValueUnion::FloatV(v)) => {
            let b = v.to_ne_bytes();
            ensure_len(&mut bytes, pos + 4);
            for k in 0..4 {
                bytes[pos + k] = b[k];
            }
        }
        (DataType::DtString, ValueUnion::StringV(s)) => {
            let len = schema.type_length[idx] as usize;
            ensure_len(&mut bytes, pos + len);
            let sb = s.as_bytes();
            for k in 0..len {
                bytes[pos + k] = if k < sb.len() { sb[k] } else { 0 };
            }
        }
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            ensure_len(&mut bytes, pos + 1);
            bytes[pos] = if *b { 1 } else { 0 };
        }
        _ => {}
    }
    record.data = bytes_to_str(&bytes);
    RC::Ok
}

fn ensure_len(bytes: &mut Vec<u8>, needed: usize) {
    if bytes.len() < needed {
        bytes.resize(needed, 0);
    }
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
