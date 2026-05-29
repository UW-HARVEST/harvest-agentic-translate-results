use crate::dberror::{RC, PAGE_SIZE};
use crate::storage_mgr::{
    self, ensure_capacity, open_page_file, read_block, write_block, SM_FileHandle,
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

#[allow(non_camel_case_types)]
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

#[allow(non_camel_case_types)]
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

fn clone_strategy(s: &ReplacementStrategy) -> ReplacementStrategy {
    match s {
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
    let strategy_int = strategy_to_int(&strategy);
    let bp = Bufferpool {
        num_read: 0,
        num_write: 0,
        total_pages: num_pages,
        updated_strategy: strategy_int,
        free_space: num_pages,
        updated_order: vec![NO_PAGE; num_pages as usize],
        bitdirty: vec![false; num_pages as usize],
        fix_count: vec![0; num_pages as usize],
        access_time: vec![0; num_pages as usize],
        pagenum: vec![NO_PAGE; num_pages as usize],
        pagedata: storage_mgr::bytes_to_string(&vec![0u8; (num_pages * PAGE_SIZE) as usize]),
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
            // Build slice for the data of this frame
            let start = j * PAGE_SIZE as usize;
            let end = start + PAGE_SIZE as usize;
            let slice: String = bp.pagedata.chars().skip(start).take(PAGE_SIZE as usize).collect();
            // Truncate slice properly considering character/byte mismatch.
            let slice = if slice.chars().count() < PAGE_SIZE as usize {
                // Re-extract using char iteration
                let chars: Vec<char> = bp.pagedata.chars().collect();
                if start < chars.len() {
                    let actual_end = end.min(chars.len());
                    let s: String = chars[start..actual_end].iter().collect();
                    s
                } else {
                    String::new()
                }
            } else {
                slice
            };
            // ensure capacity
            let rc = ensure_capacity(bp.pagenum[j] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let rc = write_block(bp.pagenum[j], &mut bp.fhl, &slice);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
            let _ = end;
        }
    }
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let mut bp = match bm.mgmt_data.take() {
        Some(b) => match b.downcast::<Bufferpool>() {
            Ok(bp) => *bp,
            Err(orig) => {
                bm.mgmt_data = Some(orig);
                return RC::Error;
            }
        },
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] != 0 {
            bm.mgmt_data = Some(Box::new(bp));
            return RC::BufferpoolInUse;
        }
    }
    let rc = write_dirty_pages(&mut bp);
    if rc != RC::Ok {
        bm.mgmt_data = Some(Box::new(bp));
        return rc;
    }
    let close_rc = crate::storage_mgr::close_page_file(&mut bp.fhl);
    if close_rc != RC::Ok {
        return RC::CloseFailed;
    }
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let any_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Ok,
    };
    let bp = match any_box.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let chars: Vec<char> = bp.pagedata.chars().collect();
            let start = i * PAGE_SIZE as usize;
            let end = (start + PAGE_SIZE as usize).min(chars.len());
            let slice: String = chars[start..end].iter().collect();
            let rc = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let rc = write_block(bp.pagenum[i], &mut bp.fhl, &slice);
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
    let any_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let bp = match any_box.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            bp.bitdirty[i] = true;
            break;
        }
    }
    RC::Ok
}

pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let any_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let bp = match any_box.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
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
    let any_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let bp = match any_box.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            let chars: Vec<char> = bp.pagedata.chars().collect();
            let start = i * PAGE_SIZE as usize;
            let end = (start + PAGE_SIZE as usize).min(chars.len());
            let slice: String = chars[start..end].iter().collect();
            let _ = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
            let _ = write_block(bp.pagenum[i], &mut bp.fhl, &slice);
            bp.bitdirty[i] = false;
            bp.num_write += 1;
            return RC::Ok;
        }
    }
    RC::WriteFailed
}

fn put_page_into_frame(bp: &mut Bufferpool, frame_idx: usize, data: &str) {
    let chars: Vec<char> = bp.pagedata.chars().collect();
    let mut chars = chars;
    let start = frame_idx * PAGE_SIZE as usize;
    let data_chars: Vec<char> = data.chars().collect();
    for i in 0..PAGE_SIZE as usize {
        if start + i < chars.len() {
            if i < data_chars.len() {
                chars[start + i] = data_chars[i];
            } else {
                chars[start + i] = '\0';
            }
        }
    }
    bp.pagedata = chars.iter().collect();
}

fn get_frame_data(bp: &Bufferpool, frame_idx: usize) -> String {
    let chars: Vec<char> = bp.pagedata.chars().collect();
    let start = frame_idx * PAGE_SIZE as usize;
    let end = (start + PAGE_SIZE as usize).min(chars.len());
    if start >= chars.len() {
        return String::new();
    }
    chars[start..end].iter().collect()
}

pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let any_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::Error,
    };
    let bp = match any_box.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };

    // Check if already in a frame
    let total_used = (bp.total_pages - bp.free_space) as usize;
    for i in 0..total_used {
        if bp.pagenum[i] == page_num {
            page.page_num = page_num;
            bp.fix_count[i] += 1;
            page.data = get_frame_data(bp, i);
            // LRU update
            if bp.updated_strategy == 1 {
                let last_position = total_used - 1;
                if let Some(swap_loc) = (0..=last_position).find(|&j| bp.updated_order[j] == page_num)
                {
                    for k in swap_loc..last_position {
                        bp.updated_order[k] = bp.updated_order[k + 1];
                    }
                    bp.updated_order[last_position] = page_num;
                }
            }
            return RC::Ok;
        }
    }

    // Read the requested page
    let mut page_buf = String::new();
    let read_rc = read_block(page_num, &mut bp.fhl, &mut page_buf);
    if read_rc != RC::Ok {
        // ensure capacity then read empty page
        let _ = ensure_capacity(page_num + 1, &mut bp.fhl);
        let _ = read_block(page_num, &mut bp.fhl, &mut page_buf);
    }

    // If there's free space, use it
    if bp.free_space > 0 {
        let memory_address = total_used;
        put_page_into_frame(bp, memory_address, &page_buf);
        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;
        page.page_num = page_num;
        page.data = get_frame_data(bp, memory_address);
        return RC::Ok;
    }

    // Else: replace a frame using FIFO/LRU
    let mut found = false;
    let mut memory_address: usize = 0;
    let mut swap_location: usize = 0;
    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        'outer: for j in 0..bp.total_pages as usize {
            let swap_page = bp.updated_order[j];
            for i in 0..bp.total_pages as usize {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    memory_address = i;
                    if bp.bitdirty[i] {
                        let frame_data = get_frame_data(bp, i);
                        let _ = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                        let _ = write_block(bp.pagenum[i], &mut bp.fhl, &frame_data);
                        bp.num_write += 1;
                    }
                    swap_location = j;
                    found = true;
                    break 'outer;
                }
            }
        }
    }

    if !found {
        return RC::BufferpoolFull;
    }

    put_page_into_frame(bp, memory_address, &page_buf);

    // Shift updated_order for FIFO/LRU
    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        let end = (bp.total_pages - 1) as usize;
        for k in swap_location..end {
            bp.updated_order[k] = bp.updated_order[k + 1];
        }
        bp.updated_order[end] = page_num;
    }

    bp.pagenum[memory_address] = page_num;
    bp.num_read += 1;
    bp.fix_count[memory_address] += 1;
    bp.bitdirty[memory_address] = false;

    page.page_num = page_num;
    page.data = get_frame_data(bp, memory_address);
    RC::Ok
}

fn shift_updated_order(_start: i32, _end: i32, _bm: &mut BM_BufferPool, _page_num: i32) {
    // implementation merged into pin_page
}

fn update_bufferpool_stats(_bm: &mut BM_BufferPool, _address: i32, _page_num: i32) {
    // implementation merged into pin_page
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    if let Some(any_box) = bm.mgmt_data.as_ref() {
        if let Some(bp) = any_box.downcast_ref::<Bufferpool>() {
            return bp.pagenum.clone();
        }
    }
    Vec::new()
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    if let Some(any_box) = bm.mgmt_data.as_ref() {
        if let Some(bp) = any_box.downcast_ref::<Bufferpool>() {
            return bp.bitdirty.clone();
        }
    }
    Vec::new()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    if let Some(any_box) = bm.mgmt_data.as_ref() {
        if let Some(bp) = any_box.downcast_ref::<Bufferpool>() {
            return bp.fix_count.clone();
        }
    }
    Vec::new()
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    if let Some(any_box) = bm.mgmt_data.as_ref() {
        if let Some(bp) = any_box.downcast_ref::<Bufferpool>() {
            return bp.num_read;
        }
    }
    0
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    if let Some(any_box) = bm.mgmt_data.as_ref() {
        if let Some(bp) = any_box.downcast_ref::<Bufferpool>() {
            return bp.num_write;
        }
    }
    0
}

// Helper to silence unused warnings on items still referenced via API
#[allow(dead_code)]
fn _unused() {
    let _ = (clone_strategy, shift_updated_order, update_bufferpool_stats);
}
