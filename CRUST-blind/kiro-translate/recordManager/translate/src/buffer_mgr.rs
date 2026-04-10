use crate::dberror::{RC, PAGE_SIZE};
use crate::storage_mgr::{SM_FileHandle,
    open_page_file, close_page_file, read_block, write_block, ensure_capacity};

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

fn strategy_to_i32(s: &ReplacementStrategy) -> i32 {
    match s {
        ReplacementStrategy::RsFifo => 0,
        ReplacementStrategy::RsLru => 1,
        ReplacementStrategy::RsClock => 2,
        ReplacementStrategy::RsLfu => 3,
        ReplacementStrategy::RsLruK => 4,
    }
}

fn get_bp(bm: &mut BM_BufferPool) -> &mut Bufferpool {
    bm.mgmt_data.as_mut().unwrap().downcast_mut::<Bufferpool>().unwrap()
}

fn get_bp_ref(bm: &BM_BufferPool) -> &Bufferpool {
    bm.mgmt_data.as_ref().unwrap().downcast_ref::<Bufferpool>().unwrap()
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
    // pagedata: n pages of PAGE_SIZE chars, all null
    let pagedata: String = std::iter::repeat('\0').take(n * PAGE_SIZE as usize).collect();

    let bp = Bufferpool {
        total_pages: num_pages,
        pagedata,
        num_read: 0,
        num_write: 0,
        updated_order: vec![NO_PAGE; n],
        bitdirty: vec![false; n],
        free_space: num_pages,
        fhl: fh,
        pagenum: vec![NO_PAGE; n],
        fix_count: vec![0; n],
        access_time: vec![0; n],
        updated_strategy: strategy_to_i32(&strategy),
    };

    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

fn write_dirty_pages(bm: &mut BM_BufferPool) -> RC {
    let bp = get_bp(bm);
    let ps = PAGE_SIZE as usize;
    for j in 0..bp.total_pages as usize {
        if bp.bitdirty[j] {
            let record_pointer = j * ps;
            let page_str: String = bp.pagedata.chars().skip(record_pointer).take(ps).collect();
            let pn = bp.pagenum[j];
            let rc = ensure_capacity(pn + 1, &mut bp.fhl);
            if rc != RC::Ok { return rc; }
            let rc = write_block(pn, &mut bp.fhl, &page_str);
            if rc != RC::Ok { return RC::WriteFailed; }
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    {
        let bp = get_bp(bm);
        for i in 0..bp.total_pages as usize {
            if bp.fix_count[i] != 0 {
                return RC::BufferpoolInUse;
            }
        }
    }
    let rc = write_dirty_pages(bm);
    if rc != RC::Ok { return rc; }
    let bp = get_bp(bm);
    if close_page_file(&mut bp.fhl) != RC::Ok {
        return RC::CloseFailed;
    }
    bm.mgmt_data = None;
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let bp = get_bp(bm);
    let ps = PAGE_SIZE as usize;
    let file_name = bp.fhl.file_name.clone();

    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let record_pointer = i * ps;
            let page_str: String = bp.pagedata.chars().skip(record_pointer).take(ps).collect();
            let pn = bp.pagenum[i];

            // Write directly to file like the C code does
            use std::fs::OpenOptions;
            use std::io::{Seek, SeekFrom, Write};
            // Ensure file is large enough
            {
                let mut file = match OpenOptions::new().read(true).write(true).open(&file_name) {
                    Ok(f) => f,
                    Err(_) => return RC::WriteFailed,
                };
                let file_size = file.seek(SeekFrom::End(0)).unwrap_or(0);
                let required_size = ((pn + 1) as u64) * (ps as u64);
                if file_size < required_size {
                    file.seek(SeekFrom::Start(required_size - 1)).unwrap();
                    file.write_all(&[0u8]).unwrap();
                }
            }
            {
                let mut file = match OpenOptions::new().read(true).write(true).open(&file_name) {
                    Ok(f) => f,
                    Err(_) => return RC::WriteFailed,
                };
                let offset = (pn as u64) * (ps as u64);
                file.seek(SeekFrom::Start(offset)).unwrap();
                let bytes: Vec<u8> = page_str.chars().map(|c| c as u8).collect();
                let mut write_buf = vec![0u8; ps];
                let copy_len = bytes.len().min(ps);
                write_buf[..copy_len].copy_from_slice(&bytes[..copy_len]);
                if file.write_all(&write_buf).is_err() {
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
    let bp = get_bp(bm);
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            if !bp.bitdirty[i] {
                bp.bitdirty[i] = true;
            }
            break;
        }
    }
    RC::Ok
}

pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = get_bp(bm);
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
    let bp = get_bp(bm);
    let mut found = false;
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            let _record_pointer = i * PAGE_SIZE as usize;
            println!("Simulated writing of page {} to disk at position {}.", page.page_num, _record_pointer);
            bp.bitdirty[i] = false;
            bp.num_write += 1;
            found = true;
            break;
        }
    }
    if found {
        RC::Ok
    } else {
        println!("Page {} not found in buffer pool.", page.page_num);
        RC::WriteFailed
    }
}

pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let bp = get_bp(bm);
    let ps = PAGE_SIZE as usize;
    let void_page = bp.free_space == bp.total_pages;

    // Check if page already in pool
    if !void_page {
        let total_used = (bp.total_pages - bp.free_space) as usize;
        for i in 0..total_used {
            if bp.pagenum[i] == page_num {
                page.page_num = page_num;
                bp.fix_count[i] += 1;
                let start = i * ps;
                page.data = bp.pagedata.chars().skip(start).take(ps).collect();

                if bp.updated_strategy == 1 {
                    // LRU: move to end
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
        let mut page_handle = String::new();
        let rc = read_block(page_num, &mut bp.fhl, &mut page_handle);
        if rc != RC::Ok {
            // If read fails, still proceed like C code does (it doesn't check properly)
        }
        let memory_address = (bp.total_pages - bp.free_space) as usize;
        let record_pointer = memory_address * ps;

        // Copy page_handle into pagedata at record_pointer
        let mut chars: Vec<char> = bp.pagedata.chars().collect();
        let ph_chars: Vec<char> = page_handle.chars().collect();
        for k in 0..ps {
            chars[record_pointer + k] = if k < ph_chars.len() { ph_chars[k] } else { '\0' };
        }
        bp.pagedata = chars.into_iter().collect();

        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;
        page.page_num = page_num;
        page.data = bp.pagedata.chars().skip(record_pointer).take(ps).collect();
        return RC::Ok;
    }

    // Buffer pool full - need replacement
    let strat = bp.updated_strategy;
    if strat == 0 || strat == 1 {
        // FIFO or LRU
        let mut page_handle = String::new();
        let _rc = read_block(page_num, &mut bp.fhl, &mut page_handle);

        let mut found = false;
        let mut memory_address = 0usize;
        let mut swap_location = 0usize;

        let mut j = 0usize;
        while j < bp.total_pages as usize {
            let swap_page = bp.updated_order[j];
            let mut i = 0usize;
            while i < bp.total_pages as usize {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    memory_address = i;
                    let record_pointer = i * ps;
                    if bp.bitdirty[i] {
                        let page_str: String = bp.pagedata.chars().skip(record_pointer).take(ps).collect();
                        let _ = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                        let _ = write_block(bp.pagenum[i], &mut bp.fhl, &page_str);
                        bp.num_write += 1;
                    }
                    swap_location = j;
                    found = true;
                    break;
                }
                i += 1;
            }
            if found { break; }
            j += 1;
        }

        if !found {
            return RC::BufferpoolFull;
        }

        // Copy new page data
        let record_pointer = memory_address * ps;
        let mut chars: Vec<char> = bp.pagedata.chars().collect();
        let ph_chars: Vec<char> = page_handle.chars().collect();
        for k in 0..ps {
            chars[record_pointer + k] = if k < ph_chars.len() { ph_chars[k] } else { '\0' };
        }
        bp.pagedata = chars.into_iter().collect();

        // Shift updated order
        let end = bp.total_pages as usize - 1;
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
        page.data = bp.pagedata.chars().skip(record_pointer).take(ps).collect();
        return RC::Ok;
    }

    RC::BufferpoolFull
}

fn shift_updated_order(start: i32, end: i32, bm: &mut BM_BufferPool, page_num: i32) {
    let bp = get_bp(bm);
    for i in start..end {
        bp.updated_order[i as usize] = bp.updated_order[(i + 1) as usize];
    }
    bp.updated_order[end as usize] = page_num;
}

fn update_bufferpool_stats(bm: &mut BM_BufferPool, address: i32, page_num: i32) {
    let bp = get_bp(bm);
    bp.pagenum[address as usize] = page_num;
    bp.num_read += 1;
    bp.fix_count[address as usize] += 1;
    bp.bitdirty[address as usize] = false;
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    let bp = get_bp_ref(bm);
    if bp.free_space == bp.total_pages {
        vec![]
    } else {
        bp.pagenum.clone()
    }
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    let bp = get_bp_ref(bm);
    bp.bitdirty.clone()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    let bp = get_bp_ref(bm);
    if bp.free_space == bp.total_pages {
        vec![0]
    } else {
        bp.fix_count.clone()
    }
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    match bm.mgmt_data.as_ref() {
        Some(d) => d.downcast_ref::<Bufferpool>().map_or(0, |bp| bp.num_read),
        None => 0,
    }
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    match bm.mgmt_data.as_ref() {
        Some(d) => d.downcast_ref::<Bufferpool>().map_or(0, |bp| bp.num_write),
        None => 0,
    }
}
