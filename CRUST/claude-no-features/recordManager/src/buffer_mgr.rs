use crate::dberror::{RC, PAGE_SIZE};
use crate::storage_mgr::{self, SM_FileHandle};

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

fn strat_as_int(s: &ReplacementStrategy) -> i32 {
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
    let rc = storage_mgr::open_page_file(page_file_name, &mut fh);
    if rc != RC::Ok {
        return rc;
    }
    let strat_int = strat_as_int(&strategy);
    let bp = Bufferpool {
        num_read: 0,
        num_write: 0,
        total_pages: num_pages,
        updated_strategy: strat_int,
        free_space: num_pages,
        updated_order: vec![NO_PAGE; num_pages as usize],
        bitdirty: vec![false; num_pages as usize],
        fix_count: vec![0; num_pages as usize],
        access_time: vec![0; num_pages as usize],
        pagenum: vec![NO_PAGE; num_pages as usize],
        pagedata: (0..(num_pages * PAGE_SIZE)).map(|_| '\0').collect(),
        fhl: fh,
    };
    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let mut bp = match bm.mgmt_data.take() {
        Some(m) => match m.downcast::<Bufferpool>() {
            Ok(b) => b,
            Err(_) => return RC::Error,
        },
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] != 0 {
            bm.mgmt_data = Some(bp);
            return RC::BufferpoolInUse;
        }
    }
    // Write dirty pages
    for j in 0..bp.total_pages as usize {
        if bp.bitdirty[j] {
            let _ = storage_mgr::ensure_capacity(bp.pagenum[j] + 1, &mut bp.fhl);
            let start = j * PAGE_SIZE as usize;
            let end = start + PAGE_SIZE as usize;
            let chars: String = bp.pagedata.chars().skip(start).take(PAGE_SIZE as usize).collect();
            let _ = end;
            let pn = bp.pagenum[j];
            let rc = storage_mgr::write_block(pn, &mut bp.fhl, &chars);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
        }
    }
    let _ = storage_mgr::close_page_file(&mut bp.fhl);
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let bp = match bm.mgmt_data.as_mut() {
        Some(m) => match m.downcast_mut::<Bufferpool>() {
            Some(b) => b,
            None => return RC::Error,
        },
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let start = i * PAGE_SIZE as usize;
            let chars: String = bp.pagedata.chars().skip(start).take(PAGE_SIZE as usize).collect();
            let pn = bp.pagenum[i];
            let _ = storage_mgr::ensure_capacity(pn + 1, &mut bp.fhl);
            let rc = storage_mgr::write_block(pn, &mut bp.fhl, &chars);
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
    let bp = match bm.mgmt_data.as_mut() {
        Some(m) => match m.downcast_mut::<Bufferpool>() {
            Some(b) => b,
            None => return RC::Error,
        },
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
    let bp = match bm.mgmt_data.as_mut() {
        Some(m) => match m.downcast_mut::<Bufferpool>() {
            Some(b) => b,
            None => return RC::Error,
        },
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
    let bp = match bm.mgmt_data.as_mut() {
        Some(m) => match m.downcast_mut::<Bufferpool>() {
            Some(b) => b,
            None => return RC::Error,
        },
        None => return RC::Error,
    };
    let mut found = false;
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            let start = i * PAGE_SIZE as usize;
            let chars: String = bp.pagedata.chars().skip(start).take(PAGE_SIZE as usize).collect();
            let pn = bp.pagenum[i];
            let _ = storage_mgr::ensure_capacity(pn + 1, &mut bp.fhl);
            let _ = storage_mgr::write_block(pn, &mut bp.fhl, &chars);
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
    let bp = match bm.mgmt_data.as_mut() {
        Some(m) => match m.downcast_mut::<Bufferpool>() {
            Some(b) => b,
            None => return RC::Error,
        },
        None => return RC::Error,
    };
    // Check if page already in pool
    let total_used = (bp.total_pages - bp.free_space) as usize;
    for i in 0..total_used {
        if bp.pagenum[i] == page_num {
            page.page_num = page_num;
            bp.fix_count[i] += 1;
            let start = i * PAGE_SIZE as usize;
            page.data = bp.pagedata.chars().skip(start).take(PAGE_SIZE as usize).collect();
            return RC::Ok;
        }
    }
    // Need to load from disk; ensure capacity first
    let _ = storage_mgr::ensure_capacity(page_num + 1, &mut bp.fhl);
    let mut buf: String = (0..PAGE_SIZE).map(|_| '\0').collect();
    let _ = storage_mgr::read_block(page_num, &mut bp.fhl, &mut buf);

    if bp.free_space > 0 {
        let memory_address = total_used;
        let record_pointer = memory_address * PAGE_SIZE as usize;
        // Write into pagedata at record_pointer
        let mut chars: Vec<char> = bp.pagedata.chars().collect();
        let buf_chars: Vec<char> = buf.chars().collect();
        for j in 0..PAGE_SIZE as usize {
            if j < buf_chars.len() && record_pointer + j < chars.len() {
                chars[record_pointer + j] = buf_chars[j];
            }
        }
        bp.pagedata = chars.into_iter().collect();
        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;
        page.page_num = page_num;
        page.data = bp.pagedata.chars().skip(record_pointer).take(PAGE_SIZE as usize).collect();
        return RC::Ok;
    }
    // Pool full, need replacement (FIFO/LRU)
    let mut found = false;
    let mut memory_address = 0usize;
    let mut swap_location = 0usize;
    for j in 0..bp.total_pages as usize {
        let swap_page = bp.updated_order[j];
        for i in 0..bp.total_pages as usize {
            if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                memory_address = i;
                let record_pointer = i * PAGE_SIZE as usize;
                if bp.bitdirty[i] {
                    let chars: String = bp.pagedata.chars().skip(record_pointer).take(PAGE_SIZE as usize).collect();
                    let pn = bp.pagenum[i];
                    let _ = storage_mgr::ensure_capacity(pn + 1, &mut bp.fhl);
                    let _ = storage_mgr::write_block(pn, &mut bp.fhl, &chars);
                    bp.num_write += 1;
                }
                swap_location = j;
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
    let record_pointer = memory_address * PAGE_SIZE as usize;
    let mut chars: Vec<char> = bp.pagedata.chars().collect();
    let buf_chars: Vec<char> = buf.chars().collect();
    for j in 0..PAGE_SIZE as usize {
        if j < buf_chars.len() && record_pointer + j < chars.len() {
            chars[record_pointer + j] = buf_chars[j];
        }
    }
    bp.pagedata = chars.into_iter().collect();
    // Shift updated_order
    let end = bp.total_pages as usize - 1;
    for i in swap_location..end {
        bp.updated_order[i] = bp.updated_order[i + 1];
    }
    bp.updated_order[end] = page_num;
    bp.pagenum[memory_address] = page_num;
    bp.num_read += 1;
    bp.fix_count[memory_address] += 1;
    bp.bitdirty[memory_address] = false;
    page.page_num = page_num;
    page.data = bp.pagedata.chars().skip(record_pointer).take(PAGE_SIZE as usize).collect();
    RC::Ok
}

fn shift_updated_order(_start: i32, _end: i32, _bm: &mut BM_BufferPool, _page_num: i32) {
    // Inlined into pin_page
}

fn update_bufferpool_stats(_bm: &mut BM_BufferPool, _address: i32, _page_num: i32) {
    // Inlined into pin_page
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    match bm.mgmt_data.as_ref() {
        Some(m) => match m.downcast_ref::<Bufferpool>() {
            Some(bp) => bp.pagenum.clone(),
            None => vec![],
        },
        None => vec![],
    }
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    match bm.mgmt_data.as_ref() {
        Some(m) => match m.downcast_ref::<Bufferpool>() {
            Some(bp) => bp.bitdirty.clone(),
            None => vec![],
        },
        None => vec![],
    }
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    match bm.mgmt_data.as_ref() {
        Some(m) => match m.downcast_ref::<Bufferpool>() {
            Some(bp) => bp.fix_count.clone(),
            None => vec![],
        },
        None => vec![],
    }
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    match bm.mgmt_data.as_ref() {
        Some(m) => match m.downcast_ref::<Bufferpool>() {
            Some(bp) => bp.num_read,
            None => 0,
        },
        None => 0,
    }
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    match bm.mgmt_data.as_ref() {
        Some(m) => match m.downcast_ref::<Bufferpool>() {
            Some(bp) => bp.num_write,
            None => 0,
        },
        None => 0,
    }
}
