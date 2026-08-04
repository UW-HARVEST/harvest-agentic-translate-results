use crate::dberror::{RC, PAGE_SIZE};
use crate::storage_mgr::{
    SM_FileHandle, ensure_capacity, open_page_file, close_page_file, read_block, write_block,
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
    /// Per-frame storage: each frame is a contiguous slice of size PAGE_SIZE bytes,
    /// stored as a String of latin-1 (each char is 0..=255) to satisfy the existing API.
    pub pagedata: String,
    pub fhl: SM_FileHandle,
}

pub struct BM_PageHandle {
    pub page_num: PageNumber,
    pub data: String,
}

pub type PageNumber = i32;
pub const NO_PAGE: PageNumber = -1;

#[derive(Debug, Clone, Copy)]
pub enum ReplacementStrategy {
    RsFifo = 0,
    RsLru = 1,
    RsClock = 2,
    RsLfu = 3,
    RsLruK = 4,
}

fn strat_to_int(s: &ReplacementStrategy) -> i32 {
    match s {
        ReplacementStrategy::RsFifo => 0,
        ReplacementStrategy::RsLru => 1,
        ReplacementStrategy::RsClock => 2,
        ReplacementStrategy::RsLfu => 3,
        ReplacementStrategy::RsLruK => 4,
    }
}

pub struct BM_BufferPool {
    pub page_file: String,
    pub num_pages: i32,
    pub strategy: ReplacementStrategy,
    pub mgmt_data: Option<Box<dyn std::any::Any>>,
}

fn string_from_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn bytes_from_string(s: &str) -> Vec<u8> {
    s.chars().map(|c| c as u8).collect()
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
        updated_strategy: strat_to_int(&strategy),
        free_space: num_pages,
        updated_order: vec![NO_PAGE; n],
        bitdirty: vec![false; n],
        fix_count: vec![0; n],
        access_time: vec![0; n],
        pagenum: vec![NO_PAGE; n],
        pagedata: string_from_bytes(&vec![0u8; n * PAGE_SIZE as usize]),
        fhl: fh,
    };
    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

fn frame_string(bp: &Bufferpool, frame_idx: usize) -> String {
    let start = frame_idx * PAGE_SIZE as usize;
    bp.pagedata.chars().skip(start).take(PAGE_SIZE as usize).collect()
}

fn write_dirty_pages(bm: &mut BM_BufferPool) -> RC {
    let bp = match bm.mgmt_data.as_mut().and_then(|m| m.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };
    let total = bp.total_pages as usize;
    for j in 0..total {
        if bp.bitdirty[j] {
            let pn = bp.pagenum[j];
            if ensure_capacity(pn + 1, &mut bp.fhl) != RC::Ok {
                return RC::WriteFailed;
            }
            let frame_owned = frame_string(bp, j);
            let rc = write_block(pn, &mut bp.fhl, &frame_owned);
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
        let bp = match bm.mgmt_data.as_ref().and_then(|m| m.downcast_ref::<Bufferpool>()) {
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
    {
        let bp = bm.mgmt_data.as_mut().and_then(|m| m.downcast_mut::<Bufferpool>()).unwrap();
        if close_page_file(&mut bp.fhl) != RC::Ok {
            return RC::CloseFailed;
        }
    }
    bm.mgmt_data = None;
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let bp = match bm.mgmt_data.as_mut().and_then(|m| m.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };
    let total = bp.total_pages as usize;
    for i in 0..total {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let pn = bp.pagenum[i];
            if ensure_capacity(pn + 1, &mut bp.fhl) != RC::Ok {
                return RC::WriteFailed;
            }
            let frame_owned = frame_string(bp, i);
            let rc = write_block(pn, &mut bp.fhl, &frame_owned);
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
    let bp = match bm.mgmt_data.as_mut().and_then(|m| m.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            // Write back any modifications from the page handle into the buffer pool.
            let record_pointer = i * PAGE_SIZE as usize;
            let bytes = bytes_from_string(&page.data);
            let mut buf = vec![0u8; PAGE_SIZE as usize];
            let n = bytes.len().min(buf.len());
            buf[..n].copy_from_slice(&bytes[..n]);
            let new_data: String = string_from_bytes(&buf);
            // Replace the slice
            let mut bytes_all = bytes_from_string(&bp.pagedata);
            for k in 0..PAGE_SIZE as usize {
                bytes_all[record_pointer + k] = new_data.as_bytes().iter().nth(0).copied().unwrap_or(0);
            }
            // The above is wrong; use chars directly.
            let new_chars: Vec<char> = new_data.chars().collect();
            let mut all_chars: Vec<char> = bp.pagedata.chars().collect();
            for k in 0..PAGE_SIZE as usize {
                all_chars[record_pointer + k] = new_chars[k];
            }
            bp.pagedata = all_chars.into_iter().collect();
            bp.bitdirty[i] = true;
            break;
        }
    }
    RC::Ok
}

pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = match bm.mgmt_data.as_mut().and_then(|m| m.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            // If buffer pool is dirty, also write back data from page.data.
            if bp.bitdirty[i] {
                let record_pointer = i * PAGE_SIZE as usize;
                let new_chars: Vec<char> = page.data.chars().collect();
                let mut all_chars: Vec<char> = bp.pagedata.chars().collect();
                for k in 0..PAGE_SIZE as usize {
                    if k < new_chars.len() {
                        all_chars[record_pointer + k] = new_chars[k];
                    }
                }
                bp.pagedata = all_chars.into_iter().collect();
            }
            if bp.fix_count[i] > 0 {
                bp.fix_count[i] -= 1;
            }
            break;
        }
    }
    RC::Ok
}

pub fn force_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = match bm.mgmt_data.as_mut().and_then(|m| m.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut found = false;
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            let pn = bp.pagenum[i];
            if ensure_capacity(pn + 1, &mut bp.fhl) != RC::Ok {
                return RC::WriteFailed;
            }
            let frame_owned = frame_string(bp, i);
            let rc = write_block(pn, &mut bp.fhl, &frame_owned);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.bitdirty[i] = false;
            bp.num_write += 1;
            found = true;
            break;
        }
    }
    if found { RC::Ok } else { RC::WriteFailed }
}

pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let bp = match bm.mgmt_data.as_mut().and_then(|m| m.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };
    let total_pages = bp.total_pages;
    let void_page = bp.free_space == bp.total_pages;
    // Look for the page in the loaded frames
    if !void_page {
        let used = (bp.total_pages - bp.free_space) as usize;
        for i in 0..used {
            if bp.pagenum[i] == page_num {
                page.page_num = page_num;
                bp.fix_count[i] += 1;
                let record_pointer = i * PAGE_SIZE as usize;
                let frame_data: String = bp.pagedata
                    .chars()
                    .skip(record_pointer)
                    .take(PAGE_SIZE as usize)
                    .collect();
                page.data = frame_data;
                if bp.updated_strategy == 1 /* RS_LRU */ {
                    let last_position = (bp.total_pages - bp.free_space - 1) as usize;
                    let mut swap_location: i32 = -1;
                    for j in 0..=last_position {
                        if bp.updated_order[j] == page_num {
                            swap_location = j as i32;
                            break;
                        }
                    }
                    if swap_location != -1 {
                        let s = swap_location as usize;
                        for k in s..last_position {
                            bp.updated_order[k] = bp.updated_order[k + 1];
                        }
                        bp.updated_order[last_position] = page_num;
                    }
                }
                return RC::Ok;
            }
        }
    }

    // If buffer pool has free space, load the page.
    if void_page || bp.free_space > 0 {
        let mut page_buf = String::new();
        let read_rc = read_block(page_num, &mut bp.fhl, &mut page_buf);
        // C code allows a "no such page" read; data will be zeroed by ensureCapacity logic.
        // But actually, in C the file has the page if previously written. If readBlock fails,
        // we still proceed with empty buffer (matching C behavior with calloc'd page_handle).
        if read_rc != RC::Ok {
            page_buf = string_from_bytes(&vec![0u8; PAGE_SIZE as usize]);
        }
        let used_pages = (bp.total_pages - bp.free_space) as usize;
        let memory_address = used_pages;
        let record_pointer = memory_address * PAGE_SIZE as usize;
        let new_chars: Vec<char> = page_buf.chars().collect();
        let mut all_chars: Vec<char> = bp.pagedata.chars().collect();
        for k in 0..PAGE_SIZE as usize {
            let v = if k < new_chars.len() { new_chars[k] } else { '\0' };
            all_chars[record_pointer + k] = v;
        }
        bp.pagedata = all_chars.into_iter().collect();
        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;
        page.page_num = page_num;
        page.data = bp.pagedata
            .chars()
            .skip(record_pointer)
            .take(PAGE_SIZE as usize)
            .collect();
        return RC::Ok;
    }

    // Buffer pool full: replace based on strategy.
    let mut page_buf = String::new();
    let read_rc = read_block(page_num, &mut bp.fhl, &mut page_buf);
    if read_rc != RC::Ok {
        page_buf = string_from_bytes(&vec![0u8; PAGE_SIZE as usize]);
    }
    if bp.updated_strategy == 0 /* FIFO */ || bp.updated_strategy == 1 /* LRU */ {
        let mut memory_address: i32 = -1;
        let mut swap_location: i32 = -1;
        let mut found = false;
        for j in 0..total_pages as usize {
            let swap_page = bp.updated_order[j];
            for i in 0..total_pages as usize {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    memory_address = i as i32;
                    if bp.bitdirty[i] {
                        let pn = bp.pagenum[i];
                        let _ = ensure_capacity(pn + 1, &mut bp.fhl);
                        let frame_owned = frame_string(bp, i);
                        let _ = write_block(pn, &mut bp.fhl, &frame_owned);
                        bp.num_write += 1;
                    }
                    swap_location = j as i32;
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        if !found {
            return RC::BufferpoolFull;
        }
        let record_pointer = (memory_address as usize) * PAGE_SIZE as usize;
        let new_chars: Vec<char> = page_buf.chars().collect();
        let mut all_chars: Vec<char> = bp.pagedata.chars().collect();
        for k in 0..PAGE_SIZE as usize {
            let v = if k < new_chars.len() { new_chars[k] } else { '\0' };
            all_chars[record_pointer + k] = v;
        }
        bp.pagedata = all_chars.into_iter().collect();
        // Shift updated_order
        let s = swap_location as usize;
        let e = (total_pages - 1) as usize;
        for k in s..e {
            bp.updated_order[k] = bp.updated_order[k + 1];
        }
        bp.updated_order[e] = page_num;

        // UpdateBufferPoolStats
        bp.pagenum[memory_address as usize] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address as usize] += 1;
        bp.bitdirty[memory_address as usize] = false;
        page.page_num = page_num;
        page.data = bp.pagedata
            .chars()
            .skip(record_pointer)
            .take(PAGE_SIZE as usize)
            .collect();
        return RC::Ok;
    }
    RC::BufferpoolFull
}

#[allow(dead_code)]
fn shift_updated_order(start: i32, end: i32, _bm: &mut BM_BufferPool, _page_num: i32) {
    let _ = (start, end);
}

#[allow(dead_code)]
fn update_bufferpool_stats(_bm: &mut BM_BufferPool, _address: i32, _page_num: i32) {}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    let bp = match bm.mgmt_data.as_ref().and_then(|m| m.downcast_ref::<Bufferpool>()) {
        Some(b) => b,
        None => return Vec::new(),
    };
    bp.pagenum.clone()
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    let bp = match bm.mgmt_data.as_ref().and_then(|m| m.downcast_ref::<Bufferpool>()) {
        Some(b) => b,
        None => return Vec::new(),
    };
    bp.bitdirty.clone()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    let bp = match bm.mgmt_data.as_ref().and_then(|m| m.downcast_ref::<Bufferpool>()) {
        Some(b) => b,
        None => return Vec::new(),
    };
    if bp.free_space == bp.total_pages {
        // C returns &noFixes, a static int=0
        return vec![0];
    }
    bp.fix_count.clone()
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    bm.mgmt_data
        .as_ref()
        .and_then(|m| m.downcast_ref::<Bufferpool>())
        .map(|b| b.num_read)
        .unwrap_or(0)
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    bm.mgmt_data
        .as_ref()
        .and_then(|m| m.downcast_ref::<Bufferpool>())
        .map(|b| b.num_write)
        .unwrap_or(0)
}
