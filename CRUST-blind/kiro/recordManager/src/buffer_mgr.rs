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

/// Helper: read bytes from pagedata at a given offset
fn read_page_bytes(bp: &Bufferpool, slot: i32) -> Vec<u8> {
    let ps = crate::dberror::PAGE_SIZE as usize;
    let start = (slot as usize) * ps;
    let end = start + ps;
    let bytes = bp.pagedata.as_bytes();
    if end <= bytes.len() {
        bytes[start..end].to_vec()
    } else {
        let mut result = vec![0u8; ps];
        let avail = bytes.len().saturating_sub(start);
        if avail > 0 {
            result[..avail].copy_from_slice(&bytes[start..start + avail]);
        }
        result
    }
}

/// Helper: write bytes into pagedata at a given offset
fn write_page_bytes(bp: &mut Bufferpool, slot: i32, data: &[u8]) {
    let ps = crate::dberror::PAGE_SIZE as usize;
    let start = (slot as usize) * ps;
    let bytes = unsafe { bp.pagedata.as_bytes_mut() };
    let copy_len = data.len().min(ps);
    bytes[start..start + copy_len].copy_from_slice(&data[..copy_len]);
}

pub fn init_buffer_pool(
    bm: &mut BM_BufferPool,
    page_file_name: &str,
    num_pages: i32,
    strategy: ReplacementStrategy,
    _strat_data: Option<Box<dyn std::any::Any>>,
) -> RC {
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
    let ps = crate::dberror::PAGE_SIZE as usize;
    let strat_val = match &strategy {
        ReplacementStrategy::RsFifo => 0,
        ReplacementStrategy::RsLru => 1,
        ReplacementStrategy::RsClock => 2,
        ReplacementStrategy::RsLfu => 3,
        ReplacementStrategy::RsLruK => 4,
    };

    let bp = Bufferpool {
        total_pages: num_pages,
        pagedata: unsafe { String::from_utf8_unchecked(vec![0u8; n * ps]) },
        num_read: 0,
        num_write: 0,
        updated_order: vec![NO_PAGE; n],
        bitdirty: vec![false; n],
        free_space: num_pages,
        fhl: fh,
        pagenum: vec![NO_PAGE; n],
        fix_count: vec![0; n],
        access_time: vec![0; n],
        updated_strategy: strat_val,
    };

    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    // Check for pinned pages
    {
        let bp = get_bp(bm);
        for i in 0..bp.total_pages as usize {
            if bp.fix_count[i] != 0 {
                return RC::BufferpoolInUse;
            }
        }
    }
    // Write dirty pages
    let rc = write_dirty_pages(bm);
    if rc != RC::Ok {
        return rc;
    }
    // Close file
    {
        let bp = get_bp_mut(bm);
        let rc = crate::storage_mgr::close_page_file(&mut bp.fhl);
        if rc != RC::Ok {
            return RC::CloseFailed;
        }
    }
    bm.mgmt_data = None;
    RC::Ok
}

fn write_dirty_pages(bm: &mut BM_BufferPool) -> RC {
    let bp = get_bp_mut(bm);
    let ps = crate::dberror::PAGE_SIZE as usize;
    for j in 0..bp.total_pages as usize {
        if bp.bitdirty[j] {
            let page_num = bp.pagenum[j];
            let rc = crate::storage_mgr::ensure_capacity(page_num + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let data = read_page_bytes(bp, j as i32);
            let page_str = unsafe { String::from_utf8_unchecked(data) };
            let rc = crate::storage_mgr::write_block(page_num, &mut bp.fhl, &page_str);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let bp = get_bp_mut(bm);
    let ps = crate::dberror::PAGE_SIZE as usize;
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            // Write directly to file by name (matching C behavior which reopens file)
            let page_num = bp.pagenum[i];
            let data = read_page_bytes(bp, i as i32);

            // Open file, ensure size, write at correct offset
            let file_name = bp.fhl.file_name.clone();
            if let Ok(mut file) = std::fs::OpenOptions::new().read(true).write(true).open(&file_name) {
                use std::io::{Seek, SeekFrom, Write};
                let file_size = file.seek(SeekFrom::End(0)).unwrap_or(0);
                let required_size = ((page_num + 1) as u64) * (ps as u64);
                if file_size < required_size {
                    let _ = file.seek(SeekFrom::Start(required_size - 1));
                    let _ = file.write_all(&[0u8]);
                }
                let offset = (page_num as u64) * (ps as u64);
                if file.seek(SeekFrom::Start(offset)).is_ok() {
                    if file.write_all(&data).is_ok() {
                        bp.bitdirty[i] = false;
                        bp.num_write += 1;
                    } else {
                        return RC::WriteFailed;
                    }
                } else {
                    return RC::WriteFailed;
                }
            } else {
                return RC::WriteFailed;
            }
        }
    }
    RC::Ok
}

pub fn pin_page(
    bm: &mut BM_BufferPool,
    page: &mut BM_PageHandle,
    page_num: PageNumber,
) -> RC {
    let ps = crate::dberror::PAGE_SIZE as usize;
    let bp = get_bp_mut(bm);

    let void_page = bp.free_space == bp.total_pages;

    // Check if page already in pool
    if !void_page {
        let total_used = (bp.total_pages - bp.free_space) as usize;
        for i in 0..total_used {
            if bp.pagenum[i] == page_num {
                bp.fix_count[i] += 1;
                page.page_num = page_num;
                let data = read_page_bytes(bp, i as i32);
                page.data = unsafe { String::from_utf8_unchecked(data) };

                // LRU: move to end of order
                if bp.updated_strategy == 1 {
                    let last_pos = total_used - 1;
                    let mut swap_loc: i32 = -1;
                    for j in 0..=last_pos {
                        if bp.updated_order[j] == page_num {
                            swap_loc = j as i32;
                            break;
                        }
                    }
                    if swap_loc != -1 {
                        let sl = swap_loc as usize;
                        for k in sl..last_pos {
                            bp.updated_order[k] = bp.updated_order[k + 1];
                        }
                        bp.updated_order[last_pos] = page_num;
                    }
                }
                return RC::Ok;
            }
        }
    }

    // Free space available
    if void_page || bp.free_space > 0 {
        let mut page_handle = unsafe { String::from_utf8_unchecked(vec![0u8; ps]) };
        let rc = crate::storage_mgr::read_block(page_num, &mut bp.fhl, &mut page_handle);
        if rc != RC::Ok && rc != RC::Ok {
            // still try to proceed if read succeeded
        }
        let memory_address = (bp.total_pages - bp.free_space) as usize;
        write_page_bytes(bp, memory_address as i32, page_handle.as_bytes());
        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;
        page.page_num = page_num;
        let data = read_page_bytes(bp, memory_address as i32);
        page.data = unsafe { String::from_utf8_unchecked(data) };
        return RC::Ok;
    }

    // Buffer pool full - need replacement
    let mut page_handle = unsafe { String::from_utf8_unchecked(vec![0u8; ps]) };
    let rc = crate::storage_mgr::read_block(page_num, &mut bp.fhl, &mut page_handle);

    let mut updated_stra_found = false;
    let mut memory_address: usize = 0;
    let mut swap_location: usize = 0;

    // FIFO or LRU replacement
    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        let mut j = 0;
        while j < bp.total_pages as usize {
            let swap_page = bp.updated_order[j];
            let mut i = 0;
            while i < bp.total_pages as usize {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    memory_address = i;
                    if bp.bitdirty[i] {
                        let old_page_num = bp.pagenum[i];
                        let _ = crate::storage_mgr::ensure_capacity(old_page_num + 1, &mut bp.fhl);
                        let old_data = read_page_bytes(bp, i as i32);
                        let old_str = unsafe { String::from_utf8_unchecked(old_data) };
                        let _ = crate::storage_mgr::write_block(old_page_num, &mut bp.fhl, &old_str);
                        bp.num_write += 1;
                    }
                    swap_location = j;
                    updated_stra_found = true;
                    break;
                }
                i += 1;
            }
            if updated_stra_found {
                break;
            }
            j += 1;
        }
    }

    if !updated_stra_found {
        return RC::BufferpoolFull;
    }

    // Copy new page data
    write_page_bytes(bp, memory_address as i32, page_handle.as_bytes());

    // Shift updated order
    let end = (bp.total_pages - 1) as usize;
    for k in swap_location..end {
        bp.updated_order[k] = bp.updated_order[k + 1];
    }
    bp.updated_order[end] = page_num;

    // Update stats
    bp.pagenum[memory_address] = page_num;
    bp.num_read += 1;
    bp.fix_count[memory_address] += 1;
    bp.bitdirty[memory_address] = false;

    page.page_num = page_num;
    let data = read_page_bytes(bp, memory_address as i32);
    page.data = unsafe { String::from_utf8_unchecked(data) };
    RC::Ok
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

pub fn mark_dirty(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = get_bp_mut(bm);
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            if !bp.bitdirty[i] {
                bp.bitdirty[i] = true;
            }
            break;
        }
    }
    // Also write back page data from the page handle into the pool
    write_page_bytes(bp, find_slot(bp, page.page_num) as i32, page.data.as_bytes());
    RC::Ok
}

fn find_slot(bp: &Bufferpool, page_num: i32) -> usize {
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page_num {
            return i;
        }
    }
    0
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

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    let bp = get_bp(bm);
    if bp.free_space == bp.total_pages {
        return vec![];
    }
    bp.pagenum.clone()
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    let bp = get_bp(bm);
    bp.bitdirty.clone()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    let bp = get_bp(bm);
    if bp.free_space == bp.total_pages {
        return vec![0];
    }
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
