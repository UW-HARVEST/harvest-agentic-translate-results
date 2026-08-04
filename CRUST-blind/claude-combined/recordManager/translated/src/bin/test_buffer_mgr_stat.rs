use recordManager::buffer_mgr::{
    BM_BufferPool, BM_PageHandle, ReplacementStrategy,
    init_buffer_pool, shutdown_buffer_pool, pin_page, unpin_page, mark_dirty,
};
use recordManager::buffer_mgr_stat::{sprint_pool_content, sprint_page_content, print_strat,
                                      print_pool_content, print_page_content};
use recordManager::storage_mgr::{create_page_file, destroy_page_file, ensure_capacity, open_page_file, close_page_file, SM_FileHandle};
use recordManager::dberror::PAGE_SIZE;

fn make_test_file(path: &str, num_pages: i32) {
    let _ = std::fs::remove_file(path);
    let _ = create_page_file(path);
    let mut h = SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    };
    let _ = open_page_file(path, &mut h);
    let _ = ensure_capacity(num_pages, &mut h);
    let _ = close_page_file(&mut h);
}

#[test]
fn test_sprint_pool_content_empty() {
    let path = "/tmp/bms_test_empty.bin";
    make_test_file(path, 1);
    let mut bm = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    let s = sprint_pool_content(&bm);
    // C: get_fix_counts returns &noFixes (a single int with 0). My impl returns vec![0].
    // Expected per page: "[<page>< or x><fix>]"
    // For empty pool: frames = -1,-1,-1; dirty = false,false,false; fix may be [0] (single).
    // C iterates from 0 to numPages, accessing fix[i] where fix is &noFixes (single int).
    // That's UB technically - it reads garbage past first int. Our equivalent returns 0
    // for missing entries via .get().unwrap_or(0).
    assert_eq!(s, "[-1 0],[-1 0],[-1 0]");
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

#[test]
fn test_sprint_pool_content_after_pin() {
    let path = "/tmp/bms_test_pin.bin";
    make_test_file(path, 3);
    let mut bm = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    let mut p = BM_PageHandle { page_num: -1, data: String::new() };
    let _ = pin_page(&mut bm, &mut p, 0);
    let s = sprint_pool_content(&bm);
    // First frame has page 0, fix count 1, not dirty
    assert!(s.starts_with("[0 1]"));
    let _ = mark_dirty(&mut bm, &mut p);
    let s = sprint_pool_content(&bm);
    assert!(s.starts_with("[0x1]"));
    let _ = unpin_page(&mut bm, &mut p);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

#[test]
fn test_sprint_page_content_format() {
    let mut p = BM_PageHandle { page_num: 7, data: String::new() };
    let bytes: Vec<u8> = (0..PAGE_SIZE as usize + 1).map(|i| (i as u8)).collect();
    p.data = bytes.iter().map(|&b| b as char).collect();
    let s = sprint_page_content(&p);
    // Should start with "[Page 7]\n"
    assert!(s.starts_with("[Page 7]\n"));
    // First byte printed is byte at index 1 (because i starts at 1 in C)
    // index 1 = 1 -> "01"
    assert!(s.contains("01"));
}

#[test]
fn test_print_strat_compiles() {
    let path = "/tmp/bms_test_strat.bin";
    make_test_file(path, 1);
    let mut bm = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    print_strat(&bm);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

#[test]
fn test_print_pool_compiles() {
    let path = "/tmp/bms_test_print.bin";
    make_test_file(path, 1);
    let mut bm = BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    };
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    print_pool_content(&bm);
    let p = BM_PageHandle { page_num: 0, data: (0..PAGE_SIZE as usize + 1).map(|i| (i as u8) as char).collect() };
    print_page_content(&p);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

fn main() {}
