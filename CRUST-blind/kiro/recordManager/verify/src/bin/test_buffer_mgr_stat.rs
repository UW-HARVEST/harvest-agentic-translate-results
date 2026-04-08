use recordManager::buffer_mgr::*;
use recordManager::buffer_mgr_stat::*;
use recordManager::storage_mgr::*;
use recordManager::dberror::RC;
use std::fs;

fn unique_file(name: &str) -> String {
    format!("/tmp/test_bms_{}", name)
}

fn setup_file(name: &str) -> String {
    let f = unique_file(name);
    let _ = fs::remove_file(&f);
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0, cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    append_empty_block(&mut fh);
    close_page_file(&mut fh);
    f
}

#[test]
fn test_sprint_pool_content_empty() {
    let f = setup_file("sprint_empty");
    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    init_buffer_pool(&mut bm, &f, 3, ReplacementStrategy::RsFifo, None);
    // When pool is empty, getFrameContents returns empty vec, so sprint iterates 0..num_pages
    // but frame/dirty/fix are empty. The sprint function handles this with defaults.
    let _content = sprint_pool_content(&bm);
    shutdown_buffer_pool(&mut bm);
    destroy_page_file(&f);
}

#[test]
fn test_sprint_pool_content_with_page() {
    let f = setup_file("sprint_page");
    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    init_buffer_pool(&mut bm, &f, 3, ReplacementStrategy::RsFifo, None);
    let mut ph = BM_PageHandle { page_num: NO_PAGE, data: String::new() };
    pin_page(&mut bm, &mut ph, 0);
    let content = sprint_pool_content(&bm);
    // After pinning page 0: frame=[0,-1,-1], dirty=[false,false,false], fix=[1,0,0]
    assert_eq!(content, "[0 1],[-1 0],[-1 0]");
    unpin_page(&mut bm, &mut ph);
    shutdown_buffer_pool(&mut bm);
    destroy_page_file(&f);
}

fn main() {}
