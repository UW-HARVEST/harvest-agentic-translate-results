use crate::dberror::{PAGE_SIZE, RC};
use crate::storage_mgr::{
    close_page_file, ensure_capacity, open_page_file, read_block, write_block, SM_FileHandle,
    SM_PageHandle,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

fn strategy_to_int(s: ReplacementStrategy) -> i32 {
    s as i32
}

fn get_pool_mut(bm: &mut BM_BufferPool) -> Option<&mut Bufferpool> {
    bm.mgmt_data.as_mut().and_then(|b| b.downcast_mut::<Bufferpool>())
}

fn get_pool(bm: &BM_BufferPool) -> Option<&Bufferpool> {
    bm.mgmt_data.as_ref().and_then(|b| b.downcast_ref::<Bufferpool>())
}

// Helpers to read/write bytes in pagedata (which is a String used as binary buffer).
fn pagedata_slice<'a>(pagedata: &'a str, start: usize, len: usize) -> Vec<u8> {
    let bytes = pagedata.as_bytes();
    let end = std::cmp::min(start + len, bytes.len());
    if start >= bytes.len() {
        vec![0u8; len]
    } else {
        let mut v = Vec::with_capacity(len);
        v.extend_from_slice(&bytes[start..end]);
        if v.len() < len {
            v.resize(len, 0);
        }
        v
    }
}

fn pagedata_write(pagedata: &mut String, start: usize, bytes: &[u8]) {
    unsafe {
        let v = pagedata.as_mut_vec();
        if start + bytes.len() > v.len() {
            v.resize(start + bytes.len(), 0);
        }
        v[start..start + bytes.len()].copy_from_slice(bytes);
    }
}

fn make_page_buffer(size: usize) -> String {
    let mut s = String::new();
    unsafe {
        s.as_mut_vec().resize(size, 0);
    }
    s
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
        updated_strategy: strategy_to_int(strategy),
        free_space: num_pages,
        updated_order: vec![NO_PAGE; n],
        bitdirty: vec![false; n],
        fix_count: vec![0; n],
        access_time: vec![0; n],
        pagenum: vec![NO_PAGE; n],
        pagedata: make_page_buffer(n * PAGE_SIZE as usize),
        fhl: fh,
    };

    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

fn write_dirty_pages(bm: &mut BM_BufferPool) -> RC {
    let bp = match get_pool_mut(bm) {
        Some(b) => b,
        None => return RC::Error,
    };
    let total = bp.total_pages as usize;
    for j in 0..total {
        if bp.bitdirty[j] {
            let page_num = bp.pagenum[j];
            let record_pointer = j * PAGE_SIZE as usize;
            // Build a SM_PageHandle from the data slice
            let page_data = unsafe {
                let v = bp.pagedata.as_mut_vec();
                String::from_utf8_unchecked(
                    v[record_pointer..record_pointer + PAGE_SIZE as usize].to_vec(),
                )
            };
            let rc = ensure_capacity(page_num + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let rc = write_block(page_num, &mut bp.fhl, &page_data);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
            bp.bitdirty[j] = false;
        }
    }
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    {
        let bp = match get_pool(bm) {
            Some(b) => b,
            None => return RC::Error,
        };
        for i in 0..bp.total_pages as usize {
            if bp.fix_count[i] != 0 {
                return RC::BufferpoolInUse;
            }
        }
    }
    let rc = write_dirty_pages(bm);
    if rc != RC::Ok {
        return rc;
    }
    if let Some(bp) = get_pool_mut(bm) {
        let close_rc = close_page_file(&mut bp.fhl);
        if close_rc != RC::Ok {
            return RC::CloseFailed;
        }
    }
    bm.mgmt_data = None;
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let bp = match get_pool_mut(bm) {
        Some(b) => b,
        None => return RC::Error,
    };
    let total = bp.total_pages as usize;
    for i in 0..total {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let page_num = bp.pagenum[i];
            let record_pointer = i * PAGE_SIZE as usize;
            let page_data = unsafe {
                let v = bp.pagedata.as_mut_vec();
                String::from_utf8_unchecked(
                    v[record_pointer..record_pointer + PAGE_SIZE as usize].to_vec(),
                )
            };
            let rc = ensure_capacity(page_num + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let rc = write_block(page_num, &mut bp.fhl, &page_data);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.bitdirty[i] = false;
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn mark_dirty(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = match get_pool_mut(bm) {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            // Sync data from page handle back into the buffer pool
            let bytes = page.data.as_bytes();
            let len = std::cmp::min(bytes.len(), PAGE_SIZE as usize);
            let bytes_owned = bytes[..len].to_vec();
            let mut padded = bytes_owned;
            if padded.len() < PAGE_SIZE as usize {
                padded.resize(PAGE_SIZE as usize, 0);
            }
            pagedata_write(&mut bp.pagedata, i * PAGE_SIZE as usize, &padded);
            bp.bitdirty[i] = true;
            break;
        }
    }
    RC::Ok
}

pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = match get_pool_mut(bm) {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut idx: i32 = -1;
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            idx = i as i32;
            break;
        }
    }
    if idx >= 0 {
        let i = idx as usize;
        // Sync any modifications back into the buffer pool slot.
        let bytes = page.data.as_bytes();
        let len = std::cmp::min(bytes.len(), PAGE_SIZE as usize);
        let mut padded = bytes[..len].to_vec();
        if padded.len() < PAGE_SIZE as usize {
            padded.resize(PAGE_SIZE as usize, 0);
        }
        pagedata_write(&mut bp.pagedata, i * PAGE_SIZE as usize, &padded);
        if bp.fix_count[i] > 0 {
            bp.fix_count[i] -= 1;
        }
    }
    RC::Ok
}

pub fn force_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = match get_pool_mut(bm) {
        Some(b) => b,
        None => return RC::Error,
    };
    let total = bp.total_pages as usize;
    let mut found = false;
    for i in 0..total {
        if bp.pagenum[i] == page.page_num {
            // sync back from page handle
            let bytes = page.data.as_bytes();
            let len = std::cmp::min(bytes.len(), PAGE_SIZE as usize);
            let mut padded = bytes[..len].to_vec();
            if padded.len() < PAGE_SIZE as usize {
                padded.resize(PAGE_SIZE as usize, 0);
            }
            pagedata_write(&mut bp.pagedata, i * PAGE_SIZE as usize, &padded);
            let record_pointer = i * PAGE_SIZE as usize;
            let page_data = unsafe {
                let v = bp.pagedata.as_mut_vec();
                String::from_utf8_unchecked(
                    v[record_pointer..record_pointer + PAGE_SIZE as usize].to_vec(),
                )
            };
            let rc = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let rc = write_block(bp.pagenum[i], &mut bp.fhl, &page_data);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
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
}

pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let bp = match get_pool_mut(bm) {
        Some(b) => b,
        None => return RC::Error,
    };
    let total_pages = bp.total_pages as usize;
    let void_page = bp.free_space == bp.total_pages;

    // 1. Search for already-pinned page (only if there's at least one pinned slot)
    if !void_page {
        let used = (bp.total_pages - bp.free_space) as usize;
        for i in 0..used {
            if bp.pagenum[i] == page_num {
                let memory_address = i;
                bp.fix_count[memory_address] += 1;
                let record_pointer = memory_address * PAGE_SIZE as usize;
                let data_bytes =
                    pagedata_slice(&bp.pagedata, record_pointer, PAGE_SIZE as usize);
                page.page_num = page_num;
                page.data = unsafe { String::from_utf8_unchecked(data_bytes) };

                if bp.updated_strategy == ReplacementStrategy::RsLru as i32 {
                    let last_position = (bp.total_pages - bp.free_space - 1) as i32;
                    let mut swap_location: i32 = -1;
                    for j in 0..=last_position {
                        if bp.updated_order[j as usize] == page_num {
                            swap_location = j;
                            break;
                        }
                    }
                    if swap_location != -1 {
                        let from = swap_location as usize;
                        let to = last_position as usize;
                        for k in from..to {
                            bp.updated_order[k] = bp.updated_order[k + 1];
                        }
                        bp.updated_order[to] = page_num;
                    }
                }
                return RC::Ok;
            }
        }
    }

    // 2. Free space available: load page into free slot
    if void_page || bp.free_space > 0 {
        let memory_address = (bp.total_pages - bp.free_space) as usize;
        let record_pointer = memory_address * PAGE_SIZE as usize;
        let mut tmp = make_page_buffer(PAGE_SIZE as usize);
        let rc = read_block(page_num, &mut bp.fhl, &mut tmp);
        if rc == RC::Ok {
            // Successfully read - copy into pagedata
            let bytes = tmp.as_bytes().to_vec();
            pagedata_write(&mut bp.pagedata, record_pointer, &bytes);
        } else {
            // Page does not exist on disk yet. Initialize with zeros so
            // tests that pin new pages and write to them work correctly.
            let zeros = vec![0u8; PAGE_SIZE as usize];
            pagedata_write(&mut bp.pagedata, record_pointer, &zeros);
        }
        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;

        let data_bytes = pagedata_slice(&bp.pagedata, record_pointer, PAGE_SIZE as usize);
        page.page_num = page_num;
        page.data = unsafe { String::from_utf8_unchecked(data_bytes) };
        return RC::Ok;
    }

    // 3. Buffer pool is full: evict using FIFO/LRU
    if bp.free_space == 0
        && (bp.updated_strategy == ReplacementStrategy::RsFifo as i32
            || bp.updated_strategy == ReplacementStrategy::RsLru as i32)
    {
        let mut memory_address: i32 = -1;
        let mut swap_location: i32 = -1;
        // updatedOrder is a queue; iterate and find first unfixed page.
        for j in 0..total_pages {
            let candidate_page = bp.updated_order[j];
            for i in 0..total_pages {
                if bp.pagenum[i] == candidate_page && bp.fix_count[i] == 0 {
                    memory_address = i as i32;
                    swap_location = j as i32;
                    break;
                }
            }
            if memory_address != -1 {
                break;
            }
        }
        if memory_address == -1 {
            return RC::BufferpoolFull;
        }
        let i = memory_address as usize;
        let record_pointer = i * PAGE_SIZE as usize;
        // If dirty, write back to disk
        if bp.bitdirty[i] {
            let page_data = unsafe {
                let v = bp.pagedata.as_mut_vec();
                String::from_utf8_unchecked(
                    v[record_pointer..record_pointer + PAGE_SIZE as usize].to_vec(),
                )
            };
            let _ = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
            let _ = write_block(bp.pagenum[i], &mut bp.fhl, &page_data);
            bp.num_write += 1;
        }
        // Read new page from disk into evicted slot (or zeros if not exists)
        let mut tmp = make_page_buffer(PAGE_SIZE as usize);
        let rc = read_block(page_num, &mut bp.fhl, &mut tmp);
        let new_bytes = if rc == RC::Ok {
            tmp.as_bytes().to_vec()
        } else {
            vec![0u8; PAGE_SIZE as usize]
        };
        pagedata_write(&mut bp.pagedata, record_pointer, &new_bytes);

        // Shift the updated_order if FIFO/LRU
        let s = swap_location;
        let e = total_pages as i32 - 1;
        for k in s..e {
            bp.updated_order[k as usize] = bp.updated_order[(k + 1) as usize];
        }
        bp.updated_order[e as usize] = page_num;

        // Update bookkeeping
        bp.pagenum[i] = page_num;
        bp.num_read += 1;
        bp.fix_count[i] += 1;
        bp.bitdirty[i] = false;

        let data_bytes = pagedata_slice(&bp.pagedata, record_pointer, PAGE_SIZE as usize);
        page.page_num = page_num;
        page.data = unsafe { String::from_utf8_unchecked(data_bytes) };
        return RC::Ok;
    }

    RC::BufferpoolFull
}

fn shift_updated_order(_start: i32, _end: i32, _bm: &mut BM_BufferPool, _page_num: i32) {
    // Inlined into pin_page; no-op helper kept for signature compatibility.
}

fn update_bufferpool_stats(_bm: &mut BM_BufferPool, _address: i32, _page_num: i32) {
    // Inlined into pin_page; no-op helper kept for signature compatibility.
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    match get_pool(bm) {
        Some(bp) => {
            if bp.free_space == bp.total_pages {
                vec![NO_PAGE; bp.total_pages as usize]
            } else {
                bp.pagenum.clone()
            }
        }
        None => vec![NO_PAGE; bm.num_pages as usize],
    }
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    match get_pool(bm) {
        Some(bp) => bp.bitdirty.clone(),
        None => vec![false; bm.num_pages as usize],
    }
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    match get_pool(bm) {
        Some(bp) => {
            if bp.free_space == bp.total_pages {
                vec![0; bp.total_pages as usize]
            } else {
                bp.fix_count.clone()
            }
        }
        None => vec![0; bm.num_pages as usize],
    }
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    match get_pool(bm) {
        Some(bp) => bp.num_read,
        None => 0,
    }
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    match get_pool(bm) {
        Some(bp) => bp.num_write,
        None => 0,
    }
}
