use recordManager::buffer_mgr::*;
use recordManager::buffer_mgr_stat::*;
use recordManager::storage_mgr;
use recordManager::dberror::RC;
use std::fs;

fn unique_file(name: &str) -> String {
    format!("/tmp/test_bstat_{}_{}", name, std::process::id())
}

fn setup(name: &str) -> (BM_BufferPool, String) {
    let f = unique_file(name);
    storage_mgr::create_page_file(&f);
    let mut fh = storage_mgr::SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    storage_mgr::open_page_file(&f, &mut fh);
    storage_mgr::append_empty_block(&mut fh);
    storage_mgr::close_page_file(&mut fh);

    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    init_buffer_pool(&mut bm, &f, 3, ReplacementStrategy::RsFifo, None);
    (bm, f)
}

#[test]
fn test_sprint_pool_content_empty() {
    let (mut bm, f) = setup("sprintempty");
    let s = sprint_pool_content(&bm);
    // Should contain brackets for each page slot
    assert!(s.contains("["));
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_sprint_pool_content_with_page() {
    let (mut bm, f) = setup("sprintpage");
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    pin_page(&mut bm, &mut page, 0);
    let s = sprint_pool_content(&bm);
    // Should show page 0 with fix count 1
    assert!(s.contains("0"));
    unpin_page(&mut bm, &mut page);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_sprint_page_content() {
    let (mut bm, f) = setup("sprintpg");
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    pin_page(&mut bm, &mut page, 0);
    let s = sprint_page_content(&page);
    assert!(s.starts_with("[Page 0]"));
    unpin_page(&mut bm, &mut page);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

fn main() {}
