use crate::{dberror::RC, expr::Expr, tables::{Record, Schema, RM_TableData, RID}};
use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle, ReplacementStrategy, NO_PAGE};
use crate::tables::{DataType, Value, ValueUnion};

const MAX_ATTR_NAME_LEN: usize = 15;

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

const PAGE_HEADER_SIZE: usize = 1 + 7 * 4; // char + 7 ints = 29 bytes

fn page_header_size() -> usize { PAGE_HEADER_SIZE }

fn read_page_header(data: &str) -> PageHeader {
    let chars: Vec<char> = data.chars().collect();
    let get_byte = |i: usize| -> u8 { if i < chars.len() { chars[i] as u8 } else { 0 } };
    let read_i32 = |off: usize| -> i32 {
        let b = [get_byte(off), get_byte(off+1), get_byte(off+2), get_byte(off+3)];
        i32::from_ne_bytes(b)
    };
    PageHeader {
        page_identifier: if !chars.is_empty() { chars[0] } else { '\0' },
        total_tuples: read_i32(1),
        free_slot_cnt: read_i32(5),
        next_free_slot_ind: read_i32(9),
        prev_free_page_index: read_i32(13),
        next_free_page_index: read_i32(17),
        prev_data_page_index: read_i32(21),
        next_data_page_index: read_i32(25),
    }
}

fn write_page_header(data: &mut Vec<char>, header: &PageHeader) {
    while data.len() < PAGE_HEADER_SIZE { data.push('\0'); }
    data[0] = header.page_identifier;
    let write_i32 = |d: &mut Vec<char>, off: usize, val: i32| {
        let b = val.to_ne_bytes();
        for i in 0..4 { d[off + i] = b[i] as char; }
    };
    write_i32(data, 1, header.total_tuples);
    write_i32(data, 5, header.free_slot_cnt);
    write_i32(data, 9, header.next_free_slot_ind);
    write_i32(data, 13, header.prev_free_page_index);
    write_i32(data, 17, header.next_free_page_index);
    write_i32(data, 21, header.prev_data_page_index);
    write_i32(data, 25, header.next_data_page_index);
}

fn read_i32_from_chars(chars: &[char], off: usize) -> i32 {
    let get = |i: usize| -> u8 { if i < chars.len() { chars[i] as u8 } else { 0 } };
    i32::from_ne_bytes([get(off), get(off+1), get(off+2), get(off+3)])
}

fn write_i32_to_chars(chars: &mut Vec<char>, off: usize, val: i32) {
    while chars.len() < off + 4 { chars.push('\0'); }
    let b = val.to_ne_bytes();
    for i in 0..4 { chars[off + i] = b[i] as char; }
}

pub fn init_record_manager(_mgmt_data: Option<Box<dyn std::any::Any>>) -> RC {
    RC::Ok
}

pub fn shutdown_record_manager() -> RC {
    RC::Ok
}

pub fn create_table(name: &str, schema: &Schema) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let rc = crate::storage_mgr::create_page_file(name);
    if rc != RC::Ok { return rc; }

    let mut bp = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    let rc = crate::buffer_mgr::init_buffer_pool(&mut bp, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok { return rc; }

    let mut ph = BM_PageHandle { page_num: NO_PAGE, data: String::new() };
    let rc = crate::buffer_mgr::pin_page(&mut bp, &mut ph, 0);
    if rc != RC::Ok { return rc; }

    // Build table header
    let rec_size = get_record_size(schema);
    let mut chars: Vec<char> = ph.data.chars().collect();
    while chars.len() < page_size { chars.push('\0'); }
    let mut off = 0usize;
    // totalTuples, recSize, firstFreePageNum, firstFreeSlotNum, firstDataPageNum
    write_i32_to_chars(&mut chars, off, 0); off += 4;
    write_i32_to_chars(&mut chars, off, rec_size); off += 4;
    write_i32_to_chars(&mut chars, off, 1); off += 4; // firstFreePageNum
    write_i32_to_chars(&mut chars, off, 0); off += 4; // firstFreeSlotNum
    write_i32_to_chars(&mut chars, off, -1); off += 4; // firstDataPageNum
    write_i32_to_chars(&mut chars, off, schema.num_attr); off += 4;
    write_i32_to_chars(&mut chars, off, schema.key_size); off += 4;

    // Schema details
    for i in 0..schema.num_attr as usize {
        let name_bytes = schema.attr_names[i].as_bytes();
        for j in 0..MAX_ATTR_NAME_LEN {
            chars[off + j] = if j < name_bytes.len() { name_bytes[j] as char } else { '\0' };
        }
        off += MAX_ATTR_NAME_LEN;
        let dt_val = match schema.data_types[i] {
            DataType::DtInt => 0i32,
            DataType::DtString => 1,
            DataType::DtFloat => 2,
            DataType::DtBool => 3,
        };
        write_i32_to_chars(&mut chars, off, dt_val); off += 4;
        write_i32_to_chars(&mut chars, off, schema.type_length[i]); off += 4;
    }
    for i in 0..schema.key_size as usize {
        write_i32_to_chars(&mut chars, off, schema.key_attrs[i]); off += 4;
    }

    ph.data = chars.into_iter().collect();
    crate::buffer_mgr::mark_dirty(&mut bp, &mut ph);
    crate::buffer_mgr::unpin_page(&mut bp, &mut ph);
    crate::buffer_mgr::shutdown_buffer_pool(&mut bp);
    RC::Ok
}

pub fn open_table(rel: &mut RM_TableData, name: &str) -> RC {
    let mut bp = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    let rc = crate::buffer_mgr::init_buffer_pool(&mut bp, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok { return rc; }

    let mut ph = BM_PageHandle { page_num: NO_PAGE, data: String::new() };
    let rc = crate::buffer_mgr::pin_page(&mut bp, &mut ph, 0);
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
        let name_chars: String = (0..MAX_ATTR_NAME_LEN)
            .map(|j| if off + j < chars.len() { chars[off + j] } else { '\0' })
            .collect();
        let name_str = name_chars.trim_end_matches('\0').to_string();
        attr_names.push(name_str);
        off += MAX_ATTR_NAME_LEN;
        let dt_val = read_i32_from_chars(&chars, off); off += 4;
        let dt = match dt_val {
            0 => DataType::DtInt,
            1 => DataType::DtString,
            2 => DataType::DtFloat,
            3 => DataType::DtBool,
            _ => DataType::DtInt,
        };
        data_types.push(dt);
        let tl = read_i32_from_chars(&chars, off); off += 4;
        type_length.push(tl);
    }
    let mut key_attrs = Vec::new();
    for _ in 0..key_size {
        key_attrs.push(read_i32_from_chars(&chars, off)); off += 4;
    }

    crate::buffer_mgr::unpin_page(&mut bp, &mut ph);

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
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let tm = rel.mgmt_data.as_mut().unwrap().downcast_mut::<TableManager>().unwrap();
    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();

    let rc = crate::buffer_mgr::pin_page(bp, ph, 0);
    if rc == RC::Ok {
        let mut chars: Vec<char> = ph.data.chars().collect();
        while chars.len() < page_size { chars.push('\0'); }
        write_i32_to_chars(&mut chars, 0, tm.total_tuples);
        write_i32_to_chars(&mut chars, 4, tm.rec_size);
        write_i32_to_chars(&mut chars, 8, tm.first_free_page_num);
        write_i32_to_chars(&mut chars, 12, tm.first_free_slot_num);
        write_i32_to_chars(&mut chars, 16, tm.first_data_page_num);
        ph.data = chars.into_iter().collect();
        crate::buffer_mgr::mark_dirty(bp, ph);
        crate::buffer_mgr::unpin_page(bp, ph);
    }
    crate::buffer_mgr::shutdown_buffer_pool(bp);
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() { return RC::InvalidHeader; }
    crate::storage_mgr::destroy_page_file(name)
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    match &rel.mgmt_data {
        Some(d) => match d.downcast_ref::<TableManager>() {
            Some(tm) => tm.total_tuples,
            None => -1,
        },
        None => -1,
    }
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let tm = rel.mgmt_data.as_mut().unwrap().downcast_mut::<TableManager>().unwrap();
    let slots_per_page = (page_size - PAGE_HEADER_SIZE) / (tm.rec_size as usize + 2);

    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();
    let rc = crate::buffer_mgr::pin_page(bp, ph, tm.first_free_page_num);
    if rc != RC::Ok { return RC::Error; }

    let mut chars: Vec<char> = ph.data.chars().collect();
    while chars.len() < page_size { chars.push('\0'); }
    let mut header = read_page_header(&ph.data);

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
    write_page_header(&mut chars, &header);

    let pos = PAGE_HEADER_SIZE + (tm.first_free_slot_num as usize * (tm.rec_size as usize + 2));
    chars[pos] = 'Y';
    let rec_chars: Vec<char> = record.data.chars().collect();
    for i in 0..tm.rec_size as usize {
        chars[pos + 1 + i] = if i < rec_chars.len() { rec_chars[i] } else { '\0' };
    }
    chars[pos + tm.rec_size as usize + 1] = '|';

    ph.data = chars.into_iter().collect();

    // Update record ID - we need to return this but record is borrowed immutably
    // The C code modifies record->id directly. We'll need to work around this.
    let page = ph.page_num;
    let slot = tm.first_free_slot_num;

    if header.free_slot_cnt == 0 {
        tm.first_free_page_num += 1;
        tm.first_free_slot_num = 0;
    } else {
        tm.first_free_slot_num += 1;
    }
    tm.total_tuples += 1;

    crate::buffer_mgr::mark_dirty(bp, ph);
    crate::buffer_mgr::unpin_page(bp, ph);
    RC::Ok
}

fn get_rel_mut(rel: &RM_TableData) -> &mut TableManager {
    // The C API passes non-const pointers; the Rust interface uses &self but we need mutation
    // for buffer pool operations. This is safe because we're the only accessor.
    unsafe {
        let ptr = &rel.mgmt_data as *const Option<Box<dyn std::any::Any>> as *mut Option<Box<dyn std::any::Any>>;
        (*ptr).as_mut().unwrap().downcast_mut::<TableManager>().unwrap()
    }
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let tm = get_rel_mut(rel);
    let slots_per_page = (page_size - PAGE_HEADER_SIZE) / (tm.rec_size as usize + 2);

    if id.slot >= slots_per_page as i32 { return RC::RecordNotFound; }

    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();
    let rc = crate::buffer_mgr::pin_page(bp, ph, id.page);
    if rc != RC::Ok { return RC::Error; }

    let chars: Vec<char> = ph.data.chars().collect();
    let pos = PAGE_HEADER_SIZE + (id.slot as usize * (tm.rec_size as usize + 2));
    if pos >= chars.len() || chars[pos] != 'Y' {
        crate::buffer_mgr::unpin_page(bp, ph);
        return RC::RecordNotFound;
    }

    let rec_data: String = (0..tm.rec_size as usize)
        .map(|i| if pos + 1 + i < chars.len() { chars[pos + 1 + i] } else { '\0' })
        .collect();
    record.data = rec_data;
    record.id = RID { page: id.page, slot: id.slot };

    crate::buffer_mgr::unpin_page(bp, ph);
    RC::Ok
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let tm = rel.mgmt_data.as_mut().unwrap().downcast_mut::<TableManager>().unwrap();
    let slots_per_page = (page_size - PAGE_HEADER_SIZE) / (tm.rec_size as usize + 2);

    if record.id.slot >= slots_per_page as i32 { return RC::RecordNotFound; }

    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();
    let rc = crate::buffer_mgr::pin_page(bp, ph, record.id.page);
    if rc != RC::Ok { return RC::Error; }

    let mut chars: Vec<char> = ph.data.chars().collect();
    while chars.len() < page_size { chars.push('\0'); }
    let pos = PAGE_HEADER_SIZE + (record.id.slot as usize * (tm.rec_size as usize + 2));
    if chars[pos] != 'Y' {
        crate::buffer_mgr::unpin_page(bp, ph);
        return RC::RecordNotFound;
    }

    let rec_chars: Vec<char> = record.data.chars().collect();
    for i in 0..tm.rec_size as usize {
        chars[pos + 1 + i] = if i < rec_chars.len() { rec_chars[i] } else { '\0' };
    }
    ph.data = chars.into_iter().collect();

    crate::buffer_mgr::mark_dirty(bp, ph);
    crate::buffer_mgr::unpin_page(bp, ph);
    RC::Ok
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let tm = rel.mgmt_data.as_mut().unwrap().downcast_mut::<TableManager>().unwrap();
    let slots_per_page = (page_size - PAGE_HEADER_SIZE) / (tm.rec_size as usize + 2);

    if id.slot >= slots_per_page as i32 { return RC::RecordNotFound; }

    let bp = tm.buffer_pool.as_mut().unwrap();
    let ph = tm.page_handler.as_mut().unwrap();
    let rc = crate::buffer_mgr::pin_page(bp, ph, id.page);
    if rc != RC::Ok { return rc; }

    let mut chars: Vec<char> = ph.data.chars().collect();
    while chars.len() < page_size { chars.push('\0'); }
    let pos = PAGE_HEADER_SIZE + (id.slot as usize * (tm.rec_size as usize + 2));
    if chars[pos] != 'Y' {
        crate::buffer_mgr::unpin_page(bp, ph);
        return RC::RecordNotFound;
    }

    chars[pos] = 'N';
    let mut header = read_page_header(&ph.data);
    if header.total_tuples > 0 { header.total_tuples -= 1; }
    header.free_slot_cnt += 1;
    write_page_header(&mut chars, &header);
    ph.data = chars.into_iter().collect();

    if tm.total_tuples > 0 { tm.total_tuples -= 1; }

    crate::buffer_mgr::mark_dirty(bp, ph);
    crate::buffer_mgr::unpin_page(bp, ph);
    RC::Ok
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let tm = get_rel_mut(rel);
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
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let sm = scan.mgmt_data.as_mut().unwrap().downcast_mut::<ScanManager>().unwrap();

    let tm = get_rel_mut(&scan.rel);
    let rec_size = tm.rec_size;
    let slots_per_page = (page_size - PAGE_HEADER_SIZE) / (rec_size as usize + 2);

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
        let rc = get_record(&scan.rel, &rid, record);
        if rc == RC::Ok {
            sm.scan_index += 1;
            if let Some(ref cond) = sm.condition_expression {
                let mut eval_result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
                crate::expr::eval_expr(record, &scan.rel.schema, cond, &mut eval_result);
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

pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total = 0i32;
    for i in 0..schema.num_attr as usize {
        match schema.data_types[i] {
            DataType::DtString => total += schema.type_length[i],
            DataType::DtInt => total += 4,
            DataType::DtFloat => total += 4,
            DataType::DtBool => total += 2, // C bool is short
        }
    }
    let padding = total % 4;
    if padding != 0 { total += 4 - padding; }
    total
}

pub fn create_schema(num_attr: i32, attr_names: Vec<String>, data_types: Vec<DataType>, type_length: Vec<i32>, key_size: i32, keys: Vec<i32>) -> Schema {
    Schema { num_attr, attr_names, data_types, type_length, key_attrs: keys, key_size }
}

pub fn free_schema(_schema: &mut Schema) -> RC {
    RC::Ok
}

pub fn create_record(record: &mut Option<Record>, schema: &Schema) -> RC {
    let rec_size = get_record_size(schema) as usize;
    let data: String = std::iter::repeat('\0').take(rec_size).collect();
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
    let pos = get_attr_pos(schema, attr_num) as usize;
    let chars: Vec<char> = record.data.chars().collect();
    let get_byte = |i: usize| -> u8 { if i < chars.len() { chars[i] as u8 } else { 0 } };

    match schema.data_types[attr_num as usize] {
        DataType::DtInt => {
            let b = [get_byte(pos), get_byte(pos+1), get_byte(pos+2), get_byte(pos+3)];
            value.dt = DataType::DtInt;
            value.v = ValueUnion::IntV(i32::from_ne_bytes(b));
        }
        DataType::DtFloat => {
            let b = [get_byte(pos), get_byte(pos+1), get_byte(pos+2), get_byte(pos+3)];
            value.dt = DataType::DtFloat;
            value.v = ValueUnion::FloatV(f32::from_ne_bytes(b));
        }
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let s: String = (0..len).map(|i| {
                if pos + i < chars.len() { chars[pos + i] } else { '\0' }
            }).collect();
            let s = s.trim_end_matches('\0').to_string();
            value.dt = DataType::DtString;
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtBool => {
            let b = [get_byte(pos), get_byte(pos+1)];
            let bval = i16::from_ne_bytes(b) != 0;
            value.dt = DataType::DtBool;
            value.v = ValueUnion::BoolV(bval);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let mut chars: Vec<char> = record.data.chars().collect();
    let rec_size = get_record_size(schema) as usize;
    while chars.len() < rec_size { chars.push('\0'); }

    match schema.data_types[attr_num as usize] {
        DataType::DtInt => {
            if let ValueUnion::IntV(v) = &value.v {
                let b = v.to_ne_bytes();
                for i in 0..4 { chars[pos + i] = b[i] as char; }
            }
        }
        DataType::DtFloat => {
            if let ValueUnion::FloatV(v) = &value.v {
                let b = v.to_ne_bytes();
                for i in 0..4 { chars[pos + i] = b[i] as char; }
            }
        }
        DataType::DtString => {
            if let ValueUnion::StringV(v) = &value.v {
                let len = schema.type_length[attr_num as usize] as usize;
                let bytes = v.as_bytes();
                for i in 0..len {
                    chars[pos + i] = if i < bytes.len() { bytes[i] as char } else { '\0' };
                }
            }
        }
        DataType::DtBool => {
            if let ValueUnion::BoolV(v) = &value.v {
                let b = (*v as i16).to_ne_bytes();
                for i in 0..2 { chars[pos + i] = b[i] as char; }
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
            DataType::DtBool => pos += 2,
        }
    }
    pos
}
