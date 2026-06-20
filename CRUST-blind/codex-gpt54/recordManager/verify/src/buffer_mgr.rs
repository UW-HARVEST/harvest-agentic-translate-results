use crate::dberror::RC;
use crate::storage_mgr::{
    close_page_file, ensure_capacity, open_page_file, read_block, write_block, SM_FileHandle,
};
use crate::tables::{bytes_to_data, data_to_bytes, ensure_byte_len};

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

struct InternalBufferPool {
    num_read: i32,
    num_write: i32,
    total_pages: i32,
    updated_strategy: ReplacementStrategy,
    free_space: i32,
    updated_order: Vec<i32>,
    bitdirty: Vec<bool>,
    fix_count: Vec<i32>,
    pagenum: Vec<i32>,
    frames: Vec<Vec<u8>>,
    fhl: SM_FileHandle,
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

pub struct BM_BufferPool {
    pub page_file: String,
    pub num_pages: i32,
    pub strategy: ReplacementStrategy,
    pub mgmt_data: Option<Box<dyn std::any::Any>>,
}

fn pool_mut(bm: &mut BM_BufferPool) -> Result<&mut InternalBufferPool, RC> {
    bm.mgmt_data
        .as_mut()
        .and_then(|data| data.downcast_mut::<InternalBufferPool>())
        .ok_or(RC::FileHandleNotInit)
}

fn pool_ref(bm: &BM_BufferPool) -> Option<&InternalBufferPool> {
    bm.mgmt_data
        .as_ref()
        .and_then(|data| data.downcast_ref::<InternalBufferPool>())
}

fn sync_page_back(bp: &mut InternalBufferPool, page: &BM_PageHandle) {
    if let Some(idx) = bp.pagenum.iter().position(|num| *num == page.page_num) {
        let mut bytes = data_to_bytes(&page.data);
        ensure_byte_len(&mut bytes, crate::dberror::PAGE_SIZE as usize);
        bp.frames[idx] = bytes;
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

    let total = num_pages.max(0) as usize;
    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(InternalBufferPool {
        num_read: 0,
        num_write: 0,
        total_pages: num_pages,
        updated_strategy: strategy,
        free_space: num_pages,
        updated_order: vec![NO_PAGE; total],
        bitdirty: vec![false; total],
        fix_count: vec![0; total],
        pagenum: vec![NO_PAGE; total],
        frames: vec![vec![0; crate::dberror::PAGE_SIZE as usize]; total],
        fhl: fh,
    }));
    RC::Ok
}

pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    {
        let bp = match pool_mut(bm) {
            Ok(bp) => bp,
            Err(rc) => return rc,
        };
        if bp.fix_count.iter().any(|count| *count != 0) {
            return RC::BufferpoolInUse;
        }
    }

    let rc = force_flush_pool(bm);
    if rc != RC::Ok {
        return rc;
    }

    let bp = match pool_mut(bm) {
        Ok(bp) => bp,
        Err(rc) => return rc,
    };
    let rc = close_page_file(&mut bp.fhl);
    if rc != RC::Ok {
        return RC::CloseFailed;
    }
    bm.mgmt_data = None;
    RC::Ok
}

pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let bp = match pool_mut(bm) {
        Ok(bp) => bp,
        Err(rc) => return rc,
    };

    for idx in 0..bp.total_pages as usize {
        if bp.fix_count[idx] == 0 && bp.bitdirty[idx] && bp.pagenum[idx] != NO_PAGE {
            let rc = ensure_capacity(bp.pagenum[idx] + 1, &mut bp.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let frame = bytes_to_data(&bp.frames[idx]);
            let rc = write_block(bp.pagenum[idx], &mut bp.fhl, &frame);
            if rc != RC::Ok {
                return RC::WriteFailed;
            }
            bp.bitdirty[idx] = false;
            bp.num_write += 1;
        }
    }
    RC::Ok
}

pub fn mark_dirty(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = match pool_mut(bm) {
        Ok(bp) => bp,
        Err(rc) => return rc,
    };
    sync_page_back(bp, page);
    if let Some(idx) = bp.pagenum.iter().position(|num| *num == page.page_num) {
        bp.bitdirty[idx] = true;
    }
    RC::Ok
}

pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = match pool_mut(bm) {
        Ok(bp) => bp,
        Err(rc) => return rc,
    };
    sync_page_back(bp, page);
    if let Some(idx) = bp.pagenum.iter().position(|num| *num == page.page_num) {
        if bp.fix_count[idx] > 0 {
            bp.fix_count[idx] -= 1;
        }
    }
    RC::Ok
}

pub fn force_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let bp = match pool_mut(bm) {
        Ok(bp) => bp,
        Err(rc) => return rc,
    };
    sync_page_back(bp, page);
    if let Some(idx) = bp.pagenum.iter().position(|num| *num == page.page_num) {
        let rc = ensure_capacity(bp.pagenum[idx] + 1, &mut bp.fhl);
        if rc != RC::Ok {
            return rc;
        }
        let frame = bytes_to_data(&bp.frames[idx]);
        let rc = write_block(bp.pagenum[idx], &mut bp.fhl, &frame);
        if rc != RC::Ok {
            return rc;
        }
        bp.bitdirty[idx] = false;
        bp.num_write += 1;
        return RC::Ok;
    }
    RC::WriteFailed
}

pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let bp = match pool_mut(bm) {
        Ok(bp) => bp,
        Err(rc) => return rc,
    };

    if let Some(idx) = bp.pagenum.iter().position(|num| *num == page_num) {
        bp.fix_count[idx] += 1;
        page.page_num = page_num;
        page.data = bytes_to_data(&bp.frames[idx]);
        if matches!(bp.updated_strategy, ReplacementStrategy::RsLru) {
            if let Some(pos) = bp.updated_order.iter().position(|num| *num == page_num) {
                shift_updated_order(pos as i32, bp.total_pages - bp.free_space - 1, bm, page_num);
            }
        }
        return RC::Ok;
    }

    let mut disk_page = String::new();
    let read_rc = read_block(page_num, &mut bp.fhl, &mut disk_page);
    let mut loaded = if read_rc == RC::Ok {
        data_to_bytes(&disk_page)
    } else {
        vec![0; crate::dberror::PAGE_SIZE as usize]
    };
    ensure_byte_len(&mut loaded, crate::dberror::PAGE_SIZE as usize);

    if bp.free_space > 0 {
        let idx = (bp.total_pages - bp.free_space) as usize;
        bp.frames[idx] = loaded;
        bp.free_space -= 1;
        bp.updated_order[idx] = page_num;
        bp.pagenum[idx] = page_num;
        bp.num_read += 1;
        bp.fix_count[idx] += 1;
        bp.bitdirty[idx] = false;
        page.page_num = page_num;
        page.data = bytes_to_data(&bp.frames[idx]);
        return RC::Ok;
    }

    let used = bp.total_pages as usize;
    let mut victim_order = None;
    let mut victim_idx = 0usize;
    for order_idx in 0..used {
        let candidate = bp.updated_order[order_idx];
        if let Some(frame_idx) = bp.pagenum.iter().position(|num| *num == candidate) {
            if bp.fix_count[frame_idx] == 0 {
                victim_order = Some(order_idx);
                victim_idx = frame_idx;
                break;
            }
        }
    }

    let Some(order_idx) = victim_order else {
        return RC::BufferpoolFull;
    };

    if bp.bitdirty[victim_idx] && bp.pagenum[victim_idx] != NO_PAGE {
        let rc = ensure_capacity(bp.pagenum[victim_idx] + 1, &mut bp.fhl);
        if rc != RC::Ok {
            return rc;
        }
        let frame = bytes_to_data(&bp.frames[victim_idx]);
        let rc = write_block(bp.pagenum[victim_idx], &mut bp.fhl, &frame);
        if rc != RC::Ok {
            return RC::WriteFailed;
        }
        bp.num_write += 1;
    }

    bp.frames[victim_idx] = loaded;
    for i in order_idx..bp.total_pages as usize - 1 {
        bp.updated_order[i] = bp.updated_order[i + 1];
    }
    bp.updated_order[bp.total_pages as usize - 1] = page_num;
    bp.pagenum[victim_idx] = page_num;
    bp.num_read += 1;
    bp.fix_count[victim_idx] += 1;
    bp.bitdirty[victim_idx] = false;
    page.page_num = page_num;
    page.data = bytes_to_data(&bp.frames[victim_idx]);
    RC::Ok
}

fn shift_updated_order(start: i32, end: i32, bm: &mut BM_BufferPool, page_num: i32) {
    if let Ok(bp) = pool_mut(bm) {
        for i in start as usize..end as usize {
            bp.updated_order[i] = bp.updated_order[i + 1];
        }
        bp.updated_order[end as usize] = page_num;
    }
}

pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    pool_ref(bm)
        .map(|bp| bp.pagenum.clone())
        .unwrap_or_default()
}

pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    pool_ref(bm)
        .map(|bp| bp.bitdirty.clone())
        .unwrap_or_default()
}

pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    pool_ref(bm)
        .map(|bp| bp.fix_count.clone())
        .unwrap_or_default()
}

pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    pool_ref(bm).map(|bp| bp.num_read).unwrap_or(0)
}

pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    pool_ref(bm).map(|bp| bp.num_write).unwrap_or(0)
}
