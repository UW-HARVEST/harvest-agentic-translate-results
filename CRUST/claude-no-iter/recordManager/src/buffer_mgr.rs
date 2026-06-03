use crate::dberror::RC;
use crate::storage_mgr::{
    bytes_to_page_string, close_page_file, ensure_capacity, open_page_file, page_string_to_bytes,
    read_block, write_block, SM_FileHandle, PAGE_SIZE,
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

#[derive(Clone, Copy)]
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

fn strategy_to_int(s: &ReplacementStrategy) -> i32 {
    match s {
        ReplacementStrategy::RsFifo => 0,
        ReplacementStrategy::RsLru => 1,
        ReplacementStrategy::RsClock => 2,
        ReplacementStrategy::RsLfu => 3,
        ReplacementStrategy::RsLruK => 4,
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
    let n = num_pages as usize;
    let bp = Bufferpool {
        num_read: 0,
        num_write: 0,
        total_pages: num_pages,
        updated_strategy: strategy_to_int(&strategy),
        free_space: num_pages,
        updated_order: vec![NO_PAGE; n],
        bitdirty: vec![false; n],
        fix_count: vec![0; n],
        access_time: vec![0; n],
        pagenum: vec![NO_PAGE; n],
        pagedata: vec!['\0'; n * PAGE_SIZE].into_iter().collect(),
        fhl: fh,
    };
    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

fn with_bp_mut<R>(bm: &mut BM_BufferPool, f: impl FnOnce(&mut Bufferpool) -> R) -> Option<R> {
    if let Some(boxed) = bm.mgmt_data.as_mut() {
        if let Some(bp) = boxed.downcast_mut::<Bufferpool>() {
            return Some(f(bp));
        }
    }
    None
}

fn with_bp<R>(bm: &BM_BufferPool, f: impl FnOnce(&Bufferpool) -> R) -> Option<R> {
    if let Some(boxed) = bm.mgmt_data.as_ref() {
        if let Some(bp) = boxed.downcast_ref::<Bufferpool>() {
            return Some(f(bp));
        }
    }
    None
}

fn write_dirty_pages(bp: &mut Bufferpool) -> RC {
    for j in 0..bp.total_pages as usize {
        if bp.bitdirty[j] {
            let record_pointer = j * PAGE_SIZE;
            let rc = ensure_capacity(bp.pagenum[j] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let page_str = extract_page(&bp.pagedata, record_pointer);
            let rc = write_block(bp.pagenum[j], &mut bp.fhl, &page_str);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
            bp.bitdirty[j] = false;
        }
    }
    RC::Ok
}

fn extract_page(pagedata: &str, offset: usize) -> String {
    let bytes: Vec<u8> = pagedata.chars().map(|c| c as u8).collect();
    let end = (offset + PAGE_SIZE).min(bytes.len());
    let slice = if offset < bytes.len() {
        &bytes[offset..end]
    } else {
        &[]
    };
    let mut result: Vec<u8> = slice.to_vec();
    while result.len() < PAGE_SIZE {
        result.push(0);
    }
    bytes_to_page_string(&result)
}

fn replace_page_section(pagedata: &mut String, offset: usize, new_data: &str) {
    let mut bytes: Vec<u8> = pagedata.chars().map(|c| c as u8).collect();
    while bytes.len() < offset + PAGE_SIZE {
        bytes.push(0);
    }
    let new_bytes = page_string_to_bytes(new_data);
    bytes[offset..offset + PAGE_SIZE].copy_from_slice(&new_bytes);
    *pagedata = bytes_to_page_string(&bytes);
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let rc = with_bp_mut(bm, |bp| {
        for i in 0..bp.total_pages as usize {
            if bp.fix_count[i] != 0 {
                return RC::BufferpoolInUse;
            }
        }
        let rc = write_dirty_pages(bp);
        if rc != RC::Ok {
            return rc;
        }
        let close_rc = close_page_file(&mut bp.fhl);
        if close_rc != RC::Ok {
            return RC::CloseFailed;
        }
        RC::Ok
    });
    let rc = rc.unwrap_or(RC::Error);
    if rc == RC::Ok {
        bm.mgmt_data = None;
    }
    rc
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let res = with_bp_mut(bm, |bp| {
        for i in 0..bp.total_pages as usize {
            if bp.fix_count[i] == 0 && bp.bitdirty[i] {
                let record_pointer = i * PAGE_SIZE;
                let rc = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                if rc != RC::Ok {
                    return rc;
                }
                let page_str = extract_page(&bp.pagedata, record_pointer);
                let rc = write_block(bp.pagenum[i], &mut bp.fhl, &page_str);
                if rc != RC::Ok {
                    return RC::WriteFailed;
                }
                bp.bitdirty[i] = false;
                bp.num_write += 1;
            }
        }
        RC::Ok
    });
    res.unwrap_or(RC::Error)
}

pub fn mark_dirty(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let res = with_bp_mut(bm, |bp| {
        for i in 0..bp.total_pages as usize {
            if bp.pagenum[i] == page.page_num {
                bp.bitdirty[i] = true;
                // Write back the page data from the page handle into the buffer
                let offset = i * PAGE_SIZE;
                replace_page_section(&mut bp.pagedata, offset, &page.data);
                break;
            }
        }
        RC::Ok
    });
    res.unwrap_or(RC::Error)
}

pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let res = with_bp_mut(bm, |bp| {
        // Always copy back the page data in case it was modified in the handle
        for i in 0..bp.total_pages as usize {
            if bp.pagenum[i] == page.page_num {
                if bp.bitdirty[i] {
                    let offset = i * PAGE_SIZE;
                    replace_page_section(&mut bp.pagedata, offset, &page.data);
                }
                if bp.fix_count[i] > 0 {
                    bp.fix_count[i] -= 1;
                }
                break;
            }
        }
        RC::Ok
    });
    res.unwrap_or(RC::Error)
}

pub fn force_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let res = with_bp_mut(bm, |bp| {
        let mut found = false;
        for i in 0..bp.total_pages as usize {
            if bp.pagenum[i] == page.page_num {
                let record_pointer = i * PAGE_SIZE;
                let rc = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                if rc != RC::Ok {
                    return rc;
                }
                let page_str = extract_page(&bp.pagedata, record_pointer);
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
    });
    res.unwrap_or(RC::Error)
}

pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let res = with_bp_mut(bm, |bp| {
        // Check if page is already in buffer pool
        let used = (bp.total_pages - bp.free_space) as usize;
        for i in 0..used {
            if bp.pagenum[i] == page_num {
                page.page_num = page_num;
                bp.fix_count[i] += 1;
                let offset = i * PAGE_SIZE;
                page.data = extract_page(&bp.pagedata, offset);
                if bp.updated_strategy == 1 {
                    // RS_LRU
                    let last_pos = bp.total_pages - bp.free_space - 1;
                    let mut swap_location: i32 = -1;
                    for j in 0..=last_pos {
                        if bp.updated_order[j as usize] == page_num {
                            swap_location = j;
                            break;
                        }
                    }
                    if swap_location != -1 {
                        for k in (swap_location as usize)..(last_pos as usize) {
                            bp.updated_order[k] = bp.updated_order[k + 1];
                        }
                        bp.updated_order[last_pos as usize] = page_num;
                    }
                }
                return RC::Ok;
            }
        }
        // Page not in pool. Need to load it.
        let mut page_handle: String = bytes_to_page_string(&vec![0u8; PAGE_SIZE]);
        // Ensure capacity for reading
        let needed = page_num + 1;
        if needed > bp.fhl.total_num_pages {
            let _ = ensure_capacity(needed, &mut bp.fhl);
        }
        let _ = read_block(page_num, &mut bp.fhl, &mut page_handle);

        if bp.free_space > 0 {
            // Use a free slot
            let memory_address = (bp.total_pages - bp.free_space) as usize;
            let record_pointer = memory_address * PAGE_SIZE;
            replace_page_section(&mut bp.pagedata, record_pointer, &page_handle);
            bp.free_space -= 1;
            bp.updated_order[memory_address] = page_num;
            bp.pagenum[memory_address] = page_num;
            bp.num_read += 1;
            bp.fix_count[memory_address] += 1;
            bp.bitdirty[memory_address] = false;
            page.page_num = page_num;
            page.data = extract_page(&bp.pagedata, record_pointer);
            return RC::Ok;
        }

        // Buffer is full - need to evict
        let mut updated_stra_found = false;
        let mut memory_address: usize = 0;
        let mut swap_location: i32 = -1;
        if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
            // FIFO or LRU
            'outer: for j in 0..bp.total_pages {
                let swap_page = bp.updated_order[j as usize];
                for i in 0..bp.total_pages as usize {
                    if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                        memory_address = i;
                        let record_pointer = i * PAGE_SIZE;
                        if bp.bitdirty[i] {
                            let _ = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                            let page_str = extract_page(&bp.pagedata, record_pointer);
                            let _ = write_block(bp.pagenum[i], &mut bp.fhl, &page_str);
                            bp.num_write += 1;
                        }
                        swap_location = j;
                        updated_stra_found = true;
                        break 'outer;
                    }
                }
            }
        }
        if !updated_stra_found {
            return RC::BufferpoolFull;
        }
        let record_pointer = memory_address * PAGE_SIZE;
        replace_page_section(&mut bp.pagedata, record_pointer, &page_handle);
        if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
            let total_pages_minus_one = bp.total_pages - 1;
            for k in (swap_location as usize)..(total_pages_minus_one as usize) {
                bp.updated_order[k] = bp.updated_order[k + 1];
            }
            bp.updated_order[total_pages_minus_one as usize] = page_num;
        }
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;
        page.page_num = page_num;
        page.data = extract_page(&bp.pagedata, record_pointer);
        RC::Ok
    });
    res.unwrap_or(RC::Error)
}

fn shift_updated_order(start: i32, end: i32, bm: &mut BM_BufferPool, page_num: i32) {
    with_bp_mut(bm, |bp| {
        for i in start..end {
            bp.updated_order[i as usize] = bp.updated_order[(i + 1) as usize];
        }
        bp.updated_order[end as usize] = page_num;
    });
}

fn update_bufferpool_stats(bm: &mut BM_BufferPool, address: i32, page_num: i32) {
    with_bp_mut(bm, |bp| {
        bp.pagenum[address as usize] = page_num;
        bp.num_read += 1;
        bp.fix_count[address as usize] += 1;
        bp.bitdirty[address as usize] = false;
    });
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    with_bp(bm, |bp| {
        if bp.free_space == bp.total_pages {
            vec![NO_PAGE; bp.total_pages as usize]
        } else {
            bp.pagenum.clone()
        }
    })
    .unwrap_or_default()
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    with_bp(bm, |bp| bp.bitdirty.clone()).unwrap_or_default()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    with_bp(bm, |bp| {
        if bp.free_space == bp.total_pages {
            vec![0; bp.total_pages as usize]
        } else {
            bp.fix_count.clone()
        }
    })
    .unwrap_or_default()
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    with_bp(bm, |bp| bp.num_read).unwrap_or(0)
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    with_bp(bm, |bp| bp.num_write).unwrap_or(0)
}

