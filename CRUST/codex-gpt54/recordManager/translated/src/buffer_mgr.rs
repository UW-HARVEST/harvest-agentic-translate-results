use crate::dberror::RC;
use crate::dberror::PAGE_SIZE;
use crate::storage_mgr::{ensure_capacity, open_page_file, read_block, write_block, close_page_file, SM_FileHandle};
use crate::tables::{bytes_from_string, string_from_bytes};

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
pub data: String
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
struct BufferPoolState {
    meta: Bufferpool,
    frames: Vec<Vec<u8>>,
}

pub fn init_buffer_pool(bm: &mut BM_BufferPool, page_file_name: &str, num_pages: i32, strategy: ReplacementStrategy, strat_data: Option<Box<dyn std::any::Any>>) -> RC {
    let strategy_code = strategy_code(&strategy);
    let mut file_handle = SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    };
    let rc = open_page_file(page_file_name, &mut file_handle);
    if rc != RC::Ok {
        return rc;
    }

    let _ = strat_data;
    let state = BufferPoolState {
        meta: Bufferpool {
            num_read: 0,
            num_write: 0,
            total_pages: num_pages,
            updated_strategy: strategy_code,
            free_space: num_pages,
            updated_order: vec![NO_PAGE; num_pages as usize],
            bitdirty: vec![false; num_pages as usize],
            fix_count: vec![0; num_pages as usize],
            access_time: vec![0; num_pages as usize],
            pagenum: vec![NO_PAGE; num_pages as usize],
            pagedata: String::new(),
            fhl: file_handle,
        },
        frames: vec![vec![0_u8; PAGE_SIZE as usize]; num_pages as usize],
    };

    bm.page_file = page_file_name.to_string();
    bm.num_pages = num_pages;
    bm.strategy = strategy;
    bm.mgmt_data = Some(Box::new(state));
    RC::Ok
}
pub fn shutdown_buffer_pool(bm: &mut BM_BufferPool) -> RC {
    let Some(state) = get_state_mut(bm) else {
        return RC::Error;
    };
    if state.meta.fix_count.iter().any(|count| *count != 0) {
        return RC::BufferpoolInUse;
    }
    let flush_rc = force_flush_pool(bm);
    if flush_rc != RC::Ok {
        return flush_rc;
    }
    let Some(state) = get_state_mut(bm) else {
        return RC::Error;
    };
    let close_rc = close_page_file(&mut state.meta.fhl);
    if close_rc != RC::Ok {
        return close_rc;
    }
    bm.mgmt_data = None;
    RC::Ok
}
pub fn force_flush_pool(bm: &mut BM_BufferPool) -> RC {
    let Some(state) = get_state_mut(bm) else {
        return RC::Error;
    };
    for index in 0..state.meta.total_pages as usize {
        if state.meta.fix_count[index] == 0 && state.meta.bitdirty[index] && state.meta.pagenum[index] != NO_PAGE {
            let page_num = state.meta.pagenum[index];
            let rc = ensure_capacity(page_num + 1, &mut state.meta.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let page = string_from_bytes(state.frames[index].clone());
            let rc = write_block(page_num, &mut state.meta.fhl, &page);
            if rc != RC::Ok {
                return rc;
            }
            state.meta.bitdirty[index] = false;
            state.meta.num_write += 1;
        }
    }
    RC::Ok
}
pub fn mark_dirty(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let Some(state) = get_state_mut(bm) else {
        return RC::Error;
    };
    if let Some(index) = state.meta.pagenum.iter().position(|page_num| *page_num == page.page_num) {
        state.frames[index] = page_bytes(&page.data);
        state.frames[index].resize(PAGE_SIZE as usize, 0);
        state.meta.bitdirty[index] = true;
    }
    RC::Ok
}
pub fn unpin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let Some(state) = get_state_mut(bm) else {
        return RC::Error;
    };
    if let Some(index) = state.meta.pagenum.iter().position(|page_num| *page_num == page.page_num) {
        state.frames[index] = page_bytes(&page.data);
        state.frames[index].resize(PAGE_SIZE as usize, 0);
        if state.meta.fix_count[index] > 0 {
            state.meta.fix_count[index] -= 1;
        }
    }
    RC::Ok
}
pub fn force_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle) -> RC {
    let Some(state) = get_state_mut(bm) else {
        return RC::Error;
    };
    if let Some(index) = state.meta.pagenum.iter().position(|page_num| *page_num == page.page_num) {
        state.frames[index] = page_bytes(&page.data);
        state.frames[index].resize(PAGE_SIZE as usize, 0);
        let rc = ensure_capacity(page.page_num + 1, &mut state.meta.fhl);
        if rc != RC::Ok {
            return rc;
        }
        let data = string_from_bytes(state.frames[index].clone());
        let rc = write_block(page.page_num, &mut state.meta.fhl, &data);
        if rc != RC::Ok {
            return rc;
        }
        state.meta.bitdirty[index] = false;
        state.meta.num_write += 1;
        return RC::Ok;
    }
    RC::WriteFailed
}
pub fn pin_page(bm: &mut BM_BufferPool, page: &mut BM_PageHandle, page_num: PageNumber) -> RC {
    let strategy = strategy_code(&bm.strategy);
    let Some(state) = get_state_mut(bm) else {
        return RC::Error;
    };

    if let Some(index) = state.meta.pagenum.iter().position(|loaded| *loaded == page_num) {
        state.meta.fix_count[index] += 1;
        if strategy == ReplacementStrategy::RsLru as i32 {
            move_page_to_end(&mut state.meta.updated_order, page_num);
        }
        page.page_num = page_num;
        page.data = string_from_bytes(state.frames[index].clone());
        return RC::Ok;
    }

    let mut page_data = String::new();
    let read_rc = read_block(page_num, &mut state.meta.fhl, &mut page_data);
    if read_rc != RC::Ok {
        page_data = string_from_bytes(vec![0_u8; PAGE_SIZE as usize]);
    }
    let page_bytes = page_bytes(&page_data);

    let target_index = if state.meta.free_space > 0 {
        (state.meta.total_pages - state.meta.free_space) as usize
    } else if let Some(victim_index) = select_victim(state) {
        let victim_page = state.meta.pagenum[victim_index];
        if state.meta.bitdirty[victim_index] && victim_page != NO_PAGE {
            let rc = ensure_capacity(victim_page + 1, &mut state.meta.fhl);
            if rc != RC::Ok {
                return rc;
            }
            let data = string_from_bytes(state.frames[victim_index].clone());
            let rc = write_block(victim_page, &mut state.meta.fhl, &data);
            if rc != RC::Ok {
                return rc;
            }
            state.meta.num_write += 1;
            state.meta.bitdirty[victim_index] = false;
        }
        victim_index
    } else {
        return RC::BufferpoolFull;
    };

    state.frames[target_index] = page_bytes;
    state.frames[target_index].resize(PAGE_SIZE as usize, 0);
    state.meta.pagenum[target_index] = page_num;
    state.meta.fix_count[target_index] = 1;
    state.meta.bitdirty[target_index] = false;
    state.meta.num_read += 1;
    if state.meta.free_space > 0 {
        state.meta.free_space -= 1;
    }

    if let Some(position) = state.meta.updated_order.iter().position(|value| *value == page_num) {
        for idx in position..state.meta.updated_order.len().saturating_sub(1) {
            state.meta.updated_order[idx] = state.meta.updated_order[idx + 1];
        }
    }
    if let Some(slot) = state.meta.updated_order.iter().position(|value| *value == NO_PAGE) {
        state.meta.updated_order[slot] = page_num;
    } else if !state.meta.updated_order.is_empty() {
        for idx in 0..state.meta.updated_order.len() - 1 {
            state.meta.updated_order[idx] = state.meta.updated_order[idx + 1];
        }
        let last = state.meta.updated_order.len() - 1;
        state.meta.updated_order[last] = page_num;
    }

    page.page_num = page_num;
    page.data = string_from_bytes(state.frames[target_index].clone());
    RC::Ok
}
fn shift_updated_order(start: i32, end: i32, bm: &mut BM_BufferPool, page_num: i32){
    if let Some(state) = get_state_mut(bm) {
        if start < 0 || end < start || end as usize >= state.meta.updated_order.len() {
            return;
        }
        for index in start as usize..end as usize {
            state.meta.updated_order[index] = state.meta.updated_order[index + 1];
        }
        state.meta.updated_order[end as usize] = page_num;
    }
}
fn update_bufferpool_stats(bm: &mut BM_BufferPool, address: i32, page_num: i32){
    if let Some(state) = get_state_mut(bm) {
        let index = address as usize;
        state.meta.pagenum[index] = page_num;
        state.meta.num_read += 1;
        state.meta.fix_count[index] += 1;
        state.meta.bitdirty[index] = false;
    }
}
pub fn get_frame_contents(bm: &BM_BufferPool) -> Vec<PageNumber> {
    get_state(bm).map(|state| state.meta.pagenum.clone()).unwrap_or_default()
}
pub fn get_dirty_flags(bm: &BM_BufferPool) -> Vec<bool> {
    get_state(bm).map(|state| state.meta.bitdirty.clone()).unwrap_or_default()
}
pub fn get_fix_counts(bm: &BM_BufferPool) -> Vec<i32> {
    get_state(bm).map(|state| state.meta.fix_count.clone()).unwrap_or_default()
}
pub fn get_num_read_io(bm: &BM_BufferPool) -> i32 {
    get_state(bm).map(|state| state.meta.num_read).unwrap_or(0)
}
pub fn get_num_write_io(bm: &BM_BufferPool) -> i32 {
    get_state(bm).map(|state| state.meta.num_write).unwrap_or(0)
}

fn get_state_mut(bm: &mut BM_BufferPool) -> Option<&mut BufferPoolState> {
    bm.mgmt_data.as_mut()?.downcast_mut::<BufferPoolState>()
}

fn get_state(bm: &BM_BufferPool) -> Option<&BufferPoolState> {
    bm.mgmt_data.as_ref()?.downcast_ref::<BufferPoolState>()
}

fn move_page_to_end(order: &mut [i32], page_num: i32) {
    if let Some(position) = order.iter().position(|value| *value == page_num) {
        for index in position..order.len().saturating_sub(1) {
            order[index] = order[index + 1];
        }
        if let Some(last) = order.last_mut() {
            *last = page_num;
        }
    }
}

fn select_victim(state: &BufferPoolState) -> Option<usize> {
    for page_num in &state.meta.updated_order {
        if *page_num == NO_PAGE {
            continue;
        }
        if let Some(index) = state.meta.pagenum.iter().position(|loaded_page_num| *loaded_page_num == *page_num) {
            if state.meta.fix_count[index] == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn page_bytes(data: &str) -> Vec<u8> {
    let mut bytes = bytes_from_string(data);
    bytes.resize(PAGE_SIZE as usize, 0);
    bytes
}

fn strategy_code(strategy: &ReplacementStrategy) -> i32 {
    match strategy {
        ReplacementStrategy::RsFifo => 0,
        ReplacementStrategy::RsLru => 1,
        ReplacementStrategy::RsClock => 2,
        ReplacementStrategy::RsLfu => 3,
        ReplacementStrategy::RsLruK => 4,
    }
}
