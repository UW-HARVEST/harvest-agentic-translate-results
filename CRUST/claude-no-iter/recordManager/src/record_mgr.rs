use crate::{
    dberror::{RC, PAGE_SIZE},
    expr::{eval_expr, Expr},
    storage_mgr,
    tables::{DataType, RM_TableData, Record, Schema, Value, ValueUnion, RID},
};
use crate::buffer_mgr::{
    self, force_flush_pool, init_buffer_pool, mark_dirty, pin_page, shutdown_buffer_pool,
    unpin_page, BM_BufferPool, BM_PageHandle, ReplacementStrategy,
};

const MAX_ATTR_NAME_LEN: usize = 15;

// Record header per slot in a data page: 1 byte status ('Y' or 'N') + recSize bytes + 1 byte sentinel ('|')
// PageHeader layout (matches C struct):
//   char pageIdentifier (we use 1 byte but C struct alignment makes it 4 bytes - use 32 to match calc)
// In C, sizeof(PageHeader) = 1 (char) + 7 ints = depending on alignment; with padding it's 32 bytes typically.
const PAGE_HEADER_SIZE: usize = 32;

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

// Helper functions for byte conversion within a page string
fn page_str_to_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|c| (c as u32 & 0xFF) as u8).collect()
}

fn bytes_to_page_str(b: &[u8]) -> String {
    b.iter().map(|&x| x as char).collect()
}

fn read_i32_at(bytes: &[u8], offset: usize) -> i32 {
    if offset + 4 > bytes.len() {
        return 0;
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[offset..offset + 4]);
    i32::from_le_bytes(buf)
}

fn write_i32_at(bytes: &mut [u8], offset: usize, val: i32) {
    let buf = val.to_le_bytes();
    bytes[offset..offset + 4].copy_from_slice(&buf);
}

fn read_f32_at(bytes: &[u8], offset: usize) -> f32 {
    if offset + 4 > bytes.len() {
        return 0.0;
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[offset..offset + 4]);
    f32::from_le_bytes(buf)
}

fn write_f32_at(bytes: &mut [u8], offset: usize, val: f32) {
    let buf = val.to_le_bytes();
    bytes[offset..offset + 4].copy_from_slice(&buf);
}

fn serialize_page_header(ph: &PageHeader) -> [u8; PAGE_HEADER_SIZE] {
    let mut buf = [0u8; PAGE_HEADER_SIZE];
    buf[0] = ph.page_identifier as u8;
    // 3 bytes padding to align to 4
    write_i32_at(&mut buf, 4, ph.total_tuples);
    write_i32_at(&mut buf, 8, ph.free_slot_cnt);
    write_i32_at(&mut buf, 12, ph.next_free_slot_ind);
    write_i32_at(&mut buf, 16, ph.prev_free_page_index);
    write_i32_at(&mut buf, 20, ph.next_free_page_index);
    write_i32_at(&mut buf, 24, ph.prev_data_page_index);
    write_i32_at(&mut buf, 28, ph.next_data_page_index);
    buf
}

fn deserialize_page_header(bytes: &[u8]) -> PageHeader {
    PageHeader {
        page_identifier: if bytes.is_empty() {
            '\0'
        } else {
            bytes[0] as char
        },
        total_tuples: read_i32_at(bytes, 4),
        free_slot_cnt: read_i32_at(bytes, 8),
        next_free_slot_ind: read_i32_at(bytes, 12),
        prev_free_page_index: read_i32_at(bytes, 16),
        next_free_page_index: read_i32_at(bytes, 20),
        prev_data_page_index: read_i32_at(bytes, 24),
        next_data_page_index: read_i32_at(bytes, 28),
    }
}

fn write_page_header(page_bytes: &mut [u8], ph: &PageHeader) {
    let buf = serialize_page_header(ph);
    page_bytes[..PAGE_HEADER_SIZE].copy_from_slice(&buf);
}

fn write_table_header_to_page(
    page_bytes: &mut [u8],
    tm: &TableManager,
    schema: &Schema,
) -> usize {
    let mut pos: usize = 0;
    write_i32_at(page_bytes, pos, tm.total_tuples);
    pos += 4;
    write_i32_at(page_bytes, pos, tm.rec_size);
    pos += 4;
    write_i32_at(page_bytes, pos, tm.first_free_page_num);
    pos += 4;
    write_i32_at(page_bytes, pos, tm.first_free_slot_num);
    pos += 4;
    write_i32_at(page_bytes, pos, tm.first_data_page_num);
    pos += 4;
    write_i32_at(page_bytes, pos, schema.num_attr);
    pos += 4;
    write_i32_at(page_bytes, pos, schema.key_size);
    pos += 4;

    for i in 0..schema.num_attr as usize {
        let name_bytes = schema.attr_names[i].as_bytes();
        let copy_len = name_bytes.len().min(MAX_ATTR_NAME_LEN);
        // Zero out first
        for j in 0..MAX_ATTR_NAME_LEN {
            page_bytes[pos + j] = 0;
        }
        page_bytes[pos..pos + copy_len].copy_from_slice(&name_bytes[..copy_len]);
        pos += MAX_ATTR_NAME_LEN;
        // DataType as 4-byte int
        let dt_val = match schema.data_types[i] {
            DataType::DtInt => 0i32,
            DataType::DtString => 1,
            DataType::DtFloat => 2,
            DataType::DtBool => 3,
        };
        write_i32_at(page_bytes, pos, dt_val);
        pos += 4;
        write_i32_at(page_bytes, pos, schema.type_length[i]);
        pos += 4;
    }
    for i in 0..schema.key_size as usize {
        write_i32_at(page_bytes, pos, schema.key_attrs[i]);
        pos += 4;
    }
    pos
}

fn read_table_header_from_page(page_bytes: &[u8]) -> (TableManager, Schema) {
    let mut pos: usize = 0;
    let total_tuples = read_i32_at(page_bytes, pos);
    pos += 4;
    let rec_size = read_i32_at(page_bytes, pos);
    pos += 4;
    let first_free_page_num = read_i32_at(page_bytes, pos);
    pos += 4;
    let first_free_slot_num = read_i32_at(page_bytes, pos);
    pos += 4;
    let first_data_page_num = read_i32_at(page_bytes, pos);
    pos += 4;
    let num_attr = read_i32_at(page_bytes, pos);
    pos += 4;
    let key_size = read_i32_at(page_bytes, pos);
    pos += 4;

    let mut attr_names = Vec::with_capacity(num_attr as usize);
    let mut data_types = Vec::with_capacity(num_attr as usize);
    let mut type_length = Vec::with_capacity(num_attr as usize);
    for _ in 0..num_attr {
        let name_slice = &page_bytes[pos..pos + MAX_ATTR_NAME_LEN];
        let null_pos = name_slice
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(name_slice.len());
        let name = String::from_utf8_lossy(&name_slice[..null_pos]).to_string();
        attr_names.push(name);
        pos += MAX_ATTR_NAME_LEN;
        let dt_val = read_i32_at(page_bytes, pos);
        pos += 4;
        let dt = match dt_val {
            0 => DataType::DtInt,
            1 => DataType::DtString,
            2 => DataType::DtFloat,
            3 => DataType::DtBool,
            _ => DataType::DtInt,
        };
        data_types.push(dt);
        let tl = read_i32_at(page_bytes, pos);
        pos += 4;
        type_length.push(tl);
    }
    let mut key_attrs = Vec::with_capacity(key_size as usize);
    for _ in 0..key_size {
        key_attrs.push(read_i32_at(page_bytes, pos));
        pos += 4;
    }

    let tm = TableManager {
        total_tuples,
        rec_size,
        first_free_page_num,
        first_free_slot_num,
        first_data_page_num,
        buffer_pool: None,
        page_handler: None,
    };
    let schema = Schema {
        num_attr,
        attr_names,
        data_types,
        type_length,
        key_attrs,
        key_size,
    };
    (tm, schema)
}

pub fn create_table(name: &str, schema: &Schema) -> RC {
    let rc = storage_mgr::create_page_file(name);
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
    let mut page_handle = BM_PageHandle {
        page_num: 0,
        data: String::new(),
    };
    let rc = pin_page(&mut bp, &mut page_handle, 0);
    if rc != RC::Ok {
        return rc;
    }
    // Write table header into page
    let mut page_bytes = vec![0u8; PAGE_SIZE as usize];
    let tm = TableManager {
        total_tuples: 0,
        rec_size: get_record_size(schema),
        first_free_page_num: 1,
        first_free_slot_num: 0,
        first_data_page_num: -1,
        buffer_pool: None,
        page_handler: None,
    };
    write_table_header_to_page(&mut page_bytes, &tm, schema);
    page_handle.data = bytes_to_page_str(&page_bytes);

    let rc = mark_dirty(&mut bp, &mut page_handle);
    if rc != RC::Ok {
        return rc;
    }
    let rc = unpin_page(&mut bp, &mut page_handle);
    if rc != RC::Ok {
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
    let mut page_handle = BM_PageHandle {
        page_num: 0,
        data: String::new(),
    };
    let rc = pin_page(&mut bp, &mut page_handle, 0);
    if rc != RC::Ok {
        return rc;
    }
    let page_bytes = page_str_to_bytes(&page_handle.data);
    let (mut tm, schema) = read_table_header_from_page(&page_bytes);

    let rc = unpin_page(&mut bp, &mut page_handle);
    if rc != RC::Ok {
        return rc;
    }

    tm.buffer_pool = Some(bp);
    tm.page_handler = Some(page_handle);

    rel.name = name.to_string();
    rel.schema = schema;
    rel.mgmt_data = Some(Box::new(tm));
    RC::Ok
}

pub fn close_table(rel: &mut RM_TableData) -> RC {
    if let Some(boxed) = rel.mgmt_data.take() {
        if let Ok(mut tm_box) = boxed.downcast::<TableManager>() {
            // Take buffer pool out
            if let Some(mut bp) = tm_box.buffer_pool.take() {
                let mut page_handle = tm_box.page_handler.take().unwrap_or(BM_PageHandle {
                    page_num: 0,
                    data: String::new(),
                });
                let pin_rc = pin_page(&mut bp, &mut page_handle, 0);
                if pin_rc == RC::Ok {
                    let mut page_bytes = page_str_to_bytes(&page_handle.data);
                    if page_bytes.len() < PAGE_SIZE as usize {
                        page_bytes.resize(PAGE_SIZE as usize, 0);
                    }
                    write_i32_at(&mut page_bytes, 0, tm_box.total_tuples);
                    write_i32_at(&mut page_bytes, 4, tm_box.rec_size);
                    write_i32_at(&mut page_bytes, 8, tm_box.first_free_page_num);
                    write_i32_at(&mut page_bytes, 12, tm_box.first_free_slot_num);
                    write_i32_at(&mut page_bytes, 16, tm_box.first_data_page_num);
                    page_handle.data = bytes_to_page_str(&page_bytes);
                    let _ = mark_dirty(&mut bp, &mut page_handle);
                    let _ = unpin_page(&mut bp, &mut page_handle);
                }
                let rc = shutdown_buffer_pool(&mut bp);
                if rc != RC::Ok {
                    return rc;
                }
            }
        }
    }
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    if name.is_empty() {
        return RC::InvalidHeader;
    }
    storage_mgr::destroy_page_file(name)
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    if let Some(boxed) = rel.mgmt_data.as_ref() {
        if let Some(tm) = boxed.downcast_ref::<TableManager>() {
            return tm.total_tuples;
        }
    }
    -1
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let boxed = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match boxed.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };

    let rec_size = tm.rec_size;
    let slots_per_page = (PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);

    let mut bp = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page_handle = match tm.page_handler.take() {
        Some(p) => p,
        None => BM_PageHandle {
            page_num: 0,
            data: String::new(),
        },
    };

    let target_page = tm.first_free_page_num;
    let pin_rc = pin_page(&mut bp, &mut page_handle, target_page);
    if pin_rc != RC::Ok {
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page_handle);
        return RC::Error;
    }

    let mut page_bytes = page_str_to_bytes(&page_handle.data);
    if page_bytes.len() < PAGE_SIZE as usize {
        page_bytes.resize(PAGE_SIZE as usize, 0);
    }
    let mut header = deserialize_page_header(&page_bytes);

    if header.page_identifier != 'Y' {
        header.page_identifier = 'Y';
        header.total_tuples = 0;
        header.free_slot_cnt = (slots_per_page - 1) as i32;
        header.next_free_slot_ind = 1;
        header.prev_free_page_index = -1;
        header.next_free_page_index = page_handle.page_num + 1;
        header.prev_data_page_index = -1;
        header.next_data_page_index = 1;
        // First time using this data page; if first_data_page_num is unset, set it
        if tm.first_data_page_num == -1 {
            tm.first_data_page_num = target_page;
        }
    } else {
        header.total_tuples += 1;
        header.free_slot_cnt -= 1;
        if header.free_slot_cnt > 0 {
            header.next_free_slot_ind += 1;
        } else {
            header.next_free_slot_ind = -header.next_free_slot_ind;
        }
    }

    let position_for_new_data =
        PAGE_HEADER_SIZE + (tm.first_free_slot_num as usize) * (rec_size as usize + 2);
    page_bytes[position_for_new_data] = b'Y';
    let record_data_bytes = page_str_to_bytes(&record.data);
    let copy_len = record_data_bytes.len().min(rec_size as usize);
    page_bytes[position_for_new_data + 1..position_for_new_data + 1 + copy_len]
        .copy_from_slice(&record_data_bytes[..copy_len]);
    // Zero remainder
    for i in copy_len..rec_size as usize {
        page_bytes[position_for_new_data + 1 + i] = 0;
    }
    page_bytes[position_for_new_data + rec_size as usize + 1] = b'|';

    // Note: original C modifies record->id; Rust signature uses &Record so we can't here.
    let _ = (page_handle.page_num, tm.first_free_slot_num);

    write_page_header(&mut page_bytes, &header);
    page_handle.data = bytes_to_page_str(&page_bytes);

    if header.free_slot_cnt == 0 {
        tm.first_free_page_num += 1;
        tm.first_free_slot_num = 0;
    } else {
        tm.first_free_slot_num += 1;
    }
    tm.total_tuples += 1;

    let dirty_rc = mark_dirty(&mut bp, &mut page_handle);
    let unpin_rc = unpin_page(&mut bp, &mut page_handle);

    tm.buffer_pool = Some(bp);
    tm.page_handler = Some(page_handle);

    if dirty_rc != RC::Ok || unpin_rc != RC::Ok {
        return RC::Error;
    }
    RC::Ok
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let boxed = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match boxed.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };

    let rec_size = tm.rec_size;
    let max_slots_per_page = (PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);

    if id.slot >= max_slots_per_page as i32 {
        return RC::RecordNotFound;
    }

    let mut bp = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page_handle = match tm.page_handler.take() {
        Some(p) => p,
        None => BM_PageHandle {
            page_num: 0,
            data: String::new(),
        },
    };

    let pin_rc = pin_page(&mut bp, &mut page_handle, id.page);
    if pin_rc != RC::Ok {
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page_handle);
        return pin_rc;
    }

    let mut page_bytes = page_str_to_bytes(&page_handle.data);
    if page_bytes.len() < PAGE_SIZE as usize {
        page_bytes.resize(PAGE_SIZE as usize, 0);
    }
    let record_position = PAGE_HEADER_SIZE + (id.slot as usize) * (rec_size as usize + 2);
    if page_bytes[record_position] != b'Y' {
        let _ = unpin_page(&mut bp, &mut page_handle);
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page_handle);
        return RC::RecordNotFound;
    }
    page_bytes[record_position] = b'N';

    let mut header = deserialize_page_header(&page_bytes);
    if header.total_tuples > 0 {
        header.total_tuples -= 1;
    }
    header.free_slot_cnt += 1;
    write_page_header(&mut page_bytes, &header);

    if tm.total_tuples > 0 {
        tm.total_tuples -= 1;
    }

    page_handle.data = bytes_to_page_str(&page_bytes);

    let dirty_rc = mark_dirty(&mut bp, &mut page_handle);
    let unpin_rc = unpin_page(&mut bp, &mut page_handle);

    tm.buffer_pool = Some(bp);
    tm.page_handler = Some(page_handle);

    if dirty_rc != RC::Ok {
        return RC::Error;
    }
    unpin_rc
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let boxed = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match boxed.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let rec_size = tm.rec_size;
    let max_slots_per_page = (PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);
    if record.id.slot >= max_slots_per_page as i32 {
        return RC::RecordNotFound;
    }
    let mut bp = match tm.buffer_pool.take() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page_handle = match tm.page_handler.take() {
        Some(p) => p,
        None => BM_PageHandle {
            page_num: 0,
            data: String::new(),
        },
    };
    let pin_rc = pin_page(&mut bp, &mut page_handle, record.id.page);
    if pin_rc != RC::Ok {
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page_handle);
        return RC::Error;
    }
    let mut page_bytes = page_str_to_bytes(&page_handle.data);
    if page_bytes.len() < PAGE_SIZE as usize {
        page_bytes.resize(PAGE_SIZE as usize, 0);
    }
    let target_pos = PAGE_HEADER_SIZE + (record.id.slot as usize) * (rec_size as usize + 2);
    if page_bytes[target_pos] != b'Y' {
        let _ = unpin_page(&mut bp, &mut page_handle);
        tm.buffer_pool = Some(bp);
        tm.page_handler = Some(page_handle);
        return RC::RecordNotFound;
    }
    let record_data_bytes = page_str_to_bytes(&record.data);
    let copy_len = record_data_bytes.len().min(rec_size as usize);
    page_bytes[target_pos + 1..target_pos + 1 + copy_len]
        .copy_from_slice(&record_data_bytes[..copy_len]);
    page_handle.data = bytes_to_page_str(&page_bytes);

    let dirty_rc = mark_dirty(&mut bp, &mut page_handle);
    let unpin_rc = unpin_page(&mut bp, &mut page_handle);
    tm.buffer_pool = Some(bp);
    tm.page_handler = Some(page_handle);

    if dirty_rc != RC::Ok || unpin_rc != RC::Ok {
        return RC::Error;
    }
    RC::Ok
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    // We need to be able to use buffer pool but rel is &.
    // To work around this, we use raw pointer dance: but C signature also takes RM_TableData* without const.
    // The Rust signature is &RM_TableData; we'll downcast and use interior mutability via raw pointer.
    let boxed = match rel.mgmt_data.as_ref() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match boxed.downcast_ref::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    let rec_size = tm.rec_size;
    let slots_per_record = (PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);
    if id.slot >= slots_per_record as i32 {
        return RC::RecordNotFound;
    }

    // Use a fresh buffer pool to read the page (since we don't have mutable access).
    // Open the page file directly and read the page using storage_mgr.
    let mut fh = storage_mgr::SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    };
    let rc = storage_mgr::open_page_file(&rel.name, &mut fh);
    if rc != RC::Ok {
        return RC::Error;
    }
    let mut page_str = String::new();
    let rc = storage_mgr::read_block(id.page, &mut fh, &mut page_str);
    let _ = storage_mgr::close_page_file(&mut fh);
    if rc != RC::Ok {
        return RC::Error;
    }

    // Also check if the buffer pool has this page dirty (more recent data).
    // For simplicity, if the buffer pool has the page in memory, prefer that.
    if let Some(bp) = tm.buffer_pool.as_ref() {
        let frames = buffer_mgr::get_frame_contents(bp);
        for (i, fc) in frames.iter().enumerate() {
            if *fc == id.page {
                // Find the corresponding bytes in the buffer pool's pagedata
                if let Some(boxed) = bp.mgmt_data.as_ref() {
                    if let Some(bp_inner) = boxed.downcast_ref::<buffer_mgr::Bufferpool>() {
                        let offset = i * PAGE_SIZE as usize;
                        let bytes: Vec<u8> = bp_inner
                            .pagedata
                            .chars()
                            .skip(offset)
                            .take(PAGE_SIZE as usize)
                            .map(|c| (c as u32 & 0xFF) as u8)
                            .collect();
                        page_str = bytes_to_page_str(&bytes);
                    }
                }
                break;
            }
        }
    }

    let page_bytes = page_str_to_bytes(&page_str);
    let record_pos = PAGE_HEADER_SIZE + (id.slot as usize) * (rec_size as usize + 2);
    if record_pos >= page_bytes.len() || page_bytes[record_pos] != b'Y' {
        return RC::RecordNotFound;
    }
    let data_start = record_pos + 1;
    let data_end = (data_start + rec_size as usize).min(page_bytes.len());
    let data_slice = &page_bytes[data_start..data_end];
    record.data = bytes_to_page_str(data_slice);
    record.id = id.clone();
    RC::Ok
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    // We can't easily clone rel, so we'll re-open the table for the scan handle.
    let total_entries = get_num_tuples(rel);
    let first_data_page = if let Some(boxed) = rel.mgmt_data.as_ref() {
        if let Some(tm) = boxed.downcast_ref::<TableManager>() {
            tm.first_data_page_num
        } else {
            -1
        }
    } else {
        -1
    };
    let starting_page = if first_data_page == -1 {
        1
    } else {
        first_data_page
    };
    let scan_mgr = ScanManager {
        total_entries,
        scan_index: 0,
        current_page_num: starting_page,
        current_slot_num: -1,
        condition_expression: Some(cond.clone()),
        scan_page_handle_ptr: None,
    };
    // Set up scan handle: we need a copy of rel's table data; let's set scan.rel by re-opening.
    let mut new_rel = RM_TableData {
        name: rel.name.clone(),
        schema: rel.schema.clone(),
        mgmt_data: None,
    };
    let rc = open_table(&mut new_rel, &rel.name);
    if rc != RC::Ok {
        return rc;
    }
    scan.rel = new_rel;
    scan.mgmt_data = Some(Box::new(scan_mgr));
    RC::Ok
}

pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    let scan_mgr = match scan.mgmt_data.as_mut() {
        Some(b) => match b.downcast_mut::<ScanManager>() {
            Some(s) => s,
            None => return RC::Error,
        },
        None => return RC::Error,
    };

    // Snapshot scan parameters
    let total_entries = scan_mgr.total_entries;
    let mut scan_index = scan_mgr.scan_index;
    let mut current_page_num = scan_mgr.current_page_num;
    let mut current_slot_num = scan_mgr.current_slot_num;
    let cond_clone = scan_mgr.condition_expression.clone();

    if scan_index >= total_entries {
        return RC::RmNoMoreTuples;
    }

    // Determine slots per page from rel
    let rec_size = if let Some(boxed) = scan.rel.mgmt_data.as_ref() {
        if let Some(tm) = boxed.downcast_ref::<TableManager>() {
            tm.rec_size
        } else {
            return RC::Error;
        }
    } else {
        return RC::Error;
    };
    let slots_per_page = (PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (rec_size as usize + 2);

    let schema = scan.rel.schema.clone();

    let mut result_rc;
    loop {
        current_slot_num += 1;
        if current_slot_num >= slots_per_page as i32 {
            current_page_num += 1;
            current_slot_num = 0;
        }
        let current_rid = RID {
            page: current_page_num,
            slot: current_slot_num,
        };
        let record_status = get_record(&scan.rel, &current_rid, record);
        if record_status == RC::Ok {
            scan_index += 1;
            if let Some(cond) = &cond_clone {
                let mut eval_result = Value {
                    dt: DataType::DtBool,
                    v: ValueUnion::BoolV(false),
                };
                let _ = eval_expr(record, &schema, cond, &mut eval_result);
                let matched = match eval_result.v {
                    ValueUnion::BoolV(b) => b,
                    _ => false,
                };
                if matched {
                    result_rc = RC::Ok;
                    break;
                }
            } else {
                result_rc = RC::Ok;
                break;
            }
        }
        if scan_index >= total_entries {
            result_rc = RC::RmNoMoreTuples;
            break;
        }
        // Safety bound to avoid infinite loop
        if current_page_num > 10000 {
            result_rc = RC::RmNoMoreTuples;
            break;
        }
    }

    // Update scan manager state
    if let Some(b) = scan.mgmt_data.as_mut() {
        if let Some(s) = b.downcast_mut::<ScanManager>() {
            s.scan_index = scan_index;
            s.current_page_num = current_page_num;
            s.current_slot_num = current_slot_num;
        }
    }
    result_rc
}

pub fn close_scan(scan: &mut RM_ScanHandle) -> RC {
    // Close the table opened during start_scan
    let _ = close_table(&mut scan.rel);
    scan.mgmt_data = None;
    RC::Ok
}

pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total: i32 = 0;
    for i in 0..schema.num_attr as usize {
        let dt = &schema.data_types[i];
        let len = schema.type_length[i];
        match dt {
            DataType::DtString => total += len,
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
    let pos = get_attr_pos(schema, attr_num);
    let bytes = page_str_to_bytes(&record.data);
    let idx = attr_num as usize;
    let dt = schema.data_types[idx].clone();
    match dt {
        DataType::DtString => {
            let len = schema.type_length[idx] as usize;
            let end = (pos as usize + len).min(bytes.len());
            let slice = &bytes[pos as usize..end];
            // strip trailing zeros
            let null_pos = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
            let s = String::from_utf8_lossy(&slice[..null_pos]).to_string();
            value.dt = DataType::DtString;
            value.v = ValueUnion::StringV(s);
        }
        DataType::DtInt => {
            let v = read_i32_at(&bytes, pos as usize);
            value.dt = DataType::DtInt;
            value.v = ValueUnion::IntV(v);
        }
        DataType::DtFloat => {
            let v = read_f32_at(&bytes, pos as usize);
            value.dt = DataType::DtFloat;
            value.v = ValueUnion::FloatV(v);
        }
        DataType::DtBool => {
            let b = bytes
                .get(pos as usize)
                .map(|&x| x != 0)
                .unwrap_or(false);
            value.dt = DataType::DtBool;
            value.v = ValueUnion::BoolV(b);
        }
    }
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let pos = get_attr_pos(schema, attr_num) as usize;
    let mut bytes = page_str_to_bytes(&record.data);
    // Ensure capacity
    let rec_size = get_record_size(schema) as usize;
    if bytes.len() < rec_size {
        bytes.resize(rec_size, 0);
    }
    let idx = attr_num as usize;
    match (&schema.data_types[idx], &value.v) {
        (DataType::DtInt, ValueUnion::IntV(v)) => {
            write_i32_at(&mut bytes, pos, *v);
        }
        (DataType::DtFloat, ValueUnion::FloatV(v)) => {
            write_f32_at(&mut bytes, pos, *v);
        }
        (DataType::DtString, ValueUnion::StringV(s)) => {
            let len = schema.type_length[idx] as usize;
            let s_bytes = s.as_bytes();
            let copy_len = s_bytes.len().min(len);
            bytes[pos..pos + copy_len].copy_from_slice(&s_bytes[..copy_len]);
            // Pad rest with zeros
            for i in copy_len..len {
                bytes[pos + i] = 0;
            }
        }
        (DataType::DtBool, ValueUnion::BoolV(b)) => {
            bytes[pos] = if *b { 1 } else { 0 };
        }
        _ => {}
    }
    record.data = bytes_to_page_str(&bytes);
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut attr_pos: i32 = 0;
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

// Force flush all dirty pages helper
#[allow(dead_code)]
fn flush_table(rel: &mut RM_TableData) -> RC {
    let boxed = match rel.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let tm = match boxed.downcast_mut::<TableManager>() {
        Some(t) => t,
        None => return RC::Error,
    };
    if let Some(bp) = tm.buffer_pool.as_mut() {
        return force_flush_pool(bp);
    }
    RC::Ok
}
