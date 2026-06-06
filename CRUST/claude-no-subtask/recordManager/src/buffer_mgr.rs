use crate::dberror::{RC, PAGE_SIZE};
use crate::storage_mgr::{
    self, SM_FileHandle,
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

impl ReplacementStrategy {
    pub fn code(&self) -> i32 {
        match self {
            ReplacementStrategy::RsFifo => 0,
            ReplacementStrategy::RsLru => 1,
            ReplacementStrategy::RsClock => 2,
            ReplacementStrategy::RsLfu => 3,
            ReplacementStrategy::RsLruK => 4,
        }
    }
}

pub struct BM_BufferPool {
    pub page_file: String,
    pub num_pages: i32,
    pub strategy: ReplacementStrategy,
    pub mgmt_data: Option<Box<dyn std::any::Any>>,
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
    let total = num_pages as usize;
    let bp = Bufferpool {
        num_read: 0,
        num_write: 0,
        total_pages: num_pages,
        updated_strategy: strategy.code(),
        free_space: num_pages,
        updated_order: vec![NO_PAGE; total],
        bitdirty: vec![false; total],
        fix_count: vec![0; total],
        access_time: vec![0; total],
        pagenum: vec![NO_PAGE; total],
        pagedata: String::from_utf8(vec![0u8; total * PAGE_SIZE as usize]).unwrap_or_default(),
        fhl: fh,
    };
    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(bp));
    RC::Ok
}

fn write_dirty_pages(bp: &mut Bufferpool) -> RC {
    let total = bp.total_pages as usize;
    for j in 0..total {
        if bp.bitdirty[j] {
            let record_pointer = j * PAGE_SIZE as usize;
            let rc = storage_mgr::ensure_capacity(bp.pagenum[j] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let page_str = read_page_str(&bp.pagedata, record_pointer);
            let rc = storage_mgr::write_block(bp.pagenum[j], &mut bp.fhl, &page_str);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.num_write += 1;
        }
    }
    RC::Ok
}

fn read_page_str(pagedata: &str, offset: usize) -> String {
    let bytes = pagedata.as_bytes();
    let end = std::cmp::min(offset + PAGE_SIZE as usize, bytes.len());
    let slice = if offset < bytes.len() { &bytes[offset..end] } else { &[][..] };
    String::from_utf8_lossy(slice).to_string()
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(bp) => bp,
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
    if storage_mgr::close_page_file(&mut bp.fhl) != RC::Ok {
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
        Some(bp) => bp,
        None => return RC::Error,
    };
    let total = bp.total_pages as usize;
    for i in 0..total {
        if bp.fix_count[i] == 0 && bp.bitdirty[i] {
            let record_pointer = i * PAGE_SIZE as usize;
            let rc = storage_mgr::ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let page_str = read_page_str(&bp.pagedata, record_pointer);
            let rc = storage_mgr::write_block(bp.pagenum[i], &mut bp.fhl, &page_str);
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
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(bp) => bp,
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
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(bp) => bp,
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
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return RC::Error,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(bp) => bp,
        None => return RC::Error,
    };
    let mut found = false;
    for i in 0..bp.total_pages as usize {
        if bp.pagenum[i] == page.page_num {
            let record_pointer = i * PAGE_SIZE as usize;
            let rc = storage_mgr::ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let page_str = read_page_str(&bp.pagedata, record_pointer);
            let rc = storage_mgr::write_block(bp.pagenum[i], &mut bp.fhl, &page_str);
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
        Some(bp) => bp,
        None => return RC::Error,
    };

    // Check if already in pool
    let used = (bp.total_pages - bp.free_space) as usize;
    if bp.free_space != bp.total_pages {
        for i in 0..used {
            if bp.pagenum[i] == page_num {
                page.page_num = page_num;
                bp.fix_count[i] += 1;
                let record_pointer = i * PAGE_SIZE as usize;
                page.data = read_page_str(&bp.pagedata, record_pointer);
                if bp.updated_strategy == 1 {
                    // RS_LRU - move to end
                    let last_position = used - 1;
                    let mut swap_location: i32 = -1;
                    for j in 0..=last_position {
                        if bp.updated_order[j] == page_num {
                            swap_location = j as i32;
                            break;
                        }
                    }
                    if swap_location >= 0 {
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

    // Page not in pool; need to load
    let mut buf = String::from_utf8(vec![0u8; PAGE_SIZE as usize]).unwrap_or_default();
    let read_rc = storage_mgr::read_block(page_num, &mut bp.fhl, &mut buf);
    if read_rc != RC::Ok && read_rc != RC::ReadNonExistingPage {
        // If it's not a recoverable read failure, we still try to add an empty page
    }

    if bp.free_space > 0 {
        let mem_address = used;
        let record_pointer = mem_address * PAGE_SIZE as usize;
        // Write buf into pagedata at record_pointer
        write_page_to_pagedata(&mut bp.pagedata, record_pointer, &buf);
        bp.free_space -= 1;
        bp.updated_order[mem_address] = page_num;
        bp.pagenum[mem_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[mem_address] += 1;
        bp.bitdirty[mem_address] = false;
        page.page_num = page_num;
        page.data = read_page_str(&bp.pagedata, record_pointer);
        return RC::Ok;
    }

    // Buffer pool is full. Need to evict.
    if bp.updated_strategy == 0 || bp.updated_strategy == 1 {
        // FIFO or LRU - evict oldest in updated_order with fix_count=0
        let total = bp.total_pages as usize;
        let mut found = false;
        let mut mem_address: usize = 0;
        let mut swap_location: usize = 0;
        'outer: for j in 0..total {
            let swap_page = bp.updated_order[j];
            for i in 0..total {
                if bp.pagenum[i] == swap_page && bp.fix_count[i] == 0 {
                    mem_address = i;
                    let record_pointer = i * PAGE_SIZE as usize;
                    if bp.bitdirty[i] {
                        let rc = storage_mgr::ensure_capacity(bp.pagenum[i] + 1, &mut bp.fhl);
                        if rc != RC::Ok {
                            return rc;
                        }
                        let page_str = read_page_str(&bp.pagedata, record_pointer);
                        let _ = storage_mgr::write_block(bp.pagenum[i], &mut bp.fhl, &page_str);
                        bp.num_write += 1;
                    }
                    swap_location = j;
                    found = true;
                    break 'outer;
                }
            }
        }
        if !found {
            return RC::BufferpoolFull;
        }
        let record_pointer = mem_address * PAGE_SIZE as usize;
        write_page_to_pagedata(&mut bp.pagedata, record_pointer, &buf);
        // Shift updated_order
        let end = total - 1;
        for k in swap_location..end {
            bp.updated_order[k] = bp.updated_order[k + 1];
        }
        bp.updated_order[end] = page_num;
        bp.pagenum[mem_address] = page_num;
        bp.num_read += 1;
        bp.fix_count[mem_address] += 1;
        bp.bitdirty[mem_address] = false;
        page.page_num = page_num;
        page.data = read_page_str(&bp.pagedata, record_pointer);
        return RC::Ok;
    }

    RC::BufferpoolFull
}

fn write_page_to_pagedata(pagedata: &mut String, offset: usize, buf: &str) {
    // Replace the bytes from offset..offset+PAGE_SIZE in pagedata with buf bytes (zero-padded)
    let buf_bytes = buf.as_bytes();
    let mut bytes = pagedata.as_bytes().to_vec();
    if bytes.len() < offset + PAGE_SIZE as usize {
        bytes.resize(offset + PAGE_SIZE as usize, 0);
    }
    let len = std::cmp::min(buf_bytes.len(), PAGE_SIZE as usize);
    bytes[offset..offset + len].copy_from_slice(&buf_bytes[..len]);
    // Zero-fill remainder of slot
    for b in &mut bytes[offset + len..offset + PAGE_SIZE as usize] {
        *b = 0;
    }
    *pagedata = String::from_utf8_lossy(&bytes).to_string();
}

fn shift_updated_order(start: i32, end: i32, bm: &mut BM_BufferPool, page_num: i32) {
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(bp) => bp,
        None => return,
    };
    let s = start as usize;
    let e = end as usize;
    for i in s..e {
        bp.updated_order[i] = bp.updated_order[i + 1];
    }
    bp.updated_order[e] = page_num;
}

fn update_bufferpool_stats(bm: &mut BM_BufferPool, address: i32, page_num: i32) {
    let mgmt = match bm.mgmt_data.as_mut() {
        Some(m) => m,
        None => return,
    };
    let bp = match mgmt.downcast_mut::<Bufferpool>() {
        Some(bp) => bp,
        None => return,
    };
    let i = address as usize;
    bp.pagenum[i] = page_num;
    bp.num_read += 1;
    bp.fix_count[i] += 1;
    bp.bitdirty[i] = false;
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    let mgmt = match bm.mgmt_data.as_ref() {
        Some(m) => m,
        None => return vec![],
    };
    let bp = match mgmt.downcast_ref::<Bufferpool>() {
        Some(bp) => bp,
        None => return vec![],
    };
    bp.pagenum.clone()
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    let mgmt = match bm.mgmt_data.as_ref() {
        Some(m) => m,
        None => return vec![],
    };
    let bp = match mgmt.downcast_ref::<Bufferpool>() {
        Some(bp) => bp,
        None => return vec![],
    };
    bp.bitdirty.clone()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    let mgmt = match bm.mgmt_data.as_ref() {
        Some(m) => m,
        None => return vec![],
    };
    let bp = match mgmt.downcast_ref::<Bufferpool>() {
        Some(bp) => bp,
        None => return vec![],
    };
    bp.fix_count.clone()
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    let mgmt = match bm.mgmt_data.as_ref() {
        Some(m) => m,
        None => return 0,
    };
    let bp = match mgmt.downcast_ref::<Bufferpool>() {
        Some(bp) => bp,
        None => return 0,
    };
    bp.num_read
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    let mgmt = match bm.mgmt_data.as_ref() {
        Some(m) => m,
        None => return 0,
    };
    let bp = match mgmt.downcast_ref::<Bufferpool>() {
        Some(bp) => bp,
        None => return 0,
    };
    bp.num_write
}
