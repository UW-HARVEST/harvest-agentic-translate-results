use crate::buffer_mgr::{
    init_buffer_pool, mark_dirty, pin_page, shutdown_buffer_pool, unpin_page, BM_BufferPool,
    BM_PageHandle, ReplacementStrategy,
};
use crate::dberror::{PAGE_SIZE, RC};
use crate::storage_mgr::{create_page_file, destroy_page_file};
use crate::tables::{DataType, Record, Schema, Value, ValueUnion, RID, RM_TableData};
use crate::expr::Expr;
use std::cell::RefCell;

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
const PAGE_HEADER_SIZE: usize = 32; // 1 char + 3 padding + 7 ints

fn page_size() -> usize {
    PAGE_SIZE as usize
}

// ---------- byte helpers ----------

fn make_zero_str(n: usize) -> String {
    // SAFETY: zero bytes are valid UTF-8
    unsafe { String::from_utf8_unchecked(vec![0u8; n]) }
}

fn write_i32(buf: &mut [u8], off: usize, val: i32) {
    let bytes = val.to_ne_bytes();
    buf[off..off + 4].copy_from_slice(&bytes);
}

fn read_i32(buf: &[u8], off: usize) -> i32 {
    let mut b = [0u8; 4];
    let take = (buf.len().saturating_sub(off)).min(4);
    if take > 0 {
        b[..take].copy_from_slice(&buf[off..off + take]);
    }
    i32::from_ne_bytes(b)
}

fn data_as_mut(page: &mut BM_PageHandle) -> &mut [u8] {
    // SAFETY: We treat the underlying buffer as binary bytes; the storage is
    // a fixed-size buffer for the page.
    unsafe {
        let v = page.data.as_mut_vec();
        if v.len() < page_size() {
            v.resize(page_size(), 0);
        }
        &mut v[..page_size()]
    }
}

fn data_as(page: &BM_PageHandle) -> &[u8] {
    let bytes = page.data.as_bytes();
    if bytes.len() >= page_size() {
        &bytes[..page_size()]
    } else {
        bytes
    }
}

fn ensure_page_capacity(page: &mut BM_PageHandle) {
    let bytes = page.data.as_bytes();
    if bytes.len() < page_size() {
        let mut v = std::mem::take(&mut page.data).into_bytes();
        v.resize(page_size(), 0);
        page.data = unsafe { String::from_utf8_unchecked(v) };
    }
}

// Read/write page header from the first PAGE_HEADER_SIZE bytes of the page.
fn read_page_header(buf: &[u8]) -> PageHeader {
    PageHeader {
        page_identifier: buf[0] as char,
        total_tuples: read_i32(buf, 4),
        free_slot_cnt: read_i32(buf, 8),
        next_free_slot_ind: read_i32(buf, 12),
        prev_free_page_index: read_i32(buf, 16),
        next_free_page_index: read_i32(buf, 20),
        prev_data_page_index: read_i32(buf, 24),
        next_data_page_index: read_i32(buf, 28),
    }
}

fn write_page_header(buf: &mut [u8], h: &PageHeader) {
    buf[0] = h.page_identifier as u8;
    // bytes 1..4 reserved as padding
    write_i32(buf, 4, h.total_tuples);
    write_i32(buf, 8, h.free_slot_cnt);
    write_i32(buf, 12, h.next_free_slot_ind);
    write_i32(buf, 16, h.prev_free_page_index);
    write_i32(buf, 20, h.next_free_page_index);
    write_i32(buf, 24, h.prev_data_page_index);
    write_i32(buf, 28, h.next_data_page_index);
}

// ---------- public API ----------

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
    let mut bp = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut bp, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let mut page = BM_PageHandle {
        page_num: 0,
        data: make_zero_str(page_size()),
    };
    let rc = pin_page(&mut bp, &mut page, 0);
    if rc != RC::Ok {
        let _ = shutdown_buffer_pool(&mut bp);
        return rc;
    }

    {
        let buf = data_as_mut(&mut page);
        // Header layout: 7 ints
        let total_tuples = 0i32;
        let rec_size = get_record_size(schema);
        let first_free_page_num = 1i32;
        let first_free_slot_num = 0i32;
        let first_data_page_num = -1i32;

        write_i32(buf, 0, total_tuples);
        write_i32(buf, 4, rec_size);
        write_i32(buf, 8, first_free_page_num);
        write_i32(buf, 12, first_free_slot_num);
        write_i32(buf, 16, first_data_page_num);
        write_i32(buf, 20, schema.num_attr);
        write_i32(buf, 24, schema.key_size);

        // Attribute info follows
        let mut off = 28usize;
        for i in 0..(schema.num_attr as usize) {
            let name_bytes = schema.attr_names[i].as_bytes();
            let copy_len = name_bytes.len().min(MAX_ATTR_NAME_LEN);
            // zero out region
            for j in 0..MAX_ATTR_NAME_LEN {
                buf[off + j] = 0;
            }
            buf[off..off + copy_len].copy_from_slice(&name_bytes[..copy_len]);
            off += MAX_ATTR_NAME_LEN;
            let dt_code = match schema.data_types[i] {
                DataType::DtInt => 0,
                DataType::DtString => 1,
                DataType::DtFloat => 2,
                DataType::DtBool => 3,
            };
            write_i32(buf, off, dt_code);
            off += 4;
            write_i32(buf, off, schema.type_length[i]);
            off += 4;
        }
        // Key attrs
        for i in 0..(schema.key_size as usize) {
            write_i32(buf, off, schema.key_attrs[i]);
            off += 4;
        }
    }

    let rc = mark_dirty(&mut bp, &mut page);
    if rc != RC::Ok {
        let _ = shutdown_buffer_pool(&mut bp);
        return rc;
    }
    let rc = unpin_page(&mut bp, &mut page);
    if rc != RC::Ok {
        let _ = shutdown_buffer_pool(&mut bp);
        return rc;
    }
    let rc = shutdown_buffer_pool(&mut bp);
    if rc != RC::Ok {
        return rc;
    }
    RC::Ok
}

pub fn open_table(rel: &mut RM_TableData, name: &str) -> RC {
    let mut bp = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut bp, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let mut page = BM_PageHandle {
        page_num: 0,
        data: make_zero_str(page_size()),
    };
    let rc = pin_page(&mut bp, &mut page, 0);
    if rc != RC::Ok {
        let _ = shutdown_buffer_pool(&mut bp);
        return rc;
    }

    let (tm, schema) = {
        let buf = data_as(&page);
        let total_tuples = read_i32(buf, 0);
        let rec_size = read_i32(buf, 4);
        let first_free_page_num = read_i32(buf, 8);
        let first_free_slot_num = read_i32(buf, 12);
        let first_data_page_num = read_i32(buf, 16);
        let num_attr = read_i32(buf, 20);
        let key_size = read_i32(buf, 24);

        let mut off = 28usize;
        let mut attr_names = Vec::with_capacity(num_attr as usize);
        let mut data_types = Vec::with_capacity(num_attr as usize);
        let mut type_length = Vec::with_capacity(num_attr as usize);
        for _ in 0..(num_attr as usize) {
            let raw = &buf[off..off + MAX_ATTR_NAME_LEN];
            let name_end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            let name = String::from_utf8_lossy(&raw[..name_end]).to_string();
            attr_names.push(name);
            off += MAX_ATTR_NAME_LEN;
            let dt_code = read_i32(buf, off);
            off += 4;
            let dt = match dt_code {
                0 => DataType::DtInt,
                1 => DataType::DtString,
                2 => DataType::DtFloat,
                _ => DataType::DtBool,
            };
            data_types.push(dt);
            type_length.push(read_i32(buf, off));
            off += 4;
        }
        let mut key_attrs = Vec::with_capacity(key_size as usize);
        for _ in 0..(key_size as usize) {
            key_attrs.push(read_i32(buf, off));
            off += 4;
        }

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
            buffer_pool: Some(bp),
            page_handler: Some(page),
        };
        (tm, schema)
    };

    // Need to take the page out of tm to unpin first
    let mut tm = tm;
    {
        let bp_mut = tm.buffer_pool.as_mut().unwrap();
        let ph_mut = tm.page_handler.as_mut().unwrap();
        let _ = unpin_page(bp_mut, ph_mut);
    }

    rel.name = name.to_string();
    rel.schema = schema;
    rel.mgmt_data = Some(Box::new(RefCell::new(tm)));
    RC::Ok
}

pub fn close_table(rel: &mut RM_TableData) -> RC {
    let cell: Box<RefCell<TableManager>> = match rel.mgmt_data.take() {
        Some(b) => match b.downcast::<RefCell<TableManager>>() {
            Ok(t) => t,
            Err(_) => return RC::Error,
        },
        None => return RC::Error,
    };
    let mut tm = cell.into_inner();

    // Pin page 0, write back the table header integers
    {
        let bp = tm.buffer_pool.as_mut().unwrap();
        let mut local_page = BM_PageHandle {
            page_num: 0,
            data: make_zero_str(page_size()),
        };
        let rc = pin_page(bp, &mut local_page, 0);
        if rc == RC::Ok {
            {
                let buf = data_as_mut(&mut local_page);
                write_i32(buf, 0, tm.total_tuples);
                write_i32(buf, 4, tm.rec_size);
                write_i32(buf, 8, tm.first_free_page_num);
                write_i32(buf, 12, tm.first_free_slot_num);
                write_i32(buf, 16, tm.first_data_page_num);
            }
            let _ = mark_dirty(bp, &mut local_page);
            let _ = unpin_page(bp, &mut local_page);
        }
        let _ = shutdown_buffer_pool(bp);
    }

    tm.buffer_pool = None;
    tm.page_handler = None;
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() {
        return RC::InvalidHeader;
    }
    destroy_page_file(name)
}

fn tm_ref(rel: &RM_TableData) -> Option<&RefCell<TableManager>> {
    rel.mgmt_data
        .as_ref()
        .and_then(|b| b.downcast_ref::<RefCell<TableManager>>())
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    match tm_ref(rel) {
        Some(c) => c.borrow().total_tuples,
        None => -1,
    }
}

fn slots_per_page(rec_size: i32) -> i32 {
    (page_size() as i32 - PAGE_HEADER_SIZE as i32) / (rec_size + 2)
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let cell = match tm_ref(rel) {
        Some(c) => c,
        None => return RC::Error,
    };
    let mut tm = cell.borrow_mut();
    let rec_size = tm.rec_size;
    let slots = slots_per_page(rec_size);

    let target_page = tm.first_free_page_num;
    let target_slot = tm.first_free_slot_num;

    // Take owned page and bp temporarily.
    let mut bp = tm.buffer_pool.take().unwrap_or_else(|| BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    });
    let mut page = tm
        .page_handler
        .take()
        .unwrap_or_else(|| BM_PageHandle { page_num: 0, data: make_zero_str(page_size()) });

    let rc = pin_page(&mut bp, &mut page, target_page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page);
        return RC::Error;
    }

    {
        ensure_page_capacity(&mut page);
        let buf = data_as_mut(&mut page);
        let mut header = read_page_header(buf);
        if header.page_identifier != 'Y' {
            header.page_identifier = 'Y';
            header.total_tuples = 0;
            header.free_slot_cnt = slots - 1;
            header.next_free_slot_ind = 1;
            header.prev_free_page_index = -1;
            header.next_free_page_index = target_page + 1;
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
        let pos = PAGE_HEADER_SIZE + (target_slot as usize) * (rec_size as usize + 2);
        buf[pos] = b'Y';
        let rd = record.data.as_bytes();
        let take = rd.len().min(rec_size as usize);
        buf[pos + 1..pos + 1 + take].copy_from_slice(&rd[..take]);
        // remaining bytes within record area zeroed (already are)
        buf[pos + rec_size as usize + 1] = b'|';

        write_page_header(buf, &header);

        if header.free_slot_cnt == 0 {
            tm.first_free_page_num += 1;
            tm.first_free_slot_num = 0;
        } else {
            tm.first_free_slot_num += 1;
        }
    }

    tm.total_tuples += 1;

    let dirty_status = mark_dirty(&mut bp, &mut page);
    let unpin_status = unpin_page(&mut bp, &mut page);

    // Mirror C's behavior: insertRecord mutates the input record's id field
    // through a pointer. The Rust signature is `&Record`, so we need to update
    // through interior mutability. Since the public Record struct does not use
    // Cell, we use a raw-pointer cast. The caller is the unique owner during
    // this call, matching the C contract.
    #[allow(invalid_reference_casting)]
    unsafe {
        let r_mut = &record.id as *const RID as *mut RID;
        std::ptr::write(r_mut, RID { page: target_page, slot: target_slot });
    }

    tm.buffer_pool = Some(bp);
    tm.page_handler = Some(page);

    if dirty_status != RC::Ok || unpin_status != RC::Ok {
        return RC::Error;
    }
    RC::Ok
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let cell = match tm_ref(rel) {
        Some(c) => c,
        None => return RC::Error,
    };
    let mut tm = cell.borrow_mut();
    let rec_size = tm.rec_size;
    let slots = slots_per_page(rec_size);
    if id.slot >= slots {
        return RC::RecordNotFound;
    }
    let mut bp = tm.buffer_pool.take().unwrap();
    let mut page = tm.page_handler.take().unwrap();

    let rc = pin_page(&mut bp, &mut page, id.page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page);
        return rc;
    }
    let mut not_found = false;
    {
        ensure_page_capacity(&mut page);
        let buf = data_as_mut(&mut page);
        let pos = PAGE_HEADER_SIZE + (id.slot as usize) * (rec_size as usize + 2);
        if buf[pos] != b'Y' {
            not_found = true;
        } else {
            buf[pos] = b'N';
            let mut header = read_page_header(buf);
            header.total_tuples = (header.total_tuples - 1).max(0);
            header.free_slot_cnt += 1;
            write_page_header(buf, &header);
        }
    }
    if not_found {
        let _ = unpin_page(&mut bp, &mut page);
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    tm.total_tuples = (tm.total_tuples - 1).max(0);

    let dirty_rc = mark_dirty(&mut bp, &mut page);
    if dirty_rc != RC::Ok {
        let _ = unpin_page(&mut bp, &mut page);
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let unpin_rc = unpin_page(&mut bp, &mut page);
    tm.buffer_pool = Some(bp);
    tm.page_handler = Some(page);
    if unpin_rc != RC::Ok {
        return unpin_rc;
    }
    RC::Ok
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let cell = match tm_ref(rel) {
        Some(c) => c,
        None => return RC::Error,
    };
    let mut tm = cell.borrow_mut();
    let rec_size = tm.rec_size;
    let slots = slots_per_page(rec_size);
    if record.id.slot >= slots {
        return RC::RecordNotFound;
    }
    let mut bp = tm.buffer_pool.take().unwrap();
    let mut page = tm.page_handler.take().unwrap();

    let rc = pin_page(&mut bp, &mut page, record.id.page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let mut not_found = false;
    {
        ensure_page_capacity(&mut page);
        let buf = data_as_mut(&mut page);
        let pos = PAGE_HEADER_SIZE + (record.id.slot as usize) * (rec_size as usize + 2);
        if buf[pos] != b'Y' {
            not_found = true;
        } else {
            let rd = record.data.as_bytes();
            let take = rd.len().min(rec_size as usize);
            buf[pos + 1..pos + 1 + take].copy_from_slice(&rd[..take]);
        }
    }
    if not_found {
        let _ = unpin_page(&mut bp, &mut page);
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page);
        return RC::RecordNotFound;
    }
    let dirty_rc = mark_dirty(&mut bp, &mut page);
    let unpin_rc = unpin_page(&mut bp, &mut page);
    tm.buffer_pool = Some(bp);
    tm.page_handler = Some(page);
    if dirty_rc != RC::Ok || unpin_rc != RC::Ok {
        return RC::Error;
    }
    RC::Ok
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    let cell = match tm_ref(rel) {
        Some(c) => c,
        None => return RC::Error,
    };
    let mut tm = cell.borrow_mut();
    let rec_size = tm.rec_size;
    let slots = slots_per_page(rec_size);
    if id.slot >= slots {
        return RC::RecordNotFound;
    }

    let mut bp = tm.buffer_pool.take().unwrap();
    let mut page = tm.page_handler.take().unwrap();

    let rc = pin_page(&mut bp, &mut page, id.page);
    if rc != RC::Ok {
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page);
        return RC::Error;
    }
    let mut not_found = false;
    {
        ensure_page_capacity(&mut page);
        let buf = data_as(&page);
        let pos = PAGE_HEADER_SIZE + (id.slot as usize) * (rec_size as usize + 2);
        if buf[pos] != b'Y' {
            not_found = true;
        } else {
            let src = &buf[pos + 1..pos + 1 + rec_size as usize];
            // Set record.data to be exactly rec_size bytes (or larger if pre-allocated)
            ensure_record_capacity(record, rec_size as usize);
            unsafe {
                let v = record.data.as_mut_vec();
                v[..rec_size as usize].copy_from_slice(src);
            }
            record.id = id.clone();
        }
    }
    let unpin_rc = unpin_page(&mut bp, &mut page);
    tm.buffer_pool = Some(bp);
    tm.page_handler = Some(page);
    if not_found {
        return RC::RecordNotFound;
    }
    unpin_rc
}

fn ensure_record_capacity(record: &mut Record, n: usize) {
    let bytes = record.data.as_bytes();
    if bytes.len() < n {
        let mut v = std::mem::take(&mut record.data).into_bytes();
        v.resize(n, 0);
        record.data = unsafe { String::from_utf8_unchecked(v) };
    }
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let cell = match tm_ref(rel) {
        Some(c) => c,
        None => return RC::Error,
    };
    let tm = cell.borrow();
    let total_entries = tm.total_tuples;
    let first_data_page_num = tm.first_data_page_num;
    drop(tm);

    let scan_mgr = ScanManager {
        total_entries,
        scan_index: 0,
        current_page_num: first_data_page_num,
        current_slot_num: -1,
        condition_expression: Some(cond.clone()),
        scan_page_handle_ptr: None,
    };
    scan.mgmt_data = Some(Box::new(scan_mgr));
    // We need next() to access the original rel's TableManager state. Save a
    // pointer-as-usize to the original rel; the caller must keep rel alive for
    // the duration of the scan.
    let rel_ptr = rel as *const RM_TableData as usize;
    scan.rel = RM_TableData {
        name: rel.name.clone(),
        schema: rel.schema.clone(),
        mgmt_data: Some(Box::new(rel_ptr)),
    };
    RC::Ok
}

pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    // Recover the original rel pointer
    let rel_ptr: usize = match scan
        .rel
        .mgmt_data
        .as_ref()
        .and_then(|b| b.downcast_ref::<usize>())
    {
        Some(p) => *p,
        None => return RC::Error,
    };
    let rel: &RM_TableData = unsafe { &*(rel_ptr as *const RM_TableData) };
    let cell = match tm_ref(rel) {
        Some(c) => c,
        None => return RC::Error,
    };
    let rec_size = cell.borrow().rec_size;
    let slots = slots_per_page(rec_size);

    let scan_mgr = match scan
        .mgmt_data
        .as_mut()
        .and_then(|b| b.downcast_mut::<ScanManager>())
    {
        Some(s) => s,
        None => return RC::Error,
    };
    if scan_mgr.scan_index >= scan_mgr.total_entries {
        return RC::RmNoMoreTuples;
    }

    let cond = scan_mgr.condition_expression.clone();
    let mut eval_result = Value {
        dt: DataType::DtInt,
        v: ValueUnion::IntV(-1),
    };

    loop {
        scan_mgr.current_slot_num += 1;
        if scan_mgr.current_slot_num >= slots {
            scan_mgr.current_page_num += 1;
            scan_mgr.current_slot_num = 0;
        }
        let cur_id = RID {
            page: scan_mgr.current_page_num,
            slot: scan_mgr.current_slot_num,
        };
        let rc = get_record(rel, &cur_id, record);
        if rc == RC::Ok {
            scan_mgr.scan_index += 1;
            match &cond {
                Some(c) => {
                    // Treat the "no condition" sentinel: a const-true bool returns true.
                    let rc = crate::expr::eval_expr(record, &rel.schema, c, &mut eval_result);
                    if rc != RC::Ok {
                        return rc;
                    }
                    if let ValueUnion::BoolV(true) = eval_result.v {
                        return RC::Ok;
                    }
                }
                None => {
                    return RC::Ok;
                }
            }
        }
        if scan_mgr.scan_index >= scan_mgr.total_entries {
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
    let mut total = 0i32;
    for i in 0..(schema.num_attr as usize) {
        let dt = &schema.data_types[i];
        let len = schema.type_length[i];
        match dt {
            DataType::DtString => total += len,
            DataType::DtInt => total += std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => total += std::mem::size_of::<f32>() as i32,
            DataType::DtBool => total += std::mem::size_of::<bool>() as i32,
        }
    }
    let pad = total % 4;
    if pad != 0 {
        total += 4 - pad;
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
    let data = make_zero_str(rec_size as usize + 1);
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
    let idx = attr_num as usize;
    let dt = &schema.data_types[idx];
    value.dt = dt.clone();
    let pos = get_attr_pos(schema, attr_num) as usize;
    let bytes = record.data.as_bytes();
    match dt {
        DataType::DtString => {
            let len = schema.type_length[idx] as usize;
            let end = (pos + len).min(bytes.len());
            let raw = if pos < bytes.len() {
                &bytes[pos..end]
            } else {
                &bytes[..0]
            };
            let truncated: Vec<u8> = raw.iter().cloned().take_while(|&b| b != 0).collect();
            // Pad with zero bytes if shorter than len? C uses strncpy then a zero-padded buffer.
            // For comparison purposes (memcmp), strncpy zero-fills the remaining bytes.
            // Here we keep the trimmed string for the Value.
            let s = String::from_utf8_lossy(&truncated).to_string();
            // Replicate strncpy padding behaviour by storing exactly len bytes
            // (zero-padded) inside the StringV.
            let mut padded = vec![0u8; len];
            let n = truncated.len().min(len);
            padded[..n].copy_from_slice(&truncated[..n]);
            // For test compatibility we keep just the trimmed text.
            let _ = s;
            value.v = ValueUnion::StringV(
                String::from_utf8_lossy(&padded[..n]).to_string(),
            );
        }
        DataType::DtInt => {
            value.v = ValueUnion::IntV(read_i32(bytes, pos));
        }
        DataType::DtFloat => {
            let mut b = [0u8; 4];
            let take = (bytes.len() - pos.min(bytes.len())).min(4);
            if take > 0 {
                b[..take].copy_from_slice(&bytes[pos..pos + take]);
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
    let idx = attr_num as usize;
    let pos = get_attr_pos(schema, attr_num) as usize;
    ensure_record_capacity(
        record,
        pos + match schema.data_types[idx] {
            DataType::DtString => schema.type_length[idx] as usize,
            DataType::DtInt | DataType::DtFloat => 4,
            DataType::DtBool => 1,
        },
    );
    let buf = unsafe { record.data.as_mut_vec() };
    match (&schema.data_types[idx], &value.v) {
        (DataType::DtInt, ValueUnion::IntV(i)) => {
            buf[pos..pos + 4].copy_from_slice(&i.to_ne_bytes());
        }
        (DataType::DtFloat, ValueUnion::FloatV(f)) => {
            buf[pos..pos + 4].copy_from_slice(&f.to_ne_bytes());
        }
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            buf[pos] = if *b { 1 } else { 0 };
        }
        (DataType::DtString, ValueUnion::StringV(s)) => {
            let len = schema.type_length[idx] as usize;
            // memcpy(target, value->v.stringV, schema->typeLength[attrNum]);
            let bytes = s.as_bytes();
            let take = bytes.len().min(len);
            buf[pos..pos + take].copy_from_slice(&bytes[..take]);
            // Zero remainder so the slot has consistent contents.
            for i in pos + take..pos + len {
                buf[i] = 0;
            }
        }
        _ => {}
    }
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut pos = 0i32;
    for i in 0..(attr_num as usize) {
        match schema.data_types[i] {
            DataType::DtString => pos += schema.type_length[i],
            DataType::DtInt => pos += std::mem::size_of::<i32>() as i32,
            DataType::DtFloat => pos += std::mem::size_of::<f32>() as i32,
            DataType::DtBool => pos += std::mem::size_of::<bool>() as i32,
        }
    }
    pos
}
