use crate::dberror::RC;
use crate::storage_mgr::SM_FileHandle;
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

pub fn init_buffer_pool(bm: &mut BM_BufferPool, page_file_name: &str, num_pages: i32, strategy: ReplacementStrategy, _strat_data: Option<Box<dyn std::any::Any>>) -> RC {
    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = None;
    RC::Ok
}

pub fn shutdown_buffer_pool(_bm: &mut BM_BufferPool) -> RC {
    RC::Ok
}

pub fn force_flush_pool(_bm: &mut BM_BufferPool) -> RC {
    RC::Ok
}

pub fn mark_dirty(_bm: &mut BM_BufferPool, _page: &mut BM_PageHandle) -> RC {
    RC::Ok
}

pub fn unpin_page(_bm: &mut BM_BufferPool, _page: &mut BM_PageHandle) -> RC {
    RC::Ok
}

pub fn force_page(_bm: &mut BM_BufferPool, _page: &mut BM_PageHandle) -> RC {
    RC::Ok
}

pub fn pin_page(_bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    page.page_num = page_num;
    RC::Ok
}

#[allow(dead_code)]
fn shift_updated_order(_start: i32, _end: i32, _bm: &mut BM_BufferPool, _page_num: i32) {
    // No-op
}

#[allow(dead_code)]
fn update_bufferpool_stats(_bm: &mut BM_BufferPool, _address: i32, _page_num: i32) {
    // No-op
}

pub fn get_frame_contents(_bm: &BM_BufferPool) -> Vec<PageNumber> {
    Vec::new()
}

pub fn get_dirty_flags(_bm: &BM_BufferPool) -> Vec<bool> {
    Vec::new()
}

pub fn get_fix_counts(_bm: &BM_BufferPool) -> Vec<i32> {
    Vec::new()
}

pub fn get_num_read_io(_bm: &BM_BufferPool) -> i32 {
    0
}

pub fn get_num_write_io(_bm: &BM_BufferPool) -> i32 {
    0
}
