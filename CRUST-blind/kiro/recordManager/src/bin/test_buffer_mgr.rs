use recordManager::dberror::RC;
use recordManager::buffer_mgr::*;
use recordManager::storage_mgr;
use std::fs;

fn unique_file(name: &str) -> String {
    format!("/tmp/test_bmgr_{}_{}", name, std::process::id())
}

fn setup_pool(name: &str, num_pages: i32) -> (BM_BufferPool, String) {
    let f = unique_file(name);
    storage_mgr::create_page_file(&f);
    // Open and add some pages
    let mut fh = storage_mgr::SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    storage_mgr::open_page_file(&f, &mut fh);
    for _ in 0..3 {
        storage_mgr::append_empty_block(&mut fh);
    }
    storage_mgr::close_page_file(&mut fh);

    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut bm, &f, num_pages, ReplacementStrategy::RsFifo, None);
    assert_eq!(rc, RC::Ok);
    (bm, f)
}

#[test]
fn test_init_buffer_pool() {
    let (mut bm, f) = setup_pool("init", 3);
    assert_eq!(bm.num_pages, 3);
    assert_eq!(get_num_read_io(&bm), 0);
    assert_eq!(get_num_write_io(&bm), 0);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_init_buffer_pool_nonexistent() {
    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut bm, "/tmp/nonexistent_file_xyz", 3, ReplacementStrategy::RsFifo, None);
    assert_eq!(rc, RC::FileNotFound);
}

#[test]
fn test_pin_unpin_page() {
    let (mut bm, f) = setup_pool("pinunpin", 3);
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    let rc = pin_page(&mut bm, &mut page, 0);
    assert_eq!(rc, RC::Ok);
    assert_eq!(page.page_num, 0);

    let rc = unpin_page(&mut bm, &mut page);
    assert_eq!(rc, RC::Ok);

    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_mark_dirty() {
    let (mut bm, f) = setup_pool("dirty", 3);
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    pin_page(&mut bm, &mut page, 0);
    let rc = mark_dirty(&mut bm, &mut page);
    assert_eq!(rc, RC::Ok);

    let flags = get_dirty_flags(&bm);
    assert!(flags[0]);

    unpin_page(&mut bm, &mut page);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_force_page() {
    let (mut bm, f) = setup_pool("force", 3);
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    pin_page(&mut bm, &mut page, 0);
    let rc = force_page(&mut bm, &mut page);
    assert_eq!(rc, RC::Ok);
    unpin_page(&mut bm, &mut page);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_get_frame_contents_empty() {
    let (mut bm, f) = setup_pool("frameempty", 3);
    let contents = get_frame_contents(&bm);
    // When pool is empty (all free), returns empty vec
    assert!(contents.is_empty());
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_get_frame_contents_after_pin() {
    let (mut bm, f) = setup_pool("framepin", 3);
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    pin_page(&mut bm, &mut page, 0);
    let contents = get_frame_contents(&bm);
    assert!(!contents.is_empty());
    assert_eq!(contents[0], 0);
    unpin_page(&mut bm, &mut page);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_get_fix_counts_empty() {
    let (mut bm, f) = setup_pool("fixempty", 3);
    let counts = get_fix_counts(&bm);
    assert_eq!(counts, vec![0]);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_get_fix_counts_after_pin() {
    let (mut bm, f) = setup_pool("fixpin", 3);
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    pin_page(&mut bm, &mut page, 0);
    let counts = get_fix_counts(&bm);
    assert_eq!(counts[0], 1);
    unpin_page(&mut bm, &mut page);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_num_read_io() {
    let (mut bm, f) = setup_pool("readio", 3);
    assert_eq!(get_num_read_io(&bm), 0);
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    pin_page(&mut bm, &mut page, 0);
    assert_eq!(get_num_read_io(&bm), 1);
    unpin_page(&mut bm, &mut page);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_shutdown_with_pinned_page() {
    let (mut bm, f) = setup_pool("shutpin", 3);
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    pin_page(&mut bm, &mut page, 0);
    // Shutdown should fail because page is pinned
    let rc = shutdown_buffer_pool(&mut bm);
    assert_eq!(rc, RC::BufferpoolInUse);
    // Unpin and try again
    unpin_page(&mut bm, &mut page);
    let rc = shutdown_buffer_pool(&mut bm);
    assert_eq!(rc, RC::Ok);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_pin_same_page_twice() {
    let (mut bm, f) = setup_pool("pinsame", 3);
    let mut page = BM_PageHandle { page_num: 0, data: String::new() };
    pin_page(&mut bm, &mut page, 0);
    // Pin same page again - should increase fix count
    pin_page(&mut bm, &mut page, 0);
    let counts = get_fix_counts(&bm);
    assert_eq!(counts[0], 2);
    // Only 1 read IO since page was already in pool
    assert_eq!(get_num_read_io(&bm), 1);
    unpin_page(&mut bm, &mut page);
    unpin_page(&mut bm, &mut page);
    shutdown_buffer_pool(&mut bm);
    let _ = fs::remove_file(&f);
}

fn main() {}
