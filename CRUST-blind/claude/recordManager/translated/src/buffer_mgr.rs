use crate::dberror::{PAGE_SIZE, RC};
use crate::storage_mgr::{
    close_page_file, ensure_capacity, open_page_file, read_block, write_block, SM_FileHandle,
};
use std::cell::RefCell;

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

#[derive(Clone, Copy, PartialEq, Eq)]
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

fn page_size() -> usize {
    PAGE_SIZE as usize
}

fn make_zero_str(n: usize) -> String {
    // SAFETY: All-zero bytes form valid UTF-8.
    unsafe { String::from_utf8_unchecked(vec![0u8; n]) }
}

fn write_bytes_into(dst: &mut String, dst_off: usize, src: &[u8]) {
    // SAFETY: We are writing arbitrary bytes into the String's heap storage,
    // which may produce non-UTF-8 content. The internal buffer is sized
    // correctly by the caller.
    unsafe {
        let v = dst.as_mut_vec();
        if dst_off + src.len() > v.len() {
            v.resize(dst_off + src.len(), 0);
        }
        v[dst_off..dst_off + src.len()].copy_from_slice(src);
    }
}

fn read_bytes_from(src: &str, off: usize, len: usize) -> Vec<u8> {
    let bytes = src.as_bytes();
    let end = (off + len).min(bytes.len());
    let mut out = vec![0u8; len];
    if off < bytes.len() {
        let avail = end - off;
        out[..avail].copy_from_slice(&bytes[off..end]);
    }
    out
}

fn with_pool<R>(bm: &BM_BufferPool, f: impl FnOnce(&Bufferpool) -> R) -> Option<R> {
    bm.mgmt_data
        .as_ref()
        .and_then(|b| b.downcast_ref::<RefCell<Bufferpool>>())
        .map(|cell| f(&cell.borrow()))
}

fn with_pool_mut<R>(bm: &mut BM_BufferPool, f: impl FnOnce(&mut Bufferpool) -> R) -> Option<R> {
    bm.mgmt_data
        .as_mut()
        .and_then(|b| b.downcast_mut::<RefCell<Bufferpool>>())
        .map(|cell| f(cell.get_mut()))
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
    let rc = open_page_file(page_file_name, &mut fh);
    if rc != RC::Ok {
        return rc;
    }
    let n = num_pages as usize;
    let bp = Bufferpool {
        num_read: 0,
        num_write: 0,
        total_pages: num_pages,
        updated_strategy: strategy as i32,
        free_space: num_pages,
        updated_order: vec![NO_PAGE; n],
        bitdirty: vec![false; n],
        fix_count: vec![0; n],
        access_time: vec![0; n],
        pagenum: vec![NO_PAGE; n],
        pagedata: make_zero_str(n * page_size()),
        fhl: fh,
    };
    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(RefCell::new(bp)));
    RC::Ok
}

fn write_dirty_pages(bp: &mut Bufferpool) -> RC {
    for j in 0..(bp.total_pages as usize) {
        if bp.bitdirty[j] {
            let record_pointer = j * page_size();
            let rc = ensure_capacity(bp.pagenum[j] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let data = read_bytes_from(&bp.pagedata, record_pointer, page_size());
            let buf_str = unsafe { String::from_utf8_unchecked(data) };
            let rc = write_block(bp.pagenum[j], &mut bp.fhl, &buf_str);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let rc_pool = with_pool_mut(bm, |bp| {
        for i in 0..(bp.total_pages as usize) {
            if bp.fix_count[i] != 0 {
                return RC::BufferpoolInUse;
            }
        }
        let rc = write_dirty_pages(bp);
        if rc != RC::Ok {
            return rc;
        }
        if close_page_file(&mut bp.fhl) != RC::Ok {
            return RC::CloseFailed;
        }
        RC::Ok
    });
    let rc = match rc_pool {
        Some(r) => r,
        None => return RC::Error,
    };
    if rc != RC::Ok {
        return rc;
    }
    bm.mgmt_data = None;
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    with_pool_mut(bm, |bp| {
        for i in 0..(bp.total_pages as usize) {
            if bp.fix_count[i] == 0 && bp.bitdirty[i] {
                let record_pointer = i * page_size();
                let rc = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                if rc != RC::Ok {
                    return rc;
                }
                let data = read_bytes_from(&bp.pagedata, record_pointer, page_size());
                let buf_str = unsafe { String::from_utf8_unchecked(data) };
                let rc = write_block(bp.pagenum[i], &mut bp.fhl, &buf_str);
                if rc != RC::Ok {
                    return RC::WriteFailed;
                }
                bp.bitdirty[i] = false;
                bp.num_write += 1;
            }
        }
        RC::Ok
    })
    .unwrap_or(RC::Error)
}

/// Sync the local page handle's data back into the buffer pool's cache.
/// This emulates C's pointer-aliased page data semantics where edits to the
/// page handle directly modify the buffer pool's storage.
fn sync_page_to_cache(bp: &mut Bufferpool, page: &BM_PageHandle) {
    for i in 0..(bp.total_pages as usize) {
        if bp.pagenum[i] == page.page_num {
            let off = i * page_size();
            let bytes = page.data.as_bytes();
            let n = page_size().min(bytes.len());
            // SAFETY: write raw bytes into the pre-sized cache buffer.
            unsafe {
                let v = bp.pagedata.as_mut_vec();
                if v.len() < off + page_size() {
                    v.resize(off + page_size(), 0);
                }
                v[off..off + n].copy_from_slice(&bytes[..n]);
                if n < page_size() {
                    for j in off + n..off + page_size() {
                        v[j] = 0;
                    }
                }
            }
            break;
        }
    }
}

pub fn mark_dirty(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    with_pool_mut(bm, |bp| {
        sync_page_to_cache(bp, page);
        for i in 0..(bp.total_pages as usize) {
            if bp.pagenum[i] == page.page_num {
                bp.bitdirty[i] = true;
                break;
            }
        }
        RC::Ok
    })
    .unwrap_or(RC::Error)
}

pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    with_pool_mut(bm, |bp| {
        sync_page_to_cache(bp, page);
        for i in 0..(bp.total_pages as usize) {
            if bp.pagenum[i] == page.page_num {
                if bp.fix_count[i] > 0 {
                    bp.fix_count[i] -= 1;
                }
                break;
            }
        }
        RC::Ok
    })
    .unwrap_or(RC::Error)
}

pub fn force_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    with_pool_mut(bm, |bp| {
        sync_page_to_cache(bp, page);
        let mut found = false;
        for i in 0..(bp.total_pages as usize) {
            if bp.pagenum[i] == page.page_num {
                bp.bitdirty[i] = false;
                bp.num_write += 1;
                found = true;
                break;
            }
        }
        if found {
            RC::Ok
        } else {
            RC::WriteFailed
        }
    })
    .unwrap_or(RC::Error)
}

pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    with_pool_mut(bm, |bp| pin_page_inner(bp, page, page_num)).unwrap_or(RC::Error)
}

fn pin_page_inner(bp: &mut Bufferpool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let void_page = bp.free_space == bp.total_pages;
    // Search if the page is already in the buffer pool
    if !void_page {
        let used = (bp.total_pages - bp.free_space) as usize;
        for i in 0..used {
            if bp.pagenum[i] == page_num {
                page.page_num = page_num;
                bp.fix_count[i] += 1;
                let off = i * page_size();
                let data = read_bytes_from(&bp.pagedata, off, page_size());
                page.data = unsafe { String::from_utf8_unchecked(data) };

                if bp.updated_strategy == ReplacementStrategy::RsLru as i32 {
                    let last_pos = (bp.total_pages - bp.free_space - 1) as usize;
                    let mut swap_loc: Option<usize> = None;
                    for j in 0..=last_pos {
                        if bp.updated_order[j] == page_num {
                            swap_loc = Some(j);
                            break;
                        }
                    }
                    if let Some(swap) = swap_loc {
                        for k in swap..last_pos {
                            bp.updated_order[k] = bp.updated_order[k + 1];
                        }
                        bp.updated_order[last_pos] = page_num;
                    }
                }
                return RC::Ok;
            }
        }
    }

    // Free space available -- read into next slot
    if void_page || bp.free_space > 0 {
        let memory_address = (bp.total_pages - bp.free_space) as usize;
        let record_pointer = memory_address * page_size();
        let mut handle_buf = make_zero_str(page_size());
        let rc = read_block(page_num, &mut bp.fhl, &mut handle_buf);
        if rc != RC::Ok && rc != RC::ReadNonExistingPage {
            // Match C's behavior: ignore non-existing page and proceed.
        }
        // Copy into pagedata
        let bytes = handle_buf.as_bytes()[..page_size()].to_vec();
        write_bytes_into(&mut bp.pagedata, record_pointer, &bytes);
        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.numread_inc();
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;
        page.page_num = page_num;
        page.data = unsafe { String::from_utf8_unchecked(bytes) };
        return RC::Ok;
    }

    // Buffer pool is full: pick eviction victim
    let mut handle_buf = make_zero_str(page_size());
    let _ = read_block(page_num, &mut bp.fhl, &mut handle_buf);
    let mut found_idx: Option<usize> = None;
    let mut swap_location_in_order: Option<usize> = None;

    if bp.updated_strategy == ReplacementStrategy::RsFifo as i32
        || bp.updated_strategy == ReplacementStrategy::RsLru as i32
    {
        for j in 0..(bp.total_pages as usize) {
            let swap_page = bp.updated_order[j];
            for i in 0..(bp.total_pages as usize) {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    let record_pointer = i * page_size();
                    if bp.bitdirty[i] {
                        let _ = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                        let data = read_bytes_from(&bp.pagedata, record_pointer, page_size());
                        let buf_str = unsafe { String::from_utf8_unchecked(data) };
                        let _ = write_block(bp.pagenum[i], &mut bp.fhl, &buf_str);
                        bp.num_write += 1;
                    }
                    found_idx = Some(i);
                    swap_location_in_order = Some(j);
                    break;
                }
            }
            if found_idx.is_some() {
                break;
            }
        }
    }

    let memory_address = match found_idx {
        Some(i) => i,
        None => return RC::BufferpoolFull,
    };
    let record_pointer = memory_address * page_size();
    let new_bytes = handle_buf.as_bytes()[..page_size()].to_vec();
    write_bytes_into(&mut bp.pagedata, record_pointer, &new_bytes);

    if bp.updated_strategy == ReplacementStrategy::RsLru as i32
        || bp.updated_strategy == ReplacementStrategy::RsFifo as i32
    {
        let start = swap_location_in_order.unwrap_or(0);
        let end = (bp.total_pages - 1) as usize;
        for k in start..end {
            bp.updated_order[k] = bp.updated_order[k + 1];
        }
        bp.updated_order[end] = page_num;
    }

    bp.pagenum[memory_address] = page_num;
    bp.num_read += 1;
    bp.fix_count[memory_address] += 1;
    bp.bitdirty[memory_address] = false;

    page.page_num = page_num;
    page.data = unsafe { String::from_utf8_unchecked(new_bytes) };
    RC::Ok
}

impl Bufferpool {
    fn numread_inc(&mut self) {
        self.num_read += 1;
    }
}

#[allow(dead_code)]
fn shift_updated_order(start: i32, end: i32, bm: &mut BM_BufferPool, page_num: i32) {
    with_pool_mut(bm, |bp| {
        let s = start as usize;
        let e = end as usize;
        if e < bp.updated_order.len() {
            for i in s..e {
                bp.updated_order[i] = bp.updated_order[i + 1];
            }
            bp.updated_order[e] = page_num;
        }
    });
}

#[allow(dead_code)]
fn update_bufferpool_stats(bm: &mut BM_BufferPool, address: i32, page_num: i32) {
    with_pool_mut(bm, |bp| {
        let a = address as usize;
        if a < bp.pagenum.len() {
            bp.pagenum[a] = page_num;
            bp.num_read += 1;
            bp.fix_count[a] += 1;
            bp.bitdirty[a] = false;
        }
    });
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    with_pool(bm, |bp| {
        if bp.free_space == bp.total_pages {
            vec![NO_PAGE; bp.total_pages as usize]
        } else {
            bp.pagenum.clone()
        }
    })
    .unwrap_or_default()
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    with_pool(bm, |bp| bp.bitdirty.clone()).unwrap_or_default()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    with_pool(bm, |bp| {
        if bp.free_space == bp.total_pages {
            vec![0; bp.total_pages as usize]
        } else {
            bp.fix_count.clone()
        }
    })
    .unwrap_or_default()
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    with_pool(bm, |bp| bp.num_read).unwrap_or(0)
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    with_pool(bm, |bp| bp.num_write).unwrap_or(0)
}
