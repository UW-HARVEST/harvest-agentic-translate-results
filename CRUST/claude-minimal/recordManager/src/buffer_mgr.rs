use crate::dberror::{PAGE_SIZE, RC};
use crate::storage_mgr::{
    close_page_file, ensure_capacity, open_page_file, read_block, write_block, SM_FileHandle,
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

fn strategy_to_i32(strategy: &ReplacementStrategy) -> i32 {
    match strategy {
        ReplacementStrategy::RsFifo => 0,
        ReplacementStrategy::RsLru => 1,
        ReplacementStrategy::RsClock => 2,
        ReplacementStrategy::RsLfu => 3,
        ReplacementStrategy::RsLruK => 4,
    }
}

fn clone_strategy(strategy: &ReplacementStrategy) -> ReplacementStrategy {
    match strategy {
        ReplacementStrategy::RsFifo => ReplacementStrategy::RsFifo,
        ReplacementStrategy::RsLru => ReplacementStrategy::RsLru,
        ReplacementStrategy::RsClock => ReplacementStrategy::RsClock,
        ReplacementStrategy::RsLfu => ReplacementStrategy::RsLfu,
        ReplacementStrategy::RsLruK => ReplacementStrategy::RsLruK,
    }
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
    let strategy_i = strategy_to_i32(&strategy);
    let bp = Bufferpool {
        num_read: 0,
        num_write: 0,
        total_pages: num_pages,
        updated_strategy: strategy_i,
        free_space: num_pages,
        updated_order: vec![NO_PAGE; num_pages as usize],
        bitdirty: vec![false; num_pages as usize],
        fix_count: vec![0; num_pages as usize],
        access_time: vec![0; num_pages as usize],
        pagenum: vec![NO_PAGE; num_pages as usize],
        pagedata: String::from_utf8(vec![0u8; (num_pages * PAGE_SIZE) as usize]).unwrap_or_default(),
        fhl: fh,
    };
    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

fn write_dirty_pages(bp: &mut Bufferpool) -> RC {
    for j in 0..bp.total_pages as usize {
        if bp.bitdirty[j] {
            let rc = ensure_capacity(bp.pagenum[j] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            // Slice out the page data for this frame.
            let page_start = j * PAGE_SIZE as usize;
            let page_end = page_start + PAGE_SIZE as usize;
            let bytes = bp.pagedata.as_bytes();
            let slice = &bytes[page_start..page_end.min(bytes.len())];
            let page_str = String::from_utf8_lossy(slice).into_owned();
            let rc = write_block(bp.pagenum[j], &mut bp.fhl, &page_str);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] != 0 {
            return RC::BufferpoolInUse;
        }
    }
    let rc = write_dirty_pages(bp);
    if rc != RC::Ok {
        return rc;
    }
    let rc = close_page_file(&mut bp.fhl);
    if rc != RC::Ok {
        return RC::CloseFailed;
    }
    bm.mgmt_data = None;
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let page_start = i * PAGE_SIZE as usize;
            let page_end = page_start + PAGE_SIZE as usize;
            let bytes = bp.pagedata.as_bytes();
            let slice = &bytes[page_start..page_end.min(bytes.len())];
            let page_str = String::from_utf8_lossy(slice).into_owned();
            let rc = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let rc = write_block(bp.pagenum[i], &mut bp.fhl, &page_str);
            if rc != RC::Ok {
                return rc;
            }
            bp.bitdirty[i] = false;
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn mark_dirty(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            bp.bitdirty[i] = true;
            return RC::Ok;
        }
    }
    RC::Ok
}

pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            if bp.fix_count[i] > 0 {
                bp.fix_count[i] -= 1;
            }
            return RC::Ok;
        }
    }
    RC::Ok
}

pub fn force_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut found = false;
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            let page_start = i * PAGE_SIZE as usize;
            let page_end = page_start + PAGE_SIZE as usize;
            let bytes = bp.pagedata.as_bytes();
            let slice = &bytes[page_start..page_end.min(bytes.len())];
            let page_str = String::from_utf8_lossy(slice).into_owned();
            let rc = write_block(bp.pagenum[i], &mut bp.fhl, &page_str);
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
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };

    // Check if page is already in pool.
    let used = (bp.total_pages - bp.free_space) as usize;
    for i in 0..used {
        if bp.pagenum[i] == page_num {
            page.page_num = page_num;
            bp.fix_count[i] += 1;
            // Copy the page data into the handle.
            let page_start = i * PAGE_SIZE as usize;
            let page_end = page_start + PAGE_SIZE as usize;
            let bytes = bp.pagedata.as_bytes();
            let slice = &bytes[page_start..page_end.min(bytes.len())];
            page.data = String::from_utf8_lossy(slice).into_owned();

            // LRU bookkeeping
            if bp.updated_strategy == 1 {
                let last_pos = (bp.total_pages - bp.free_space - 1) as usize;
                let mut swap = -1i32;
                for j in 0..=last_pos {
                    if bp.updated_order[j] == page_num {
                        swap = j as i32;
                        break;
                    }
                }
                if swap >= 0 && (swap as usize) < last_pos {
                    let s = swap as usize;
                    for k in s..last_pos {
                        bp.updated_order[k] = bp.updated_order[k + 1];
                    }
                    bp.updated_order[last_pos] = page_num;
                }
            }
            return RC::Ok;
        }
    }

    // Not present; check for free slot.
    if bp.free_space > 0 {
        let mem_addr = (bp.total_pages - bp.free_space) as usize;
        // Read the page from disk
        let mut buf = String::from_utf8(vec![0u8; PAGE_SIZE as usize]).unwrap_or_default();
        let _ = read_block(page_num, &mut bp.fhl, &mut buf);
        // Copy bytes into pool
        let page_start = mem_addr * PAGE_SIZE as usize;
        let mut bytes = std::mem::take(&mut bp.pagedata).into_bytes();
        let buf_bytes = buf.as_bytes();
        let n = (PAGE_SIZE as usize).min(buf_bytes.len());
        for k in 0..n {
            bytes[page_start + k] = buf_bytes[k];
        }
        // pad if buf shorter
        for k in n..PAGE_SIZE as usize {
            if page_start + k < bytes.len() {
                bytes[page_start + k] = 0;
            }
        }
        bp.pagedata = String::from_utf8_lossy(&bytes).into_owned();

        bp.free_space -= 1;
        bp.updated_order[mem_addr] = page_num;
        bp.pagenum[mem_addr] = page_num;
        bp.num_read += 1;
        bp.fix_count[mem_addr] += 1;
        bp.bitdirty[mem_addr] = false;
        page.page_num = page_num;
        let bytes = bp.pagedata.as_bytes();
        let slice = &bytes[page_start..page_start + PAGE_SIZE as usize];
        page.data = String::from_utf8_lossy(slice).into_owned();
        return RC::Ok;
    }

    // Buffer pool full: replacement.
    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        let mut found = false;
        let mut mem_addr = 0usize;
        let mut swap_loc: i32 = -1;
        for j in 0..bp.total_pages as usize {
            let swap_page = bp.updated_order[j];
            for i in 0..bp.total_pages as usize {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    mem_addr = i;
                    if bp.bitdirty[i] {
                        let page_start = i * PAGE_SIZE as usize;
                        let bytes = bp.pagedata.as_bytes();
                        let slice = &bytes[page_start..page_start + PAGE_SIZE as usize];
                        let page_str = String::from_utf8_lossy(slice).into_owned();
                        let _ = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                        let _ = write_block(bp.pagenum[i], &mut bp.fhl, &page_str);
                        bp.num_write += 1;
                    }
                    swap_loc = j as i32;
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
        // Read the page from disk
        let mut buf = String::from_utf8(vec![0u8; PAGE_SIZE as usize]).unwrap_or_default();
        let _ = read_block(page_num, &mut bp.fhl, &mut buf);
        let page_start = mem_addr * PAGE_SIZE as usize;
        let mut bytes = std::mem::take(&mut bp.pagedata).into_bytes();
        let buf_bytes = buf.as_bytes();
        let n = (PAGE_SIZE as usize).min(buf_bytes.len());
        for k in 0..n {
            bytes[page_start + k] = buf_bytes[k];
        }
        for k in n..PAGE_SIZE as usize {
            if page_start + k < bytes.len() {
                bytes[page_start + k] = 0;
            }
        }
        bp.pagedata = String::from_utf8_lossy(&bytes).into_owned();

        // Update order
        if swap_loc >= 0 {
            let s = swap_loc as usize;
            for k in s..(bp.total_pages as usize - 1) {
                bp.updated_order[k] = bp.updated_order[k + 1];
            }
            bp.updated_order[bp.total_pages as usize - 1] = page_num;
        }
        bp.pagenum[mem_addr] = page_num;
        bp.num_read += 1;
        bp.fix_count[mem_addr] += 1;
        bp.bitdirty[mem_addr] = false;
        page.page_num = page_num;
        let bytes = bp.pagedata.as_bytes();
        let slice = &bytes[page_start..page_start + PAGE_SIZE as usize];
        page.data = String::from_utf8_lossy(slice).into_owned();
        return RC::Ok;
    }

    RC::BufferpoolFull
}

fn shift_updated_order(start: i32, end: i32, bm: &mut BM_BufferPool, page_num: i32) {
    if let Some(mgmt) = bm.mgmt_data.as_mut() {
        if let Some(bp) = mgmt.downcast_mut::<Bufferpool>() {
            for i in start as usize..end as usize {
                bp.updated_order[i] = bp.updated_order[i + 1];
            }
            if (end as usize) < bp.updated_order.len() {
                bp.updated_order[end as usize] = page_num;
            }
        }
    }
}

fn update_bufferpool_stats(bm: &mut BM_BufferPool, address: i32, page_num: i32) {
    if let Some(mgmt) = bm.mgmt_data.as_mut() {
        if let Some(bp) = mgmt.downcast_mut::<Bufferpool>() {
            let i = address as usize;
            bp.pagenum[i] = page_num;
            bp.num_read += 1;
            bp.fix_count[i] += 1;
            bp.bitdirty[i] = false;
        }
    }
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    if let Some(mgmt) = bm.mgmt_data.as_ref() {
        if let Some(bp) = mgmt.downcast_ref::<Bufferpool>() {
            return bp.pagenum.clone();
        }
    }
    vec![NO_PAGE; bm.num_pages as usize]
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    if let Some(mgmt) = bm.mgmt_data.as_ref() {
        if let Some(bp) = mgmt.downcast_ref::<Bufferpool>() {
            return bp.bitdirty.clone();
        }
    }
    vec![false; bm.num_pages as usize]
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    if let Some(mgmt) = bm.mgmt_data.as_ref() {
        if let Some(bp) = mgmt.downcast_ref::<Bufferpool>() {
            return bp.fix_count.clone();
        }
    }
    vec![0; bm.num_pages as usize]
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    if let Some(mgmt) = bm.mgmt_data.as_ref() {
        if let Some(bp) = mgmt.downcast_ref::<Bufferpool>() {
            return bp.num_read;
        }
    }
    0
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    if let Some(mgmt) = bm.mgmt_data.as_ref() {
        if let Some(bp) = mgmt.downcast_ref::<Bufferpool>() {
            return bp.num_write;
        }
    }
    0
}

#[allow(dead_code)]
fn _retain_helpers() {
    let _ = clone_strategy;
    let _ = shift_updated_order;
    let _ = update_bufferpool_stats;
}
