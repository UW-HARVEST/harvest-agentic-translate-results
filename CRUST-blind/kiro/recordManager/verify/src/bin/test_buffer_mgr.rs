use recordManager::buffer_mgr::*;
use recordManager::storage_mgr::*;
use recordManager::dberror::RC;
use std::fs;

fn unique_file(name: &str) -> String {
    format!("/tmp/test_bm_{}", name)
}

fn setup_file(name: &str) -> String {
    let f = unique_file(name);
    let _ = fs::remove_file(&f);
    create_page_file(&f);
    // Need to write page count to header
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0, cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    // Append a data page so totalNumPages becomes 1
    append_empty_block(&mut fh);
    close_page_file(&mut fh);
    f
}

#[test]
fn test_init_and_shutdown_buffer_pool() {
    let f = setup_file("init_shutdown");
    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    let rc = init_buffer_pool(&mut bm, &f, 3, ReplacementStrategy::RsFifo, None);
    assert_eq!(rc, RC::Ok);
    assert_eq!(bm.num_pages, 3);
    let rc = shutdown_buffer_pool(&mut bm);
    assert_eq!(rc, RC::Ok);
    destroy_page_file(&f);
}

#[test]
fn test_pin_and_unpin_page() {
    let f = setup_file("pin_unpin");
    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    init_buffer_pool(&mut bm, &f, 3, ReplacementStrategy::RsFifo, None);
    let mut ph = BM_PageHandle { page_num: NO_PAGE, data: String::new() };
    let rc = pin_page(&mut bm, &mut ph, 0);
    assert_eq!(rc, RC::Ok);
    assert_eq!(ph.page_num, 0);
    let rc = unpin_page(&mut bm, &mut ph);
    assert_eq!(rc, RC::Ok);
    shutdown_buffer_pool(&mut bm);
    destroy_page_file(&f);
}

#[test]
fn test_get_num_read_write_io() {
    let f = setup_file("io_counts");
    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    init_buffer_pool(&mut bm, &f, 3, ReplacementStrategy::RsFifo, None);
    assert_eq!(get_num_read_io(&bm), 0);
    assert_eq!(get_num_write_io(&bm), 0);
    let mut ph = BM_PageHandle { page_num: NO_PAGE, data: String::new() };
    pin_page(&mut bm, &mut ph, 0);
    assert_eq!(get_num_read_io(&bm), 1);
    unpin_page(&mut bm, &mut ph);
    shutdown_buffer_pool(&mut bm);
    destroy_page_file(&f);
}

#[test]
fn test_get_frame_contents_empty() {
    let f = setup_file("frame_empty");
    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    init_buffer_pool(&mut bm, &f, 3, ReplacementStrategy::RsFifo, None);
    let frames = get_frame_contents(&bm);
    assert!(frames.is_empty());
    shutdown_buffer_pool(&mut bm);
    destroy_page_file(&f);
}

#[test]
fn test_mark_dirty() {
    let f = setup_file("mark_dirty");
    let mut bm = BM_BufferPool {
        page_file: String::new(), num_pages: 0,
        strategy: ReplacementStrategy::RsFifo, mgmt_data: None,
    };
    init_buffer_pool(&mut bm, &f, 3, ReplacementStrategy::RsFifo, None);
    let mut ph = BM_PageHandle { page_num: NO_PAGE, data: String::new() };
    pin_page(&mut bm, &mut ph, 0);
    let rc = mark_dirty(&mut bm, &mut ph);
    assert_eq!(rc, RC::Ok);
    let dirty = get_dirty_flags(&bm);
    assert!(dirty[0]);
    unpin_page(&mut bm, &mut ph);
    shutdown_buffer_pool(&mut bm);
    destroy_page_file(&f);
}

fn main() {}
