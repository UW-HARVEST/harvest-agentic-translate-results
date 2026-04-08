use crate::{dberror::RC, expr::Expr, tables::{Record, Schema, RM_TableData, RID}};
use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle};
use crate::tables::{DataType, Value, ValueUnion};
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

pub struct TableManagerCell(pub RefCell<TableManager>);

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

const PAGE_HEADER_SIZE: usize = 32;
const MAX_ATTR_NAME_LEN: usize = 15;
const SIZEOF_INT: i32 = 4;
const SIZEOF_FLOAT: i32 = 4;
const SIZEOF_BOOL: i32 = 2;

fn get_cell(rel: &RM_TableData) -> &TableManagerCell {
    rel.mgmt_data.as_ref().unwrap().downcast_ref::<TableManagerCell>().unwrap()
}

fn page_header_to_bytes(ph: &PageHeader) -> Vec<u8> {
    let mut buf = vec![0u8; PAGE_HEADER_SIZE];
    buf[0] = ph.page_identifier as u8;
    buf[4..8].copy_from_slice(&ph.total_tuples.to_ne_bytes());
    buf[8..12].copy_from_slice(&ph.free_slot_cnt.to_ne_bytes());
    buf[12..16].copy_from_slice(&ph.next_free_slot_ind.to_ne_bytes());
    buf[16..20].copy_from_slice(&ph.prev_free_page_index.to_ne_bytes());
    buf[20..24].copy_from_slice(&ph.next_free_page_index.to_ne_bytes());
    buf[24..28].copy_from_slice(&ph.prev_data_page_index.to_ne_bytes());
    buf[28..32].copy_from_slice(&ph.next_data_page_index.to_ne_bytes());
    buf
}

fn page_header_from_bytes(data: &[u8]) -> PageHeader {
    PageHeader {
        page_identifier: data[0] as char,
        total_tuples: i32::from_ne_bytes([data[4], data[5], data[6], data[7]]),
        free_slot_cnt: i32::from_ne_bytes([data[8], data[9], data[10], data[11]]),
        next_free_slot_ind: i32::from_ne_bytes([data[12], data[13], data[14], data[15]]),
        prev_free_page_index: i32::from_ne_bytes([data[16], data[17], data[18], data[19]]),
        next_free_page_index: i32::from_ne_bytes([data[20], data[21], data[22], data[23]]),
        prev_data_page_index: i32::from_ne_bytes([data[24], data[25], data[26], data[27]]),
        next_data_page_index: i32::from_ne_bytes([data[28], data[29], data[30], data[31]]),
    }
}

fn write_i32_to(buf: &mut Vec<u8>, pos: &mut usize, val: i32) {
    buf[*pos..*pos + 4].copy_from_slice(&val.to_ne_bytes());
    *pos += 4;
}

fn read_i32_from(buf: &[u8], pos: &mut usize) -> i32 {
    let val = i32::from_ne_bytes([buf[*pos], buf[*pos+1], buf[*pos+2], buf[*pos+3]]);
    *pos += 4;
    val
}

/// Take buffer_pool and page_handler out of TableManager, run a closure, put them back.
fn with_bp_ph<F, R>(tm: &mut TableManager, f: F) -> R
where F: FnOnce(&mut TableManager, &mut BM_BufferPool, &mut BM_PageHandle) -> R
{
    let mut bp = tm.buffer_pool.take().unwrap();
    let mut ph = tm.page_handler.take().unwrap();
    let result = f(tm, &mut bp, &mut ph);
    tm.buffer_pool = Some(bp);
    tm.page_handler = Some(ph);
    result
}

pub fn init_record_manager(_mgmt_data: Option<Box<dyn std::any::Any>>) -> RC {
    RC::Ok
}

pub fn shutdown_record_manager() -> RC {
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() { return RC::InvalidHeader; }
    crate::storage_mgr::destroy_page_file(name)
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    get_cell(rel).0.borrow().total_tuples
}

pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total = 0i32;
    for i in 0..schema.num_attr as usize {
        match schema.data_types[i] {
            DataType::DtString => total += schema.type_length[i],
            DataType::DtInt => total += SIZEOF_INT,
            DataType::DtFloat => total += SIZEOF_FLOAT,
            DataType::DtBool => total += SIZEOF_BOOL,
        }
    }
    let padding = total % 4;
    if padding != 0 { total += 4 - padding; }
    total
}

pub fn create_schema(num_attr: i32, attr_names: Vec<String>, data_types: Vec<DataType>,
    type_length: Vec<i32>, key_size: i32, keys: Vec<i32>) -> Schema {
    Schema { num_attr, attr_names, data_types, type_length, key_attrs: keys, key_size }
}

pub fn free_schema(_schema: &mut Schema) -> RC { RC::Ok }

pub fn create_record(record: &mut Option<Record>, schema: &Schema) -> RC {
    let rec_size = get_record_size(schema) as usize;
    *record = Some(Record {
        id: RID { page: 0, slot: 0 },
        data: unsafe { String::from_utf8_unchecked(vec![0u8; rec_size + 1]) },
    });
    RC::Ok
}

pub fn free_record(record: &mut Record) -> RC {
    record.data = String::new();
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut pos = 0i32;
    for i in 0..attr_num as usize {
        match schema.data_types[i] {
            DataType::DtString => pos += schema.type_length[i],
            DataType::DtInt => pos += SIZEOF_INT,
            DataType::DtFloat => pos += SIZEOF_FLOAT,
            DataType::DtBool => pos += SIZEOF_BOOL,
        }
    }
    pos
}

pub fn get_attr(record: &Record, schema: &Schema, attr_num: i32, value: &mut Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let data = record.data.as_bytes();
    value.dt = schema.data_types[attr_num as usize].clone();
    match schema.data_types[attr_num as usize] {
        DataType::DtInt => {
            let mut b = [0u8; 4];
            for j in 0..4 { if pos+j < data.len() { b[j] = data[pos+j]; } }
            value.v = ValueUnion::IntV(i32::from_ne_bytes(b));
        }
        DataType::DtFloat => {
            let mut b = [0u8; 4];
            for j in 0..4 { if pos+j < data.len() { b[j] = data[pos+j]; } }
            value.v = ValueUnion::FloatV(f32::from_ne_bytes(b));
        }
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let end = (pos + len).min(data.len());
            let s = String::from_utf8_lossy(&data[pos..end]).trim_end_matches('\0').to_string();
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtBool => {
            let mut b = [0u8; 2];
            for j in 0..2 { if pos+j < data.len() { b[j] = data[pos+j]; } }
            value.v = ValueUnion::BoolV(i16::from_ne_bytes(b) != 0);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let data = unsafe { record.data.as_bytes_mut() };
    match schema.data_types[attr_num as usize] {
        DataType::DtInt => {
            if let ValueUnion::IntV(v) = &value.v {
                let b = v.to_ne_bytes();
                for j in 0..4 { if pos+j < data.len() { data[pos+j] = b[j]; } }
            }
        }
        DataType::DtFloat => {
            if let ValueUnion::FloatV(v) = &value.v {
                let b = v.to_ne_bytes();
                for j in 0..4 { if pos+j < data.len() { data[pos+j] = b[j]; } }
            }
        }
        DataType::DtString => {
            if let ValueUnion::StringV(v) = &value.v {
                let len = schema.type_length[attr_num as usize] as usize;
                let src = v.as_bytes();
                let copy_len = src.len().min(len);
                for j in 0..copy_len { if pos+j < data.len() { data[pos+j] = src[j]; } }
            }
        }
        DataType::DtBool => {
            if let ValueUnion::BoolV(v) = &value.v {
                let val: i16 = if *v { 1 } else { 0 };
                let b = val.to_ne_bytes();
                for j in 0..2 { if pos+j < data.len() { data[pos+j] = b[j]; } }
            }
        }
    }
    RC::Ok
}

pub fn create_table(name: &str, schema: &Schema) -> RC {
    if name.is_empty() { return RC::GeneralError; }
    let ps = crate::dberror::PAGE_SIZE as usize;

    let rc = crate::storage_mgr::create_page_file(name);
    if rc != RC::Ok { return rc; }

    let mut bp = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: crate::buffer_mgr::ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    let rc = crate::buffer_mgr::init_buffer_pool(&mut bp, name, 3,
        crate::buffer_mgr::ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok { return rc; }

    let mut ph = BM_PageHandle { page_num: 0, data: String::new() };
    let rc = crate::buffer_mgr::pin_page(&mut bp, &mut ph, 0);
    if rc != RC::Ok { return rc; }

    let rec_size = get_record_size(schema);
    let mut header = vec![0u8; ps];
    let mut pos = 0usize;
    write_i32_to(&mut header, &mut pos, 0);
    write_i32_to(&mut header, &mut pos, rec_size);
    write_i32_to(&mut header, &mut pos, 1);
    write_i32_to(&mut header, &mut pos, 0);
    write_i32_to(&mut header, &mut pos, -1);
    write_i32_to(&mut header, &mut pos, schema.num_attr);
    write_i32_to(&mut header, &mut pos, schema.key_size);

    for i in 0..schema.num_attr as usize {
        let name_bytes = schema.attr_names[i].as_bytes();
        let copy_len = name_bytes.len().min(MAX_ATTR_NAME_LEN);
        header[pos..pos+copy_len].copy_from_slice(&name_bytes[..copy_len]);
        pos += MAX_ATTR_NAME_LEN;
        let dt_val = match schema.data_types[i] {
            DataType::DtInt => 0i32, DataType::DtString => 1i32,
            DataType::DtFloat => 2i32, DataType::DtBool => 3i32,
        };
        write_i32_to(&mut header, &mut pos, dt_val);
        write_i32_to(&mut header, &mut pos, schema.type_length[i]);
    }
    for i in 0..schema.key_size as usize {
        write_i32_to(&mut header, &mut pos, schema.key_attrs[i]);
    }

    ph.data = unsafe { String::from_utf8_unchecked(header) };
    let _ = crate::buffer_mgr::mark_dirty(&mut bp, &mut ph);
    let _ = crate::buffer_mgr::unpin_page(&mut bp, &mut ph);
    let _ = crate::buffer_mgr::shutdown_buffer_pool(&mut bp);
    RC::Ok
}

pub fn open_table(rel: &mut RM_TableData, name: &str) -> RC {
    let mut bp = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: crate::buffer_mgr::ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    let rc = crate::buffer_mgr::init_buffer_pool(&mut bp, name, 3,
        crate::buffer_mgr::ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok { return rc; }

    let mut ph = BM_PageHandle { page_num: 0, data: String::new() };
    let rc = crate::buffer_mgr::pin_page(&mut bp, &mut ph, 0);
    if rc != RC::Ok { return rc; }

    let data = ph.data.as_bytes();
    let mut pos = 0usize;
    let total_tuples = read_i32_from(data, &mut pos);
    let rec_size = read_i32_from(data, &mut pos);
    let first_free_page = read_i32_from(data, &mut pos);
    let first_free_slot = read_i32_from(data, &mut pos);
    let first_data_page = read_i32_from(data, &mut pos);
    let num_attr = read_i32_from(data, &mut pos);
    let key_size = read_i32_from(data, &mut pos);

    let mut attr_names = Vec::new();
    let mut data_types = Vec::new();
    let mut type_length = Vec::new();
    for _ in 0..num_attr {
        let name_end = pos + MAX_ATTR_NAME_LEN;
        let name_slice = &data[pos..name_end];
        let nul = name_slice.iter().position(|&b| b == 0).unwrap_or(MAX_ATTR_NAME_LEN);
        attr_names.push(String::from_utf8_lossy(&name_slice[..nul]).to_string());
        pos = name_end;
        let dt_val = read_i32_from(data, &mut pos);
        data_types.push(match dt_val {
            0 => DataType::DtInt, 1 => DataType::DtString,
            2 => DataType::DtFloat, 3 => DataType::DtBool, _ => DataType::DtInt,
        });
        type_length.push(read_i32_from(data, &mut pos));
    }
    let mut key_attrs = Vec::new();
    for _ in 0..key_size { key_attrs.push(read_i32_from(data, &mut pos)); }

    let _ = crate::buffer_mgr::unpin_page(&mut bp, &mut ph);

    let schema = Schema { num_attr, attr_names, data_types, type_length, key_attrs, key_size };
    let tm = TableManager {
        total_tuples, rec_size, first_free_page_num: first_free_page,
        first_free_slot_num: first_free_slot, first_data_page_num: first_data_page,
        buffer_pool: Some(bp), page_handler: Some(ph),
    };
    rel.name = name.to_string();
    rel.schema = schema;
    rel.mgmt_data = Some(Box::new(TableManagerCell(RefCell::new(tm))));
    RC::Ok
}

pub fn close_table(rel: &mut RM_TableData) -> RC {
    let cell = get_cell(rel);
    let mut tm = cell.0.borrow_mut();
    let ps = crate::dberror::PAGE_SIZE as usize;
    let result = with_bp_ph(&mut tm, |tm, bp, ph| {
        let rc = crate::buffer_mgr::pin_page(bp, ph, 0);
        if rc == RC::Ok {
            let mut header = vec![0u8; ps];
            let mut pos = 0usize;
            write_i32_to(&mut header, &mut pos, tm.total_tuples);
            write_i32_to(&mut header, &mut pos, tm.rec_size);
            write_i32_to(&mut header, &mut pos, tm.first_free_page_num);
            write_i32_to(&mut header, &mut pos, tm.first_free_slot_num);
            write_i32_to(&mut header, &mut pos, tm.first_data_page_num);
            let existing = ph.data.as_bytes();
            if existing.len() > pos {
                let copy_len = (existing.len() - pos).min(ps - pos);
                header[pos..pos+copy_len].copy_from_slice(&existing[pos..pos+copy_len]);
            }
            ph.data = unsafe { String::from_utf8_unchecked(header) };
            let _ = crate::buffer_mgr::mark_dirty(bp, ph);
            let _ = crate::buffer_mgr::unpin_page(bp, ph);
        }
        crate::buffer_mgr::shutdown_buffer_pool(bp)
    });
    result
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let cell = get_cell(rel);
    let mut tm = cell.0.borrow_mut();
    let rec_size = tm.rec_size;
    let slots_per_page = (crate::dberror::PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);
    let first_free_page = tm.first_free_page_num;
    let first_free_slot = tm.first_free_slot_num;

    with_bp_ph(&mut tm, |tm, bp, ph| {
        let rc = crate::buffer_mgr::pin_page(bp, ph, first_free_page);
        if rc != RC::Ok { return RC::Error; }

        let data = unsafe { ph.data.as_bytes_mut() };
        let mut hdr = page_header_from_bytes(data);

        if hdr.page_identifier != 'Y' {
            hdr.page_identifier = 'Y';
            hdr.total_tuples = 0;
            hdr.free_slot_cnt = slots_per_page as i32 - 1;
            hdr.next_free_slot_ind = 1;
            hdr.prev_free_page_index = -1;
            hdr.next_free_page_index = ph.page_num + 1;
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

        let hdr_bytes = page_header_to_bytes(&hdr);
        data[..PAGE_HEADER_SIZE].copy_from_slice(&hdr_bytes);

        let slot_pos = PAGE_HEADER_SIZE + (first_free_slot as usize) * (rec_size as usize + 2);
        data[slot_pos] = b'Y';
        let rec_bytes = record.data.as_bytes();
        let copy_len = rec_bytes.len().min(rec_size as usize);
        data[slot_pos + 1..slot_pos + 1 + copy_len].copy_from_slice(&rec_bytes[..copy_len]);
        data[slot_pos + rec_size as usize + 1] = b'|';

        if hdr.free_slot_cnt == 0 {
            tm.first_free_page_num += 1;
            tm.first_free_slot_num = 0;
        } else {
            tm.first_free_slot_num += 1;
        }
        tm.total_tuples += 1;

        let _ = crate::buffer_mgr::mark_dirty(bp, ph);
        let _ = crate::buffer_mgr::unpin_page(bp, ph);
        RC::Ok
    })
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    let cell = get_cell(rel);
    let mut tm = cell.0.borrow_mut();
    let rec_size = tm.rec_size;
    let slots_per_page = (crate::dberror::PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);
    if id.slot >= slots_per_page as i32 { return RC::RecordNotFound; }

    with_bp_ph(&mut tm, |_tm, bp, ph| {
        let rc = crate::buffer_mgr::pin_page(bp, ph, id.page);
        if rc != RC::Ok { return RC::Error; }

        let data = ph.data.as_bytes();
        let slot_pos = PAGE_HEADER_SIZE + (id.slot as usize) * (rec_size as usize + 2);
        if slot_pos >= data.len() || data[slot_pos] != b'Y' {
            let _ = crate::buffer_mgr::unpin_page(bp, ph);
            return RC::RecordNotFound;
        }

        let rec_start = slot_pos + 1;
        let rec_end = rec_start + rec_size as usize;
        let rec_data = if rec_end <= data.len() {
            data[rec_start..rec_end].to_vec()
        } else {
            let mut v = vec![0u8; rec_size as usize];
            let avail = data.len().saturating_sub(rec_start);
            v[..avail].copy_from_slice(&data[rec_start..rec_start+avail]);
            v
        };
        record.data = unsafe { String::from_utf8_unchecked(rec_data) };
        record.id = RID { page: id.page, slot: id.slot };

        let _ = crate::buffer_mgr::unpin_page(bp, ph);
        RC::Ok
    })
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let cell = get_cell(rel);
    let mut tm = cell.0.borrow_mut();
    let rec_size = tm.rec_size;
    let slots_per_page = (crate::dberror::PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);
    if record.id.slot >= slots_per_page as i32 { return RC::RecordNotFound; }

    with_bp_ph(&mut tm, |_tm, bp, ph| {
        let rc = crate::buffer_mgr::pin_page(bp, ph, record.id.page);
        if rc != RC::Ok { return RC::Error; }

        let data = unsafe { ph.data.as_bytes_mut() };
        let slot_pos = PAGE_HEADER_SIZE + (record.id.slot as usize) * (rec_size as usize + 2);
        if data[slot_pos] != b'Y' {
            let _ = crate::buffer_mgr::unpin_page(bp, ph);
            return RC::RecordNotFound;
        }

        let rec_bytes = record.data.as_bytes();
        let copy_len = rec_bytes.len().min(rec_size as usize);
        data[slot_pos + 1..slot_pos + 1 + copy_len].copy_from_slice(&rec_bytes[..copy_len]);

        let _ = crate::buffer_mgr::mark_dirty(bp, ph);
        let _ = crate::buffer_mgr::unpin_page(bp, ph);
        RC::Ok
    })
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let cell = get_cell(rel);
    let mut tm = cell.0.borrow_mut();
    let rec_size = tm.rec_size;
    let slots_per_page = (crate::dberror::PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);
    if id.slot >= slots_per_page as i32 { return RC::RecordNotFound; }

    with_bp_ph(&mut tm, |tm, bp, ph| {
        let rc = crate::buffer_mgr::pin_page(bp, ph, id.page);
        if rc != RC::Ok { return rc; }

        let data = unsafe { ph.data.as_bytes_mut() };
        let slot_pos = PAGE_HEADER_SIZE + (id.slot as usize) * (rec_size as usize + 2);
        if data[slot_pos] != b'Y' {
            let _ = crate::buffer_mgr::unpin_page(bp, ph);
            return RC::RecordNotFound;
        }

        data[slot_pos] = b'N';
        let mut hdr = page_header_from_bytes(data);
        hdr.total_tuples = if hdr.total_tuples > 0 { hdr.total_tuples - 1 } else { 0 };
        hdr.free_slot_cnt += 1;
        let hdr_bytes = page_header_to_bytes(&hdr);
        data[..PAGE_HEADER_SIZE].copy_from_slice(&hdr_bytes);

        tm.total_tuples = if tm.total_tuples > 0 { tm.total_tuples - 1 } else { 0 };

        let _ = crate::buffer_mgr::mark_dirty(bp, ph);
        let _ = crate::buffer_mgr::unpin_page(bp, ph);
        RC::Ok
    })
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let tm = get_cell(rel).0.borrow();
    let sm = ScanManager {
        total_entries: tm.total_tuples,
        current_page_num: tm.first_data_page_num,
        current_slot_num: -1,
        scan_index: 0,
        condition_expression: Some(cond.clone()),
        scan_page_handle_ptr: None,
    };
    drop(tm);
    scan.mgmt_data = Some(Box::new(sm));
    RC::Ok
}

pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    let sm = scan.mgmt_data.as_mut().unwrap().downcast_mut::<ScanManager>().unwrap();
    let rel = &scan.rel;

    let rec_size = get_cell(rel).0.borrow().rec_size;
    let slots_per_page = (crate::dberror::PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);

    if sm.scan_index >= sm.total_entries {
        return RC::RmNoMoreTuples;
    }

    loop {
        sm.current_slot_num += 1;
        if sm.current_slot_num >= slots_per_page as i32 {
            sm.current_page_num += 1;
            sm.current_slot_num = 0;
        }

        let rid = RID { page: sm.current_page_num, slot: sm.current_slot_num };
        let rc = get_record(rel, &rid, record);
        if rc == RC::Ok {
            sm.scan_index += 1;
            if let Some(ref cond) = sm.condition_expression {
                let mut eval_result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
                crate::expr::eval_expr(record, &rel.schema, cond, &mut eval_result);
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
    RC::Ok
}
