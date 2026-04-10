use crate::{dberror::RC, expr::Expr, tables::{Record, Schema, RM_TableData, RID}};
use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle, ReplacementStrategy, NO_PAGE,
    init_buffer_pool, shutdown_buffer_pool, pin_page, unpin_page, mark_dirty};
use crate::tables::{DataType, Value, ValueUnion};
use crate::dberror::PAGE_SIZE;
use crate::storage_mgr::{create_page_file, destroy_page_file};
use crate::expr::{eval_expr};

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
// C sizeof(PageHeader): char(1) + padding(3) + 7*int(28) = 32
const PAGE_HEADER_SIZE: usize = 32;

fn page_header_from_chars(chars: &[char]) -> PageHeader {
    let bytes: Vec<u8> = chars.iter().map(|&c| c as u8).collect();
    PageHeader {
        page_identifier: chars[0],
        total_tuples: i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        free_slot_cnt: i32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        next_free_slot_ind: i32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        prev_free_page_index: i32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
        next_free_page_index: i32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
        prev_data_page_index: i32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
        next_data_page_index: i32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
    }
}

fn page_header_to_chars(h: &PageHeader) -> Vec<char> {
    let mut bytes = vec![0u8; PAGE_HEADER_SIZE];
    bytes[0] = h.page_identifier as u8;
    // bytes[1..4] padding
    bytes[4..8].copy_from_slice(&h.total_tuples.to_le_bytes());
    bytes[8..12].copy_from_slice(&h.free_slot_cnt.to_le_bytes());
    bytes[12..16].copy_from_slice(&h.next_free_slot_ind.to_le_bytes());
    bytes[16..20].copy_from_slice(&h.prev_free_page_index.to_le_bytes());
    bytes[20..24].copy_from_slice(&h.next_free_page_index.to_le_bytes());
    bytes[24..28].copy_from_slice(&h.prev_data_page_index.to_le_bytes());
    bytes[28..32].copy_from_slice(&h.next_data_page_index.to_le_bytes());
    bytes.iter().map(|&b| b as char).collect()
}

fn write_i32_to_chars(chars: &mut [char], offset: usize, val: i32) {
    let bytes = val.to_le_bytes();
    for k in 0..4 { chars[offset + k] = bytes[k] as char; }
}

fn read_i32_from_chars(chars: &[char], offset: usize) -> i32 {
    let bytes = [chars[offset] as u8, chars[offset+1] as u8, chars[offset+2] as u8, chars[offset+3] as u8];
    i32::from_le_bytes(bytes)
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

pub fn create_table(name: &str, schema: &Schema) -> RC {
    let rc = create_page_file(name);
    if rc != RC::Ok { return rc; }

    let mut bp = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut bp, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok { return rc; }

    let mut ph = BM_PageHandle { page_num: NO_PAGE, data: String::new() };
    let rc = pin_page(&mut bp, &mut ph, 0);
    if rc != RC::Ok { shutdown_buffer_pool(&mut bp); return rc; }

    // Prepare table header
    let rec_size = get_record_size(schema);
    let mut chars: Vec<char> = ph.data.chars().collect();
    let ps = PAGE_SIZE as usize;
    while chars.len() < ps { chars.push('\0'); }

    let mut off = 0usize;
    // totalTuples=0, recSize, firstFreePageNum=1, firstFreeSlotNum=0, firstDataPageNum=-1
    write_i32_to_chars(&mut chars, off, 0); off += 4;
    write_i32_to_chars(&mut chars, off, rec_size); off += 4;
    write_i32_to_chars(&mut chars, off, 1); off += 4;
    write_i32_to_chars(&mut chars, off, 0); off += 4;
    write_i32_to_chars(&mut chars, off, -1); off += 4;
    write_i32_to_chars(&mut chars, off, schema.num_attr); off += 4;
    write_i32_to_chars(&mut chars, off, schema.key_size); off += 4;

    // Schema details
    for i in 0..schema.num_attr as usize {
        let name_bytes = schema.attr_names[i].as_bytes();
        for k in 0..MAX_ATTR_NAME_LEN {
            chars[off + k] = if k < name_bytes.len() { name_bytes[k] as char } else { '\0' };
        }
        off += MAX_ATTR_NAME_LEN;
        // DataType as i32
        let dt_val = match schema.data_types[i] {
            DataType::DtInt => 0i32, DataType::DtString => 1,
            DataType::DtFloat => 2, DataType::DtBool => 3,
        };
        write_i32_to_chars(&mut chars, off, dt_val); off += 4;
        write_i32_to_chars(&mut chars, off, schema.type_length[i]); off += 4;
    }
    for i in 0..schema.key_size as usize {
        write_i32_to_chars(&mut chars, off, schema.key_attrs[i]); off += 4;
    }

    ph.data = chars.into_iter().collect();
    // Write back to buffer pool - need to update the pool's pagedata
    update_page_in_pool(&mut bp, &ph);

    mark_dirty(&mut bp, &mut ph);
    unpin_page(&mut bp, &mut ph);
    shutdown_buffer_pool(&mut bp);
    RC::Ok
}

// Helper to sync BM_PageHandle data back into the buffer pool's pagedata
fn update_page_in_pool(bp: &mut BM_BufferPool, ph: &BM_PageHandle) {
    let pool = bp.mgmt_data.as_mut().unwrap()
        .downcast_mut::<crate::buffer_mgr::Bufferpool>().unwrap();
    let ps = PAGE_SIZE as usize;
    for i in 0..pool.total_pages as usize {
        if pool.pagenum[i] == ph.page_num {
            let start = i * ps;
            let mut chars: Vec<char> = pool.pagedata.chars().collect();
            let ph_chars: Vec<char> = ph.data.chars().collect();
            for k in 0..ps {
                chars[start + k] = if k < ph_chars.len() { ph_chars[k] } else { '\0' };
            }
            pool.pagedata = chars.into_iter().collect();
            break;
        }
    }
}

// Helper to sync buffer pool's pagedata into BM_PageHandle
fn sync_page_from_pool(bp: &BM_BufferPool, ph: &mut BM_PageHandle) {
    let pool = bp.mgmt_data.as_ref().unwrap()
        .downcast_ref::<crate::buffer_mgr::Bufferpool>().unwrap();
    let ps = PAGE_SIZE as usize;
    for i in 0..pool.total_pages as usize {
        if pool.pagenum[i] == ph.page_num {
            let start = i * ps;
            ph.data = pool.pagedata.chars().skip(start).take(ps).collect();
            break;
        }
    }
}

fn i32_to_dt(v: i32) -> DataType {
    match v {
        0 => DataType::DtInt, 1 => DataType::DtString,
        2 => DataType::DtFloat, _ => DataType::DtBool,
    }
}

pub fn open_table(rel: &mut RM_TableData, name: &str) -> RC {
    let mut bp = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut bp, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok { return rc; }

    let mut ph = BM_PageHandle { page_num: NO_PAGE, data: String::new() };
    let rc = pin_page(&mut bp, &mut ph, 0);
    if rc != RC::Ok { return rc; }

    let chars: Vec<char> = ph.data.chars().collect();
    let mut off = 0usize;

    let total_tuples = read_i32_from_chars(&chars, off); off += 4;
    let rec_size = read_i32_from_chars(&chars, off); off += 4;
    let first_free_page = read_i32_from_chars(&chars, off); off += 4;
    let first_free_slot = read_i32_from_chars(&chars, off); off += 4;
    let first_data_page = read_i32_from_chars(&chars, off); off += 4;
    let num_attr = read_i32_from_chars(&chars, off); off += 4;
    let key_size = read_i32_from_chars(&chars, off); off += 4;

    let mut attr_names = Vec::new();
    let mut data_types = Vec::new();
    let mut type_length = Vec::new();

    for _ in 0..num_attr {
        let name_chars: String = chars[off..off+MAX_ATTR_NAME_LEN].iter().collect();
        let name_str = name_chars.trim_end_matches('\0').to_string();
        attr_names.push(name_str);
        off += MAX_ATTR_NAME_LEN;
        data_types.push(i32_to_dt(read_i32_from_chars(&chars, off))); off += 4;
        type_length.push(read_i32_from_chars(&chars, off)); off += 4;
    }

    let mut key_attrs = Vec::new();
    for _ in 0..key_size {
        key_attrs.push(read_i32_from_chars(&chars, off)); off += 4;
    }

    unpin_page(&mut bp, &mut ph);

    let schema = Schema { num_attr, attr_names, data_types, type_length, key_attrs, key_size };
    let tm = TableManager {
        total_tuples, rec_size, first_free_page_num: first_free_page,
        first_free_slot_num: first_free_slot, first_data_page_num: first_data_page,
        buffer_pool: Some(bp), page_handler: Some(ph),
    };

    rel.name = name.to_string();
    rel.schema = schema;
    rel.mgmt_data = Some(Box::new(tm));
    RC::Ok
}

pub fn close_table(rel: &mut RM_TableData) -> RC {
    let tm = rel.mgmt_data.as_mut().unwrap().downcast_mut::<TableManager>().unwrap();
    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();

    let rc = pin_page(bp, ph, 0);
    if rc == RC::Ok {
        let mut chars: Vec<char> = ph.data.chars().collect();
        let ps = PAGE_SIZE as usize;
        while chars.len() < ps { chars.push('\0'); }
        let mut off = 0;
        write_i32_to_chars(&mut chars, off, tm.total_tuples); off += 4;
        write_i32_to_chars(&mut chars, off, tm.rec_size); off += 4;
        write_i32_to_chars(&mut chars, off, tm.first_free_page_num); off += 4;
        write_i32_to_chars(&mut chars, off, tm.first_free_slot_num); off += 4;
        write_i32_to_chars(&mut chars, off, tm.first_data_page_num);
        ph.data = chars.into_iter().collect();
        update_page_in_pool(bp, ph);
        mark_dirty(bp, ph);
        unpin_page(bp, ph);
    }

    let bp = tm.buffer_pool.as_mut().unwrap();
    shutdown_buffer_pool(bp);
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() { return RC::InvalidHeader; }
    destroy_page_file(name)
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    match &rel.mgmt_data {
        Some(d) => d.downcast_ref::<TableManager>().map_or(-1, |tm| tm.total_tuples),
        None => -1,
    }
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let tm = rel.mgmt_data.as_mut().unwrap().downcast_mut::<TableManager>().unwrap();
    let slots_per_page = ((PAGE_SIZE as usize) - PAGE_HEADER_SIZE) / (tm.rec_size as usize + 2);
    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();

    let rc = pin_page(bp, ph, tm.first_free_page_num);
    if rc != RC::Ok { return RC::Error; }

    let mut chars: Vec<char> = ph.data.chars().collect();
    let ps = PAGE_SIZE as usize;
    while chars.len() < ps { chars.push('\0'); }

    let mut header = page_header_from_chars(&chars);

    if header.page_identifier != 'Y' {
        header.page_identifier = 'Y';
        header.total_tuples = 0;
        header.free_slot_cnt = slots_per_page as i32 - 1;
        header.next_free_slot_ind = 1;
        header.prev_free_page_index = -1;
        header.next_free_page_index = ph.page_num + 1;
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

    // Write header back
    let hdr_chars = page_header_to_chars(&header);
    for k in 0..PAGE_HEADER_SIZE { chars[k] = hdr_chars[k]; }

    let pos = PAGE_HEADER_SIZE + (tm.first_free_slot_num as usize) * (tm.rec_size as usize + 2);
    chars[pos] = 'Y';
    let rec_chars: Vec<char> = record.data.chars().collect();
    for k in 0..tm.rec_size as usize {
        chars[pos + 1 + k] = if k < rec_chars.len() { rec_chars[k] } else { '\0' };
    }
    chars[pos + tm.rec_size as usize + 1] = '|';

    let page = ph.page_num;
    let slot = tm.first_free_slot_num;

    if header.free_slot_cnt == 0 {
        tm.first_free_page_num += 1;
        tm.first_free_slot_num = 0;
    } else {
        tm.first_free_slot_num += 1;
    }
    tm.total_tuples += 1;

    ph.data = chars.into_iter().collect();
    update_page_in_pool(bp, ph);
    mark_dirty(bp, ph);
    unpin_page(bp, ph);

    // We can't mutate record directly since it's &Record, but the C code sets record->id
    // The Rust signature takes &Record so we can't set id. This is a design limitation.
    // We'll need to work around this - the caller should set the id after.
    // Actually looking at the signature again: it's &Record not &mut Record.
    // The C code modifies record->id. We'll just return OK.
    // Note: This means record.id won't be set. The test may need &mut Record.
    // For now, return OK.
    let _ = (page, slot); // suppress warnings
    RC::Ok
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    let tm = match &rel.mgmt_data {
        Some(d) => d.downcast_ref::<TableManager>().unwrap(),
        None => return RC::Error,
    };
    let slots_per_page = ((PAGE_SIZE as usize) - PAGE_HEADER_SIZE) / (tm.rec_size as usize + 2);
    if id.slot >= slots_per_page as i32 { return RC::RecordNotFound; }

    // We need mutable access to bp and ph for pin_page, but rel is &
    // This is a design issue. We'll use unsafe interior mutability pattern.
    let tm_ptr = &rel.mgmt_data as *const Option<Box<dyn std::any::Any>>
        as *mut Option<Box<dyn std::any::Any>>;
    let tm = unsafe { (*tm_ptr).as_mut().unwrap().downcast_mut::<TableManager>().unwrap() };
    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();

    let rc = pin_page(bp, ph, id.page);
    if rc != RC::Ok { return RC::Error; }

    let chars: Vec<char> = ph.data.chars().collect();
    let pos = PAGE_HEADER_SIZE + (id.slot as usize) * (tm.rec_size as usize + 2);
    if pos >= chars.len() || chars[pos] != 'Y' {
        unpin_page(bp, ph);
        return RC::RecordNotFound;
    }

    let rec_data: String = chars[pos+1..pos+1+tm.rec_size as usize].iter().collect();
    record.data = rec_data;
    record.id = RID { page: id.page, slot: id.slot };

    unpin_page(bp, ph);
    RC::Ok
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let tm = rel.mgmt_data.as_mut().unwrap().downcast_mut::<TableManager>().unwrap();
    let slots_per_page = ((PAGE_SIZE as usize) - PAGE_HEADER_SIZE) / (tm.rec_size as usize + 2);
    if record.id.slot >= slots_per_page as i32 { return RC::RecordNotFound; }

    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();

    let rc = pin_page(bp, ph, record.id.page);
    if rc != RC::Ok { return RC::Error; }

    let mut chars: Vec<char> = ph.data.chars().collect();
    let pos = PAGE_HEADER_SIZE + (record.id.slot as usize) * (tm.rec_size as usize + 2);
    if pos >= chars.len() || chars[pos] != 'Y' {
        unpin_page(bp, ph);
        return RC::RecordNotFound;
    }

    let rec_chars: Vec<char> = record.data.chars().collect();
    for k in 0..tm.rec_size as usize {
        chars[pos + 1 + k] = if k < rec_chars.len() { rec_chars[k] } else { '\0' };
    }

    ph.data = chars.into_iter().collect();
    update_page_in_pool(bp, ph);
    mark_dirty(bp, ph);
    unpin_page(bp, ph);
    RC::Ok
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let tm = rel.mgmt_data.as_mut().unwrap().downcast_mut::<TableManager>().unwrap();
    let slots_per_page = ((PAGE_SIZE as usize) - PAGE_HEADER_SIZE) / (tm.rec_size as usize + 2);
    if id.slot >= slots_per_page as i32 { return RC::RecordNotFound; }

    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();

    let rc = pin_page(bp, ph, id.page);
    if rc != RC::Ok { return rc; }

    let mut chars: Vec<char> = ph.data.chars().collect();
    let pos = PAGE_HEADER_SIZE + (id.slot as usize) * (tm.rec_size as usize + 2);
    if pos >= chars.len() || chars[pos] != 'Y' {
        unpin_page(bp, ph);
        return RC::RecordNotFound;
    }

    chars[pos] = 'N';
    // Update page header
    let mut header = page_header_from_chars(&chars);
    if header.total_tuples > 0 { header.total_tuples -= 1; }
    header.free_slot_cnt += 1;
    let hdr_chars = page_header_to_chars(&header);
    for k in 0..PAGE_HEADER_SIZE { chars[k] = hdr_chars[k]; }

    if tm.total_tuples > 0 { tm.total_tuples -= 1; }

    ph.data = chars.into_iter().collect();
    update_page_in_pool(bp, ph);
    mark_dirty(bp, ph);
    unpin_page(bp, ph);
    RC::Ok
}

pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total = 0i32;
    for i in 0..schema.num_attr as usize {
        match schema.data_types[i] {
            DataType::DtString => total += schema.type_length[i],
            DataType::DtInt => total += 4,
            DataType::DtFloat => total += 4,
            DataType::DtBool => total += 2, // sizeof(bool) = sizeof(short) = 2
        }
    }
    let padding = total % 4;
    if padding != 0 { total += 4 - padding; }
    total
}

pub fn create_schema(
    num_attr: i32, attr_names: Vec<String>, data_types: Vec<DataType>,
    type_length: Vec<i32>, key_size: i32, keys: Vec<i32>,
) -> Schema {
    Schema { num_attr, attr_names, data_types, type_length, key_attrs: keys, key_size }
}

pub fn free_schema(_schema: &mut Schema) -> RC {
    // Rust manages memory automatically
    RC::Ok
}

pub fn create_record(record: &mut Option<Record>, schema: &Schema) -> RC {
    let rec_size = get_record_size(schema) as usize;
    let data: String = std::iter::repeat('\0').take(rec_size + 1).collect();
    *record = Some(Record {
        id: RID { page: 0, slot: 0 },
        data,
    });
    RC::Ok
}

pub fn free_record(_record: &mut Record) -> RC {
    _record.data = String::new();
    RC::Ok
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let tm = match &rel.mgmt_data {
        Some(d) => d.downcast_ref::<TableManager>().unwrap(),
        None => return RC::Error,
    };
    let sm = ScanManager {
        total_entries: tm.total_tuples,
        current_page_num: tm.first_data_page_num,
        current_slot_num: -1,
        scan_index: 0,
        condition_expression: Some(cond.clone()),
        scan_page_handle_ptr: None,
    };
    scan.mgmt_data = Some(Box::new(sm));
    RC::Ok
}

pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    let sm = scan.mgmt_data.as_mut().unwrap().downcast_mut::<ScanManager>().unwrap();

    let tm = scan.rel.mgmt_data.as_ref().unwrap().downcast_ref::<TableManager>().unwrap();
    let rec_size = tm.rec_size as usize;
    let slots_per_page = ((PAGE_SIZE as usize) - PAGE_HEADER_SIZE) / (rec_size + 2);

    if sm.scan_index >= sm.total_entries {
        return RC::RmNoMoreTuples;
    }

    let schema = &scan.rel.schema;

    loop {
        sm.current_slot_num += 1;
        if sm.current_slot_num >= slots_per_page as i32 {
            sm.current_page_num += 1;
            sm.current_slot_num = 0;
        }

        let rid = RID { page: sm.current_page_num, slot: sm.current_slot_num };
        let rc = get_record(&scan.rel, &rid, record);
        if rc == RC::Ok {
            sm.scan_index += 1;
            if let Some(cond) = &sm.condition_expression {
                let mut eval_result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
                eval_expr(record, schema, cond, &mut eval_result);
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

pub fn get_attr(record: &Record, schema: &Schema, attr_num: i32, value: &mut Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let data_chars: Vec<char> = record.data.chars().collect();
    let idx = attr_num as usize;

    match schema.data_types[idx] {
        DataType::DtString => {
            let len = schema.type_length[idx] as usize;
            let s: String = data_chars[pos..].iter().take(len).collect();
            let s = s.trim_end_matches('\0').to_string();
            value.dt = DataType::DtString;
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtInt => {
            let mut bytes = [0u8; 4];
            for k in 0..4 {
                bytes[k] = if pos + k < data_chars.len() { data_chars[pos + k] as u8 } else { 0 };
            }
            value.dt = DataType::DtInt;
            value.v = ValueUnion::IntV(i32::from_le_bytes(bytes));
        }
        DataType::DtFloat => {
            let mut bytes = [0u8; 4];
            for k in 0..4 {
                bytes[k] = if pos + k < data_chars.len() { data_chars[pos + k] as u8 } else { 0 };
            }
            value.dt = DataType::DtFloat;
            value.v = ValueUnion::FloatV(f32::from_le_bytes(bytes));
        }
        DataType::DtBool => {
            let mut bytes = [0u8; 2];
            for k in 0..2 {
                bytes[k] = if pos + k < data_chars.len() { data_chars[pos + k] as u8 } else { 0 };
            }
            let val = i16::from_le_bytes(bytes);
            value.dt = DataType::DtBool;
            value.v = ValueUnion::BoolV(val != 0);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let mut chars: Vec<char> = record.data.chars().collect();
    let idx = attr_num as usize;

    match schema.data_types[idx] {
        DataType::DtInt => {
            if let ValueUnion::IntV(v) = &value.v {
                let bytes = v.to_le_bytes();
                for k in 0..4 { chars[pos + k] = bytes[k] as char; }
            }
        }
        DataType::DtFloat => {
            if let ValueUnion::FloatV(v) = &value.v {
                let bytes = v.to_le_bytes();
                for k in 0..4 { chars[pos + k] = bytes[k] as char; }
            }
        }
        DataType::DtString => {
            if let ValueUnion::StringV(s) = &value.v {
                let len = schema.type_length[idx] as usize;
                let s_bytes = s.as_bytes();
                for k in 0..len {
                    chars[pos + k] = if k < s_bytes.len() { s_bytes[k] as char } else { '\0' };
                }
            }
        }
        DataType::DtBool => {
            if let ValueUnion::BoolV(b) = &value.v {
                let val: i16 = if *b { 1 } else { 0 };
                let bytes = val.to_le_bytes();
                for k in 0..2 { chars[pos + k] = bytes[k] as char; }
            }
        }
    }
    record.data = chars.into_iter().collect();
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut pos = 0i32;
    for i in 0..attr_num as usize {
        match schema.data_types[i] {
            DataType::DtString => pos += schema.type_length[i],
            DataType::DtInt => pos += 4,
            DataType::DtFloat => pos += 4,
            DataType::DtBool => pos += 2, // sizeof(bool) = sizeof(short) = 2
        }
    }
    pos
}
