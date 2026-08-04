use crate::{
    buffer_mgr::{
        init_buffer_pool, mark_dirty, pin_page, shutdown_buffer_pool, unpin_page, BM_BufferPool,
        BM_PageHandle, ReplacementStrategy,
    },
    dberror::{RC, PAGE_SIZE},
    expr::{eval_expr, Expr},
    tables::{
        bytes_to_data, clone_schema, data_to_bytes, ensure_byte_len, read_bool, read_f32, read_i32,
        write_bool, write_f32, write_i32, DataType, Record, RID, RM_TableData, Schema, Value,
        ValueUnion, BOOL_SIZE,
    },
};

const MAX_ATTR_NAME_LEN: usize = 15;
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

struct InternalScan {
    rel: RM_TableData,
    total_entries: i32,
    scan_index: i32,
    current_page_num: i32,
    current_slot_num: i32,
    condition_expression: Option<Expr>,
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

fn manager_mut(rel: &mut RM_TableData) -> Result<&mut TableManager, RC> {
    rel.mgmt_data
        .as_mut()
        .and_then(|data| data.downcast_mut::<TableManager>())
        .ok_or(RC::GeneralError)
}

fn manager_ref(rel: &RM_TableData) -> Result<&TableManager, RC> {
    rel.mgmt_data
        .as_ref()
        .and_then(|data| data.downcast_ref::<TableManager>())
        .ok_or(RC::GeneralError)
}

fn page_header_from_bytes(bytes: &[u8]) -> PageHeader {
    PageHeader {
        page_identifier: char::from(*bytes.first().unwrap_or(&0)),
        total_tuples: read_i32(bytes, 4),
        free_slot_cnt: read_i32(bytes, 8),
        next_free_slot_ind: read_i32(bytes, 12),
        prev_free_page_index: read_i32(bytes, 16),
        next_free_page_index: read_i32(bytes, 20),
        prev_data_page_index: read_i32(bytes, 24),
        next_data_page_index: read_i32(bytes, 28),
    }
}

fn write_page_header(bytes: &mut [u8], header: &PageHeader) {
    bytes[0] = header.page_identifier as u8;
    bytes[1..4].fill(0);
    write_i32(bytes, 4, header.total_tuples);
    write_i32(bytes, 8, header.free_slot_cnt);
    write_i32(bytes, 12, header.next_free_slot_ind);
    write_i32(bytes, 16, header.prev_free_page_index);
    write_i32(bytes, 20, header.next_free_page_index);
    write_i32(bytes, 24, header.prev_data_page_index);
    write_i32(bytes, 28, header.next_data_page_index);
}

fn table_header_bytes(table_manager: &TableManager, schema: &Schema) -> Vec<u8> {
    let mut bytes = vec![0u8; PAGE_SIZE as usize];
    let mut offset = 0usize;
    for value in [
        table_manager.total_tuples,
        table_manager.rec_size,
        table_manager.first_free_page_num,
        table_manager.first_free_slot_num,
        table_manager.first_data_page_num,
        schema.num_attr,
        schema.key_size,
    ] {
        write_i32(&mut bytes, offset, value);
        offset += 4;
    }
    for i in 0..schema.num_attr as usize {
        let mut name = schema.attr_names[i].as_bytes().to_vec();
        ensure_byte_len(&mut name, MAX_ATTR_NAME_LEN);
        bytes[offset..offset + MAX_ATTR_NAME_LEN].copy_from_slice(&name);
        offset += MAX_ATTR_NAME_LEN;
        write_i32(&mut bytes, offset, schema.data_types[i].clone() as i32);
        offset += 4;
        write_i32(&mut bytes, offset, schema.type_length[i]);
        offset += 4;
    }
    for key in &schema.key_attrs {
        write_i32(&mut bytes, offset, *key);
        offset += 4;
    }
    bytes
}

fn data_type_from_i32(value: i32) -> DataType {
    match value {
        0 => DataType::DtInt,
        1 => DataType::DtString,
        2 => DataType::DtFloat,
        3 => DataType::DtBool,
        _ => DataType::DtInt,
    }
}

pub fn init_record_manager(_mgmt_data: Option<Box<dyn std::any::Any>>) -> RC {
    RC::Ok
}

pub fn shutdown_record_manager() -> RC {
    RC::Ok
}

pub fn create_table(name: &str, schema: &Schema) -> RC {
    let rc = crate::storage_mgr::create_page_file(name);
    if rc != RC::Ok {
        return rc;
    }

    let mut buffer_pool = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let mut page_handle = BM_PageHandle {
        page_num: 0,
        data: String::new(),
    };
    let mut table_manager = TableManager {
        total_tuples: 0,
        rec_size: get_record_size(schema),
        first_free_page_num: 1,
        first_free_slot_num: 0,
        first_data_page_num: -1,
        buffer_pool: None,
        page_handler: None,
    };

    let rc = init_buffer_pool(&mut buffer_pool, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let rc = pin_page(&mut buffer_pool, &mut page_handle, 0);
    if rc != RC::Ok {
        return rc;
    }

    page_handle.data = bytes_to_data(&table_header_bytes(&table_manager, schema));
    let rc = mark_dirty(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        return rc;
    }
    let rc = unpin_page(&mut buffer_pool, &mut page_handle);
    if rc != RC::Ok {
        return rc;
    }
    table_manager.buffer_pool = Some(buffer_pool);
    if let Some(ref mut pool) = table_manager.buffer_pool {
        shutdown_buffer_pool(pool)
    } else {
        RC::GeneralError
    }
}

pub fn open_table(rel: &mut RM_TableData, name: &str) -> RC {
    let mut buffer_pool = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let mut page_handle = BM_PageHandle {
        page_num: 0,
        data: String::new(),
    };
    let rc = init_buffer_pool(&mut buffer_pool, name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return rc;
    }
    let rc = pin_page(&mut buffer_pool, &mut page_handle, 0);
    if rc != RC::Ok {
        return rc;
    }

    let bytes = data_to_bytes(&page_handle.data);
    let total_tuples = read_i32(&bytes, 0);
    let rec_size = read_i32(&bytes, 4);
    let first_free_page_num = read_i32(&bytes, 8);
    let first_free_slot_num = read_i32(&bytes, 12);
    let first_data_page_num = read_i32(&bytes, 16);
    let num_attr = read_i32(&bytes, 20);
    let key_size = read_i32(&bytes, 24);

    let mut offset = 28usize;
    let mut attr_names = Vec::with_capacity(num_attr as usize);
    let mut data_types = Vec::with_capacity(num_attr as usize);
    let mut type_length = Vec::with_capacity(num_attr as usize);
    for _ in 0..num_attr {
        let name_bytes = &bytes[offset..offset + MAX_ATTR_NAME_LEN];
        let end = name_bytes
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(MAX_ATTR_NAME_LEN);
        attr_names.push(String::from_utf8_lossy(&name_bytes[..end]).to_string());
        offset += MAX_ATTR_NAME_LEN;
        data_types.push(data_type_from_i32(read_i32(&bytes, offset)));
        offset += 4;
        type_length.push(read_i32(&bytes, offset));
        offset += 4;
    }
    let mut key_attrs = Vec::with_capacity(key_size as usize);
    for _ in 0..key_size {
        key_attrs.push(read_i32(&bytes, offset));
        offset += 4;
    }

    let rc = unpin_page(&mut buffer_pool, &mut page_handle);
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
    rel.mgmt_data = Some(Box::new(TableManager {
        total_tuples,
        rec_size,
        first_free_page_num,
        first_free_slot_num,
        first_data_page_num,
        buffer_pool: Some(buffer_pool),
        page_handler: Some(page_handle),
    }));
    RC::Ok
}

pub fn close_table(rel: &mut RM_TableData) -> RC {
    let schema = rel.schema.clone();
    let manager = match manager_mut(rel) {
        Ok(manager) => manager,
        Err(rc) => return rc,
    };
    let header_snapshot = TableManager {
        total_tuples: manager.total_tuples,
        rec_size: manager.rec_size,
        first_free_page_num: manager.first_free_page_num,
        first_free_slot_num: manager.first_free_slot_num,
        first_data_page_num: manager.first_data_page_num,
        buffer_pool: None,
        page_handler: None,
    };
    let header_bytes = table_header_bytes(&header_snapshot, &schema);
    let Some(buffer_pool) = manager.buffer_pool.as_mut() else {
        return RC::GeneralError;
    };
    let Some(page_handle) = manager.page_handler.as_mut() else {
        return RC::GeneralError;
    };

    let rc = pin_page(buffer_pool, page_handle, 0);
    if rc != RC::Ok {
        return rc;
    }
    page_handle.data = bytes_to_data(&header_bytes);
    let rc = mark_dirty(buffer_pool, page_handle);
    if rc != RC::Ok {
        return rc;
    }
    let rc = unpin_page(buffer_pool, page_handle);
    if rc != RC::Ok {
        return rc;
    }
    let rc = shutdown_buffer_pool(buffer_pool);
    if rc != RC::Ok {
        return rc;
    }
    rel.mgmt_data = None;
    RC::Ok
}

pub fn delete_table(name: &str) -> RC {
    crate::storage_mgr::destroy_page_file(name)
}

pub fn get_num_tuples(rel: &RM_TableData) -> i32 {
    manager_ref(rel).map(|manager| manager.total_tuples).unwrap_or(-1)
}

pub fn insert_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let manager = match manager_mut(rel) {
        Ok(manager) => manager,
        Err(rc) => return rc,
    };
    let slots_per_page = ((PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (manager.rec_size as usize + 2)) as i32;

    let Some(buffer_pool) = manager.buffer_pool.as_mut() else {
        return RC::GeneralError;
    };
    let Some(page_handle) = manager.page_handler.as_mut() else {
        return RC::GeneralError;
    };
    let rc = pin_page(buffer_pool, page_handle, manager.first_free_page_num);
    if rc != RC::Ok {
        return RC::Error;
    }

    let mut page = data_to_bytes(&page_handle.data);
    ensure_byte_len(&mut page, PAGE_SIZE as usize);
    let mut header = page_header_from_bytes(&page);
    if header.page_identifier != 'Y' {
        header.page_identifier = 'Y';
        header.total_tuples = 0;
        header.free_slot_cnt = slots_per_page - 1;
        header.next_free_slot_ind = 1;
        header.prev_free_page_index = -1;
        header.next_free_page_index = page_handle.page_num + 1;
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

    let pos = PAGE_HEADER_SIZE + manager.first_free_slot_num as usize * (manager.rec_size as usize + 2);
    page[pos] = b'Y';
    let mut record_bytes = data_to_bytes(&record.data);
    ensure_byte_len(&mut record_bytes, manager.rec_size as usize);
    page[pos + 1..pos + 1 + manager.rec_size as usize].copy_from_slice(&record_bytes);
    page[pos + manager.rec_size as usize + 1] = b'|';
    write_page_header(&mut page, &header);
    page_handle.data = bytes_to_data(&page);

    if header.free_slot_cnt == 0 {
        manager.first_free_page_num += 1;
        manager.first_free_slot_num = 0;
    } else {
        manager.first_free_slot_num += 1;
    }
    manager.total_tuples += 1;

    let rc = mark_dirty(buffer_pool, page_handle);
    if rc != RC::Ok {
        return RC::Error;
    }
    let rc = unpin_page(buffer_pool, page_handle);
    if rc != RC::Ok {
        return RC::Error;
    }

    let record_ptr = record as *const Record as *mut Record;
    if let Some(record_mut) = unsafe { record_ptr.as_mut() } {
        record_mut.id.page = page_handle.page_num;
        record_mut.id.slot = manager.first_free_slot_num - 1;
        if header.free_slot_cnt == 0 {
            record_mut.id.slot = slots_per_page - 1;
        }
    }
    RC::Ok
}

pub fn delete_record(rel: &mut RM_TableData, id: &RID) -> RC {
    let manager = match manager_mut(rel) {
        Ok(manager) => manager,
        Err(rc) => return rc,
    };
    let slots_per_page = ((PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (manager.rec_size as usize + 2)) as i32;
    if id.slot >= slots_per_page {
        return RC::RecordNotFound;
    }

    let Some(buffer_pool) = manager.buffer_pool.as_mut() else {
        return RC::GeneralError;
    };
    let Some(page_handle) = manager.page_handler.as_mut() else {
        return RC::GeneralError;
    };
    let rc = pin_page(buffer_pool, page_handle, id.page);
    if rc != RC::Ok {
        return rc;
    }

    let mut page = data_to_bytes(&page_handle.data);
    let pos = PAGE_HEADER_SIZE + id.slot as usize * (manager.rec_size as usize + 2);
    if page.get(pos).copied().unwrap_or_default() != b'Y' {
        let _ = unpin_page(buffer_pool, page_handle);
        return RC::RecordNotFound;
    }
    page[pos] = b'N';
    let mut header = page_header_from_bytes(&page);
    header.total_tuples = (header.total_tuples - 1).max(0);
    header.free_slot_cnt += 1;
    write_page_header(&mut page, &header);
    page_handle.data = bytes_to_data(&page);
    manager.total_tuples = (manager.total_tuples - 1).max(0);

    let rc = mark_dirty(buffer_pool, page_handle);
    if rc != RC::Ok {
        let _ = unpin_page(buffer_pool, page_handle);
        return RC::Error;
    }
    unpin_page(buffer_pool, page_handle)
}

pub fn update_record(rel: &mut RM_TableData, record: &Record) -> RC {
    let manager = match manager_mut(rel) {
        Ok(manager) => manager,
        Err(rc) => return rc,
    };
    let slots_per_page = ((PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (manager.rec_size as usize + 2)) as i32;
    if record.id.slot >= slots_per_page {
        return RC::RecordNotFound;
    }

    let Some(buffer_pool) = manager.buffer_pool.as_mut() else {
        return RC::GeneralError;
    };
    let Some(page_handle) = manager.page_handler.as_mut() else {
        return RC::GeneralError;
    };
    let rc = pin_page(buffer_pool, page_handle, record.id.page);
    if rc != RC::Ok {
        return RC::Error;
    }

    let mut page = data_to_bytes(&page_handle.data);
    let pos = PAGE_HEADER_SIZE + record.id.slot as usize * (manager.rec_size as usize + 2);
    if page.get(pos).copied().unwrap_or_default() != b'Y' {
        let _ = unpin_page(buffer_pool, page_handle);
        return RC::RecordNotFound;
    }
    let mut record_bytes = data_to_bytes(&record.data);
    ensure_byte_len(&mut record_bytes, manager.rec_size as usize);
    page[pos + 1..pos + 1 + manager.rec_size as usize].copy_from_slice(&record_bytes);
    page_handle.data = bytes_to_data(&page);
    let rc = mark_dirty(buffer_pool, page_handle);
    if rc != RC::Ok {
        return RC::Error;
    }
    let rc = unpin_page(buffer_pool, page_handle);
    if rc != RC::Ok {
        return RC::Error;
    }
    RC::Ok
}

pub fn get_record(rel: &RM_TableData, id: &RID, record: &mut Record) -> RC {
    let manager = match manager_ref(rel) {
        Ok(manager) => manager,
        Err(rc) => return rc,
    };
    let slots_per_page = ((PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (manager.rec_size as usize + 2)) as i32;
    if id.slot >= slots_per_page {
        return RC::RecordNotFound;
    }

    let mut local_pool = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let mut local_page = BM_PageHandle {
        page_num: 0,
        data: String::new(),
    };
    let rc = init_buffer_pool(&mut local_pool, &rel.name, 3, ReplacementStrategy::RsFifo, None);
    if rc != RC::Ok {
        return RC::Error;
    }
    let rc = pin_page(&mut local_pool, &mut local_page, id.page);
    if rc != RC::Ok {
        let _ = shutdown_buffer_pool(&mut local_pool);
        return RC::Error;
    }

    let page = data_to_bytes(&local_page.data);
    let pos = PAGE_HEADER_SIZE + id.slot as usize * (manager.rec_size as usize + 2);
    if page.get(pos).copied().unwrap_or_default() != b'Y' {
        let _ = unpin_page(&mut local_pool, &mut local_page);
        let _ = shutdown_buffer_pool(&mut local_pool);
        return RC::RecordNotFound;
    }

    record.id = id.clone();
    let data = &page[pos + 1..pos + 1 + manager.rec_size as usize];
    record.data = bytes_to_data(data);

    let rc = unpin_page(&mut local_pool, &mut local_page);
    let _ = shutdown_buffer_pool(&mut local_pool);
    rc
}

pub fn start_scan(rel: &RM_TableData, scan: &mut RM_ScanHandle, cond: &Expr) -> RC {
    let mut scan_rel = RM_TableData {
        name: String::new(),
        schema: Schema::default(),
        mgmt_data: None,
    };
    let rc = open_table(&mut scan_rel, &rel.name);
    if rc != RC::Ok {
        return rc;
    }
    let first_data_page_num = manager_ref(&scan_rel)
        .map(|manager| manager.first_data_page_num)
        .unwrap_or(-1);
    let total_entries = manager_ref(&scan_rel)
        .map(|manager| manager.total_tuples)
        .unwrap_or(0);

    scan.rel = RM_TableData {
        name: rel.name.clone(),
        schema: clone_schema(&rel.schema),
        mgmt_data: None,
    };
    scan.mgmt_data = Some(Box::new(InternalScan {
        rel: scan_rel,
        total_entries,
        scan_index: 0,
        current_page_num: first_data_page_num,
        current_slot_num: -1,
        condition_expression: Some(cond.clone()),
    }));
    RC::Ok
}

pub fn next(scan: &mut RM_ScanHandle, record: &mut Record) -> RC {
    let internal = scan
        .mgmt_data
        .as_mut()
        .and_then(|data| data.downcast_mut::<InternalScan>())
        .ok_or(RC::RecordNotFound);
    let internal = match internal {
        Ok(internal) => internal,
        Err(rc) => return rc,
    };

    let table_mgr = match manager_ref(&internal.rel) {
        Ok(manager) => manager,
        Err(rc) => return rc,
    };
    let slots_per_page =
        ((PAGE_SIZE as usize - PAGE_HEADER_SIZE) / (table_mgr.rec_size as usize + 2)) as i32;

    if internal.scan_index >= internal.total_entries {
        return RC::RmNoMoreTuples;
    }

    loop {
        internal.current_slot_num += 1;
        if internal.current_slot_num >= slots_per_page {
            internal.current_page_num += 1;
            internal.current_slot_num = 0;
        }

        let current_rid = RID {
            page: internal.current_page_num,
            slot: internal.current_slot_num,
        };
        let rc = get_record(&internal.rel, &current_rid, record);
        if rc == RC::Ok {
            internal.scan_index += 1;
            if let Some(expr) = &internal.condition_expression {
                let mut result = Value {
                    dt: DataType::DtInt,
                    v: ValueUnion::IntV(-1),
                };
                let rc = eval_expr(record, &internal.rel.schema, expr, &mut result);
                if rc != RC::Ok {
                    return rc;
                }
                if !matches!(result.v, ValueUnion::BoolV(true)) {
                    if internal.scan_index >= internal.total_entries {
                        return RC::RmNoMoreTuples;
                    }
                    continue;
                }
            }
            return RC::Ok;
        }

        if internal.scan_index >= internal.total_entries {
            return RC::RmNoMoreTuples;
        }
    }
}

pub fn close_scan(scan: &mut RM_ScanHandle) -> RC {
    if let Some(mut data) = scan.mgmt_data.take() {
        if let Some(internal) = data.downcast_mut::<InternalScan>() {
            let _ = close_table(&mut internal.rel);
        }
    }
    RC::Ok
}

pub fn get_record_size(schema: &Schema) -> i32 {
    let mut total_size = 0usize;
    for i in 0..schema.num_attr as usize {
        total_size += match schema.data_types[i] {
            DataType::DtString => schema.type_length[i] as usize,
            DataType::DtInt => 4,
            DataType::DtFloat => 4,
            DataType::DtBool => BOOL_SIZE,
        };
    }
    let padding = total_size % 4;
    if padding != 0 {
        total_size += 4 - padding;
    }
    total_size as i32
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
    *record = Some(Record {
        id: RID::default(),
        data: bytes_to_data(&vec![0u8; get_record_size(schema) as usize + 1]),
    });
    RC::Ok
}

pub fn free_record(record: &mut Record) -> RC {
    record.data.clear();
    RC::Ok
}

pub fn get_attr(record: &Record, schema: &Schema, attr_num: i32, value: &mut Value) -> RC {
    let offset = get_attr_pos(schema, attr_num) as usize;
    let bytes = data_to_bytes(&record.data);
    value.dt = schema.data_types[attr_num as usize].clone();
    value.v = match schema.data_types[attr_num as usize] {
        DataType::DtString => {
            let len = schema.type_length[attr_num as usize] as usize;
            let text = String::from_utf8(
                bytes[offset..offset + len]
                    .iter()
                    .copied()
                    .take_while(|b| *b != 0)
                    .collect(),
            )
            .unwrap_or_default();
            ValueUnion::StringV(text)
        }
        DataType::DtInt => ValueUnion::IntV(read_i32(&bytes, offset)),
        DataType::DtFloat => ValueUnion::FloatV(read_f32(&bytes, offset)),
        DataType::DtBool => ValueUnion::BoolV(read_bool(&bytes, offset)),
    };
    RC::Ok
}

pub fn set_attr(record: &mut Record, schema: &Schema, attr_num: i32, value: &Value) -> RC {
    let mut bytes = data_to_bytes(&record.data);
    ensure_byte_len(&mut bytes, get_record_size(schema) as usize + 1);
    let offset = get_attr_pos(schema, attr_num) as usize;
    match (&schema.data_types[attr_num as usize], &value.v) {
        (DataType::DtInt, ValueUnion::IntV(v)) => write_i32(&mut bytes, offset, *v),
        (DataType::DtFloat, ValueUnion::FloatV(v)) => write_f32(&mut bytes, offset, *v),
        (DataType::DtString, ValueUnion::StringV(v)) => {
            let len = schema.type_length[attr_num as usize] as usize;
            bytes[offset..offset + len].fill(0);
            let raw = v.as_bytes();
            let copy_len = raw.len().min(len);
            bytes[offset..offset + copy_len].copy_from_slice(&raw[..copy_len]);
        }
        (DataType::DtBool, ValueUnion::BoolV(v)) => write_bool(&mut bytes, offset, *v),
        _ => {}
    }
    record.data = bytes_to_data(&bytes);
    RC::Ok
}

pub fn get_attr_pos(schema: &Schema, attr_num: i32) -> i32 {
    let mut pos = 0usize;
    for i in 0..attr_num as usize {
        pos += match schema.data_types[i] {
            DataType::DtString => schema.type_length[i] as usize,
            DataType::DtInt => 4,
            DataType::DtFloat => 4,
            DataType::DtBool => BOOL_SIZE,
        };
    }
    pos as i32
}
