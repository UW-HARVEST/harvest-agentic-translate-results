use crate::dberror::RC;
use crate::storage_mgr::{SM_FileHandle, SM_PageHandle};

pub struct Bufferpool {
    pub num_read: i32,
    pub num_write: i32,
    pub total_pages: i32,
    pub updated_strategy: i32,
    pub free_space: i32,
    pub updated_order: Vec<i32>,
    pub bitdirty: Vec<bool>,
    pub fix_count: Vec<i32>,
    pub access_time: Vec<i32>,
    pub pagenum: Vec<i32>,
    pub pagedata: String,
    pub fhl: SM_FileHandle,
}

pub struct BM_PageHandle {
    pub page_num: PageNumber,
    pub data: String,
}

pub type PageNumber = i32;
pub const NO_PAGE: PageNumber = -1;

pub enum ReplacementStrategy {
    RsFifo = 0,
    RsLru = 1,
    RsClock = 2,
    RsLfu = 3,
    RsLruK = 4,
}

pub struct BM_BufferPool {
    pub page_file: String,
    pub num_pages: i32,
    pub strategy: ReplacementStrategy,
    pub mgmt_data: Option<Box<dyn std::any::Any>>,
}

fn get_bp(bm: &BM_BufferPool) -> &Bufferpool {
    bm.mgmt_data.as_ref().unwrap().downcast_ref::<Bufferpool>().unwrap()
}

fn get_bp_mut(bm: &mut BM_BufferPool) -> &mut Bufferpool {
    bm.mgmt_data.as_mut().unwrap().downcast_mut::<Bufferpool>().unwrap()
}

// Helper: get a slice of pagedata for a given frame index
fn get_frame_data(pagedata: &str, idx: usize, page_size: usize) -> String {
    let start = idx * page_size;
    let end = start + page_size;
    if end <= pagedata.len() {
        pagedata[start..end].to_string()
    } else {
        let mut s: String = pagedata[start..].to_string();
        while s.len() < page_size {
            s.push('\0');
        }
        s
    }
}

// Helper: set a frame's data in pagedata
fn set_frame_data(pagedata: &mut String, idx: usize, page_size: usize, data: &str) {
    let start = idx * page_size;
    let end = start + page_size;
    // Ensure pagedata is long enough
    while pagedata.len() < end {
        pagedata.push('\0');
    }
    let mut chars: Vec<char> = pagedata.chars().collect();
    let data_chars: Vec<char> = data.chars().collect();
    for i in 0..page_size {
        chars[start + i] = if i < data_chars.len() { data_chars[i] } else { '\0' };
    }
    *pagedata = chars.into_iter().collect();
}

pub fn init_buffer_pool(bm: &mut BM_BufferPool, page_file_name: &str, num_pages: i32, strategy: ReplacementStrategy, _strat_data: Option<Box<dyn std::any::Any>>) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let mut fh = SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    };
    let rc = crate::storage_mgr::open_page_file(page_file_name, &mut fh);
    if rc != RC::Ok {
        return rc;
    }
    let n = num_pages as usize;
    let strat_val = match &strategy {
        ReplacementStrategy::RsFifo => 0,
        ReplacementStrategy::RsLru => 1,
        ReplacementStrategy::RsClock => 2,
        ReplacementStrategy::RsLfu => 3,
        ReplacementStrategy::RsLruK => 4,
    };
    let bp = Bufferpool {
        num_read: 0,
        num_write: 0,
        total_pages: num_pages,
        updated_strategy: strat_val,
        free_space: num_pages,
        updated_order: vec![NO_PAGE; n],
        bitdirty: vec![false; n],
        fix_count: vec![0; n],
        access_time: vec![0; n],
        pagenum: vec![NO_PAGE; n],
        pagedata: std::iter::repeat('\0').take(n * page_size).collect(),
        fhl: fh,
    };
    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let bp = get_bp(bm);
    // Check for pinned pages
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] != 0 {
            return RC::BufferpoolInUse;
        }
    }
    // Write dirty pages
    let bp = get_bp_mut(bm);
    for j in 0..bp.total_pages as usize {
        if bp.bitdirty[j] {
            let page_num = bp.pagenum[j];
            let rc = crate::storage_mgr::ensure_capacity(page_num + 1, &mut bp.fhl);
            if rc != RC::Ok { return rc; }
            let data = get_frame_data(&bp.pagedata, j, page_size);
            let rc = crate::storage_mgr::write_block(page_num, &mut bp.fhl, &data);
            if rc != RC::Ok { return RC::WriteFailed; }
            bp.num_write += 1;
        }
    }
    let rc = crate::storage_mgr::close_page_file(&mut bp.fhl);
    if rc != RC::Ok { return RC::CloseFailed; }
    bm.mgmt_data = None;
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let bp = get_bp_mut(bm);
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let page_num = bp.pagenum[i];
            let file_name = bp.fhl.file_name.clone();
            // Replicate C behavior: open file, extend if needed, write at pagenum*PAGE_SIZE
            let data = get_frame_data(&bp.pagedata, i, page_size);
            {
                use std::fs::OpenOptions;
                use std::io::{Seek, SeekFrom, Write};
                let mut file = match OpenOptions::new().read(true).write(true).open(&file_name) {
                    Ok(f) => f,
                    Err(_) => return RC::WriteFailed,
                };
                let file_size = file.seek(SeekFrom::End(0)).unwrap_or(0);
                let required_size = ((page_num + 1) as u64) * (page_size as u64);
                if file_size < required_size {
                    file.seek(SeekFrom::Start(required_size - 1)).unwrap();
                    file.write_all(&[0u8]).unwrap();
                }
                file.seek(SeekFrom::Start((page_num as u64) * (page_size as u64))).unwrap();
                let mut buf = vec![0u8; page_size];
                for (idx, ch) in data.chars().enumerate() {
                    if idx >= page_size { break; }
                    buf[idx] = ch as u8;
                }
                if file.write_all(&buf).is_err() {
                    return RC::WriteFailed;
                }
            }
            bp.bitdirty[i] = false;
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn mark_dirty(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = get_bp_mut(bm);
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            bp.bitdirty[i] = true;
            break;
        }
    }
    RC::Ok
}

pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = get_bp_mut(bm);
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            if bp.fix_count[i] > 0 {
                bp.fix_count[i] -= 1;
            }
            break;
        }
    }
    RC::Ok
}

pub fn force_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = get_bp_mut(bm);
    let mut found = false;
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            bp.bitdirty[i] = false;
            bp.num_write += 1;
            found = true;
            break;
        }
    }
    if found { RC::Ok } else { RC::WriteFailed }
}

pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let bp = get_bp_mut(bm);
    let total_pages = bp.total_pages as usize;
    let is_void = bp.free_space == bp.total_pages;

    // Check if page already in pool
    if !is_void {
        let used = (bp.total_pages - bp.free_space) as usize;
        for i in 0..used {
            if bp.pagenum[i] == page_num {
                bp.fix_count[i] += 1;
                page.page_num = page_num;
                page.data = get_frame_data(&bp.pagedata, i, page_size);
                // LRU: move to end of updated_order
                if bp.updated_strategy == 1 {
                    let last_pos = used - 1;
                    if let Some(pos) = bp.updated_order[..=last_pos].iter().position(|&x| x == page_num) {
                        for j in pos..last_pos {
                            bp.updated_order[j] = bp.updated_order[j + 1];
                        }
                        bp.updated_order[last_pos] = page_num;
                    }
                }
                return RC::Ok;
            }
        }
    }

    // Free space available
    if is_void || bp.free_space > 0 {
        let mut page_handle = String::new();
        let rc = crate::storage_mgr::read_block(page_num, &mut bp.fhl, &mut page_handle);
        if rc != RC::Ok {
            // If read fails, still proceed (C code does)
        }
        let memory_address = (bp.total_pages - bp.free_space) as usize;
        set_frame_data(&mut bp.pagedata, memory_address, page_size, &page_handle);
        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;
        page.page_num = page_num;
        page.data = get_frame_data(&bp.pagedata, memory_address, page_size);
        return RC::Ok;
    }

    // Buffer pool full - need replacement
    let mut page_handle = String::new();
    let _rc = crate::storage_mgr::read_block(page_num, &mut bp.fhl, &mut page_handle);

    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        // FIFO or LRU
        let mut found = false;
        let mut memory_address = 0usize;
        let mut swap_location = 0usize;

        'outer: for j in 0..total_pages {
            let swap_page = bp.updated_order[j];
            for i in 0..total_pages {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    memory_address = i;
                    if bp.bitdirty[i] {
                        let pn = bp.pagenum[i];
                        let _ = crate::storage_mgr::ensure_capacity(pn + 1, &mut bp.fhl);
                        let data = get_frame_data(&bp.pagedata, i, page_size);
                        let _ = crate::storage_mgr::write_block(pn, &mut bp.fhl, &data);
                        bp.num_write += 1;
                    }
                    swap_location = j;
                    found = true;
                    break 'outer;
                }
            }
        }

        if !found {
            return RC::BufferpoolFull;
        }

        // Copy new page data
        set_frame_data(&mut bp.pagedata, memory_address, page_size, &page_handle);

        // Shift updated order
        for i in swap_location..(total_pages - 1) {
            bp.updated_order[i] = bp.updated_order[i + 1];
        }
        bp.updated_order[total_pages - 1] = page_num;

        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;

        page.page_num = page_num;
        page.data = get_frame_data(&bp.pagedata, memory_address, page_size);
        return RC::Ok;
    }

    RC::BufferpoolFull
}

fn shift_updated_order(start: i32, end: i32, bm: &mut BM_BufferPool, page_num: i32) {
    let bp = get_bp_mut(bm);
    for i in start as usize..end as usize {
        bp.updated_order[i] = bp.updated_order[i + 1];
    }
    bp.updated_order[end as usize] = page_num;
}

fn update_bufferpool_stats(bm: &mut BM_BufferPool, address: i32, page_num: i32) {
    let bp = get_bp_mut(bm);
    bp.pagenum[address as usize] = page_num;
    bp.num_read += 1;
    bp.fix_count[address as usize] += 1;
    bp.bitdirty[address as usize] = false;
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    let bp = get_bp(bm);
    bp.pagenum.clone()
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    let bp = get_bp(bm);
    bp.bitdirty.clone()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    let bp = get_bp(bm);
    bp.fix_count.clone()
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    let bp = get_bp(bm);
    bp.num_read
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    let bp = get_bp(bm);
    bp.num_write
}
