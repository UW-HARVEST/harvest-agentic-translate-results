use crate::dberror::{RC, PAGE_SIZE};
use crate::storage_mgr::{
    SM_FileHandle, SM_PageHandle, open_page_file, close_page_file,
    read_block, write_block, ensure_capacity,
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
        pagedata: String::from_utf8(vec![0u8; n * PAGE_SIZE as usize]).unwrap_or_default(),
        fhl: fh,
    };

    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = clone_strategy(&strategy);
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

fn write_dirty_pages(bp: &mut Bufferpool) -> RC {
    for j in 0..bp.total_pages as usize {
        if bp.bitdirty[j] {
            let record_pointer = j * PAGE_SIZE as usize;
            let rc = ensure_capacity(bp.pagenum[j] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let data_bytes = bp.pagedata.as_bytes();
            let end = std::cmp::min(record_pointer + PAGE_SIZE as usize, data_bytes.len());
            let slice = &data_bytes[record_pointer..end];
            let page_data = String::from_utf8_lossy(slice).into_owned();
            let rc = write_block(bp.pagenum[j], &mut bp.fhl, &page_data);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let mut bp_box = match bm.mgmt_data.take() {
        Some(b) => b,
        None => return RC::ShutdownWithoutInit,
    };
    let bp = match bp_box.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] != 0 {
            // Restore mgmt_data
            bm.mgmt_data = Some(bp_box);
            return RC::BufferpoolInUse;
        }
    }
    let rc = write_dirty_pages(bp);
    if rc != RC::Ok {
        bm.mgmt_data = Some(bp_box);
        return rc;
    }
    if close_page_file(&mut bp.fhl) != RC::Ok {
        return RC::CloseFailed;
    }
    // Drop bp_box automatically
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let bp_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::ShutdownWithoutInit,
    };
    let bp = match bp_box.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    for i in 0..bp.total_pages as usize {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let record_pointer = i * PAGE_SIZE as usize;
            let rc = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let data_bytes = bp.pagedata.as_bytes();
            let end = std::cmp::min(record_pointer + PAGE_SIZE as usize, data_bytes.len());
            let slice = &data_bytes[record_pointer..end];
            let page_data = String::from_utf8_lossy(slice).into_owned();
            let rc = write_block(bp.pagenum[i], &mut bp.fhl, &page_data);
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
    let bp_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::ShutdownWithoutInit,
    };
    let bp = match bp_box.downcast_mut::<Bufferpool>() {
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
    let bp_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::ShutdownWithoutInit,
    };
    let bp = match bp_box.downcast_mut::<Bufferpool>() {
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
    let bp_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::ShutdownWithoutInit,
    };
    let bp = match bp_box.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    let mut page_found = false;
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            let record_pointer = i * PAGE_SIZE as usize;
            println!(
                "Simulated writing of page {} to disk at position {}.",
                page.page_num, record_pointer
            );
            bp.bitdirty[i] = false;
            bp.num_write += 1;
            page_found = true;
            break;
        }
    }
    if page_found {
        RC::Ok
    } else {
        println!("Page {} not found in buffer pool.", page.page_num);
        RC::WriteFailed
    }
}

pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let bp_box = match bm.mgmt_data.as_mut() {
        Some(b) => b,
        None => return RC::ShutdownWithoutInit,
    };
    let bp = match bp_box.downcast_mut::<Bufferpool>() {
        Some(b) => b,
        None => return RC::Error,
    };
    let void_page = bp.free_space == bp.total_pages;

    // Search for existing page
    if !void_page {
        let used = (bp.total_pages - bp.free_space) as usize;
        for i in 0..used {
            if bp.pagenum[i] == page_num {
                page.page_num = page_num;
                bp.fix_count[i] += 1;
                let start = i * PAGE_SIZE as usize;
                let end = std::cmp::min(start + PAGE_SIZE as usize, bp.pagedata.len());
                let slice = &bp.pagedata.as_bytes()[start..end];
                page.data = String::from_utf8_lossy(slice).into_owned();

                if bp.updated_strategy == 1 {
                    // RS_LRU: move pageNum to the end of updated_order
                    let last_position = (bp.total_pages - bp.free_space - 1) as usize;
                    let mut swap_location: i32 = -1;
                    for j in 0..=last_position {
                        if bp.updated_order[j] == page_num {
                            swap_location = j as i32;
                            break;
                        }
                    }
                    if swap_location != -1 {
                        let sl = swap_location as usize;
                        for k in sl..last_position {
                            bp.updated_order[k] = bp.updated_order[k + 1];
                        }
                        bp.updated_order[last_position] = page_num;
                    }
                }
                return RC::Ok;
            }
        }
    }

    // If there's free space, load the page into a new slot
    if void_page || bp.free_space > 0 {
        let mut page_handle: SM_PageHandle = String::new();
        let rc = read_block(page_num, &mut bp.fhl, &mut page_handle);
        if rc != RC::Ok && rc != RC::ReadNonExistingPage {
            // Try ensure capacity then re-read
        }

        let memory_address = (bp.total_pages - bp.free_space) as usize;
        let record_pointer = memory_address * PAGE_SIZE as usize;

        // Copy page_handle into bp.pagedata at record_pointer
        let mut data_bytes = bp.pagedata.as_bytes().to_vec();
        let ph_bytes = page_handle.as_bytes();
        let copy_len = std::cmp::min(PAGE_SIZE as usize, ph_bytes.len());
        if record_pointer + PAGE_SIZE as usize <= data_bytes.len() {
            for k in 0..copy_len {
                data_bytes[record_pointer + k] = ph_bytes[k];
            }
            // Zero out rest of page
            for k in copy_len..PAGE_SIZE as usize {
                data_bytes[record_pointer + k] = 0;
            }
        }
        bp.pagedata = String::from_utf8_lossy(&data_bytes).into_owned();

        bp.free_space -= 1;
        bp.updated_order[memory_address] = page_num;
        bp.pagenum[memory_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[memory_address] += 1;
        bp.bitdirty[memory_address] = false;

        page.page_num = page_num;
        let start = record_pointer;
        let end = std::cmp::min(start + PAGE_SIZE as usize, bp.pagedata.len());
        let slice = &bp.pagedata.as_bytes()[start..end];
        page.data = String::from_utf8_lossy(slice).into_owned();
        return RC::Ok;
    }

    // Buffer pool is full: need to evict using FIFO/LRU strategy.
    let mut updated_stra_found = false;
    let mut memory_address: usize = 0;
    let mut swap_location: usize = 0;
    let mut page_handle: SM_PageHandle = String::new();
    let _ = read_block(page_num, &mut bp.fhl, &mut page_handle);

    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        let total_pages = bp.total_pages as usize;
        'outer: for j in 0..total_pages {
            let swap_page = bp.updated_order[j];
            for i in 0..total_pages {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    memory_address = i;
                    let record_pointer = i * PAGE_SIZE as usize;
                    if bp.bitdirty[i] {
                        let _ = ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                        let data_bytes = bp.pagedata.as_bytes();
                        let end = std::cmp::min(record_pointer + PAGE_SIZE as usize, data_bytes.len());
                        let slice = &data_bytes[record_pointer..end];
                        let page_data = String::from_utf8_lossy(slice).into_owned();
                        let _ = write_block(bp.pagenum[i], &mut bp.fhl, &page_data);
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

    let record_pointer = memory_address * PAGE_SIZE as usize;
    let mut data_bytes = bp.pagedata.as_bytes().to_vec();
    let ph_bytes = page_handle.as_bytes();
    let copy_len = std::cmp::min(PAGE_SIZE as usize, ph_bytes.len());
    if record_pointer + PAGE_SIZE as usize <= data_bytes.len() {
        for k in 0..copy_len {
            data_bytes[record_pointer + k] = ph_bytes[k];
        }
        for k in copy_len..PAGE_SIZE as usize {
            data_bytes[record_pointer + k] = 0;
        }
    }
    bp.pagedata = String::from_utf8_lossy(&data_bytes).into_owned();

    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        let end_pos = (bp.total_pages - 1) as usize;
        for k in swap_location..end_pos {
            bp.updated_order[k] = bp.updated_order[k + 1];
        }
        bp.updated_order[end_pos] = page_num;
    }

    bp.pagenum[memory_address] = page_num;
    bp.num_read += 1;
    bp.fix_count[memory_address] += 1;
    bp.bitdirty[memory_address] = false;

    page.page_num = page_num;
    let start = record_pointer;
    let end = std::cmp::min(start + PAGE_SIZE as usize, bp.pagedata.len());
    let slice = &bp.pagedata.as_bytes()[start..end];
    page.data = String::from_utf8_lossy(slice).into_owned();
    RC::Ok
}

fn shift_updated_order(_start: i32, _end: i32, _bm: &mut BM_BufferPool, _page_num: i32) {
    // Helper kept for parity with C; main logic is inlined in pin_page.
}

fn update_bufferpool_stats(_bm: &mut BM_BufferPool, _address: i32, _page_num: i32) {
    // Helper kept for parity with C; main logic is inlined in pin_page.
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    let bp_box = match bm.mgmt_data.as_ref() {
        Some(b) => b,
        None => return vec![],
    };
    let bp = match bp_box.downcast_ref::<Bufferpool>() {
        Some(b) => b,
        None => return vec![],
    };
    if bp.free_space == bp.total_pages {
        vec![NO_PAGE; bp.total_pages as usize]
    } else {
        bp.pagenum.clone()
    }
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    let bp_box = match bm.mgmt_data.as_ref() {
        Some(b) => b,
        None => return vec![],
    };
    let bp = match bp_box.downcast_ref::<Bufferpool>() {
        Some(b) => b,
        None => return vec![],
    };
    bp.bitdirty.clone()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    let bp_box = match bm.mgmt_data.as_ref() {
        Some(b) => b,
        None => return vec![],
    };
    let bp = match bp_box.downcast_ref::<Bufferpool>() {
        Some(b) => b,
        None => return vec![],
    };
    if bp.free_space == bp.total_pages {
        vec![0; bp.total_pages as usize]
    } else {
        bp.fix_count.clone()
    }
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    let bp_box = match bm.mgmt_data.as_ref() {
        Some(b) => b,
        None => return 0,
    };
    let bp = match bp_box.downcast_ref::<Bufferpool>() {
        Some(b) => b,
        None => return 0,
    };
    bp.num_read
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    let bp_box = match bm.mgmt_data.as_ref() {
        Some(b) => b,
        None => return 0,
    };
    let bp = match bp_box.downcast_ref::<Bufferpool>() {
        Some(b) => b,
        None => return 0,
    };
    bp.num_write
}

#[allow(dead_code)]
fn _suppress(_: i32) {
    // suppress unused warnings
    let _ = strategy_to_int;
    let _ = shift_updated_order;
    let _ = update_bufferpool_stats;
}
