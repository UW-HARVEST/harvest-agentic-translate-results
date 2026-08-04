use crate::dberror::{PAGE_SIZE, RC};
use crate::storage_mgr::{
    close_page_file, ensure_capacity, open_page_file, read_block, write_block, SM_FileHandle,
};

const PS: usize = PAGE_SIZE as usize;

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

fn copy_strategy(strategy: &ReplacementStrategy) -> ReplacementStrategy {
    match strategy {
        ReplacementStrategy::RsFifo => ReplacementStrategy::RsFifo,
        ReplacementStrategy::RsLru => ReplacementStrategy::RsLru,
        ReplacementStrategy::RsClock => ReplacementStrategy::RsClock,
        ReplacementStrategy::RsLfu => ReplacementStrategy::RsLfu,
        ReplacementStrategy::RsLruK => ReplacementStrategy::RsLruK,
    }
}

fn page_substring(s: &str, start: usize, len: usize) -> String {
    // Each char in our pagedata represents one byte (low 8 bits).
    let chars: Vec<char> = s.chars().collect();
    let end = (start + len).min(chars.len());
    let mut out = String::with_capacity(end.saturating_sub(start));
    for &c in &chars[start.min(chars.len())..end] {
        out.push(c);
    }
    // Pad if shorter
    while out.chars().count() < len {
        out.push('\0');
    }
    out
}

fn write_into_pagedata(pagedata: &mut String, start: usize, src: &str) {
    let mut chars: Vec<char> = pagedata.chars().collect();
    let src_chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < src_chars.len() && start + i < chars.len() {
        chars[start + i] = src_chars[i];
        i += 1;
    }
    *pagedata = chars.into_iter().collect();
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
        updated_strategy: strategy_to_i32(&strategy),
        free_space: num_pages,
        updated_order: vec![NO_PAGE; n],
        bitdirty: vec![false; n],
        fix_count: vec![0; n],
        access_time: vec![0; n],
        pagenum: vec![NO_PAGE; n],
        pagedata: {
            let mut s = String::with_capacity(n * PS);
            for _ in 0..(n * PS) {
                s.push('\0');
            }
            s
        },
        fhl: fh,
    };

    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = copy_strategy(&strategy);
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

fn write_dirty_pages(bp: &mut Bufferpool) -> RC {
    for j in 0..bp.total_pages as usize {
        if bp.bitdirty[j] {
            let record_pointer = j * PS;
            let pn = bp.pagenum[j];
            let rc = ensure_capacity(pn + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let chunk = page_substring(&bp.pagedata, record_pointer, PS);
            let rc = write_block(pn, &mut bp.fhl, &chunk);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let bp = match bm.mgmt_data.as_mut().and_then(|b| b.downcast_mut::<Bufferpool>()) {
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
    if close_page_file(&mut bp.fhl) != RC::Ok {
        return RC::CloseFailed;
    }
    bm.mgmt_data = None;
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let bp = match bm.mgmt_data.as_mut().and_then(|b| b.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let pn = bp.pagenum[i];
            let record_pointer = i * PS;
            let rc = ensure_capacity(pn + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let chunk = page_substring(&bp.pagedata, record_pointer, PS);
            let rc = write_block(pn, &mut bp.fhl, &chunk);
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
    let bp = match bm.mgmt_data.as_mut().and_then(|b| b.downcast_mut::<Bufferpool>()) {
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
    let bp = match bm.mgmt_data.as_mut().and_then(|b| b.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };
    // Sync the page data back into bp's pagedata before unpinning so that
    // any in-place edits performed on `page.data` are persisted.
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            let record_pointer = i * PS;
            write_into_pagedata(&mut bp.pagedata, record_pointer, &page.data);
            if bp.fix_count[i] > 0 {
                bp.fix_count[i] -= 1;
            }
            break;
        }
    }
    RC::Ok
}

pub fn force_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = match bm.mgmt_data.as_mut().and_then(|b| b.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut found = false;
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            let record_pointer = i * PS;
            let chunk = page_substring(&bp.pagedata, record_pointer, PS);
            let rc = write_block(page.page_num, &mut bp.fhl, &chunk);
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

pub fn pin_page(
    bm: &mut BM_BufferPool,
    page: &mut BM_PageHandle,
    page_num: PageNumber,
) -> RC {
    let bp = match bm.mgmt_data.as_mut().and_then(|b| b.downcast_mut::<Bufferpool>()) {
        Some(b) => b,
        None => return RC::Error,
    };

    // Case 1: page already in pool.
    let used = (bp.total_pages - bp.free_space) as usize;
    for i in 0..used {
        if bp.pagenum[i] == page_num {
            page.page_num = page_num;
            bp.fix_count[i] += 1;
            page.data = page_substring(&bp.pagedata, i * PS, PS);
            // LRU update: move pageNum to the end of updated_order.
            if bp.updated_strategy == 1 {
                let last_pos = (bp.total_pages - bp.free_space - 1) as usize;
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

    // Case 2: there's a free slot.
    if bp.free_space > 0 {
        let mut page_data = String::with_capacity(PS);
        for _ in 0..PS {
            page_data.push('\0');
        }
        let read_rc = read_block(page_num, &mut bp.fhl, &mut page_data);
        if read_rc != RC::Ok {
            // Mimic C behavior: still proceed (use zero-filled buffer).
        }
        let memory_address = (bp.total_pages - bp.free_space) as usize;
        let record_pointer = memory_address * PS;
        write_into_pagedata(&mut bp.pagedata, record_pointer, &page_data);
        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;
        page.page_num = page_num;
        page.data = page_substring(&bp.pagedata, record_pointer, PS);
        return RC::Ok;
    }

    // Case 3: buffer pool full -> need replacement.
    let mut page_data = String::with_capacity(PS);
    for _ in 0..PS {
        page_data.push('\0');
    }
    let _ = read_block(page_num, &mut bp.fhl, &mut page_data);

    let mut updated_strategy_found = false;
    let mut memory_address: usize = 0;
    let mut swap_location: usize = 0;

    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        let mut j: usize = 0;
        while j < bp.total_pages as usize {
            let swap_page = bp.updated_order[j];
            let mut i: usize = 0;
            while i < bp.total_pages as usize {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    memory_address = i;
                    let record_pointer = i * PS;
                    if bp.bitdirty[i] {
                        let pn = bp.pagenum[i];
                        let _ = ensure_capacity(pn + 1, &mut bp.fhl);
                        let chunk = page_substring(&bp.pagedata, record_pointer, PS);
                        let _ = write_block(pn, &mut bp.fhl, &chunk);
                        bp.num_write += 1;
                    }
                    swap_location = j;
                    updated_strategy_found = true;
                    break;
                }
                i += 1;
            }
            if updated_strategy_found {
                break;
            }
            j += 1;
        }
    }

    if !updated_strategy_found {
        return RC::BufferpoolFull;
    }

    let record_pointer = memory_address * PS;
    write_into_pagedata(&mut bp.pagedata, record_pointer, &page_data);

    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        shift_updated_order_inner(bp, swap_location, (bp.total_pages - 1) as usize, page_num);
    }
    update_buffer_pool_stats_inner(bp, memory_address, page_num);
    page.page_num = page_num;
    page.data = page_substring(&bp.pagedata, record_pointer, PS);
    RC::Ok
}

fn shift_updated_order_inner(bp: &mut Bufferpool, start: usize, end: usize, page_num: i32) {
    let mut i = start;
    while i < end {
        bp.updated_order[i] = bp.updated_order[i + 1];
        i += 1;
    }
    bp.updated_order[end] = page_num;
}

fn update_buffer_pool_stats_inner(bp: &mut Bufferpool, memory_address: usize, page_num: i32) {
    bp.pagenum[memory_address] = page_num;
    bp.num_read += 1;
    bp.fix_count[memory_address] += 1;
    bp.bitdirty[memory_address] = false;
}

#[allow(dead_code)]
fn shift_updated_order(_start: i32, _end: i32, _bm: &mut BM_BufferPool, _page_num: i32) {}

#[allow(dead_code)]
fn update_bufferpool_stats(_bm: &mut BM_BufferPool, _address: i32, _page_num: i32) {}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    if let Some(bp) = bm.mgmt_data.as_ref().and_then(|b| b.downcast_ref::<Bufferpool>()) {
        bp.pagenum.clone()
    } else {
        Vec::new()
    }
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    if let Some(bp) = bm.mgmt_data.as_ref().and_then(|b| b.downcast_ref::<Bufferpool>()) {
        bp.bitdirty.clone()
    } else {
        Vec::new()
    }
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    if let Some(bp) = bm.mgmt_data.as_ref().and_then(|b| b.downcast_ref::<Bufferpool>()) {
        if bp.free_space == bp.total_pages {
            vec![0; bp.total_pages as usize]
        } else {
            bp.fix_count.clone()
        }
    } else {
        Vec::new()
    }
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    if let Some(bp) = bm.mgmt_data.as_ref().and_then(|b| b.downcast_ref::<Bufferpool>()) {
        bp.num_read
    } else {
        0
    }
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    if let Some(bp) = bm.mgmt_data.as_ref().and_then(|b| b.downcast_ref::<Bufferpool>()) {
        bp.num_write
    } else {
        0
    }
}
