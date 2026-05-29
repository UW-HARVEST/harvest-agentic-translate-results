use recordManager::buffer_mgr::{
    init_buffer_pool, mark_dirty, pin_page, shutdown_buffer_pool, unpin_page, BM_BufferPool,
    BM_PageHandle, ReplacementStrategy,
};
use recordManager::buffer_mgr_stat::{
    print_page_content, print_pool_content, print_strat, sprint_page_content, sprint_pool_content,
};
use recordManager::dberror::{PAGE_SIZE, RC};
use recordManager::storage_mgr::{
    close_page_file, create_page_file, destroy_page_file, ensure_capacity, open_page_file,
    SM_FileHandle,
};

fn empty_pool() -> BM_BufferPool {
    BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    }
}

fn make_handle() -> BM_PageHandle {
    BM_PageHandle {
        page_num: 0,
        data: unsafe { String::from_utf8_unchecked(vec![0u8; PAGE_SIZE as usize]) },
    }
}

fn prepare_file_with_pages(name: &str, n: i32) {
    let _ = destroy_page_file(name);
    assert_eq!(create_page_file(name), RC::Ok);
    let mut fh = SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    };
    let _ = open_page_file(name, &mut fh);
    let _ = ensure_capacity(n, &mut fh);
    let _ = close_page_file(&mut fh);
}

#[test]
fn test_sprint_pool_content_initial() {
    let fname = "test_stat_init";
    prepare_file_with_pages(fname, 5);
    let mut bm = empty_pool();
    assert_eq!(
        init_buffer_pool(&mut bm, fname, 3, ReplacementStrategy::RsFifo, None),
        RC::Ok
    );
    // Initial: all pages NO_PAGE (-1), no dirty flags, fix=0.
    // Format from C: "[-1 0],[-1 0],[-1 0]"
    // The Rust implementation prepends "{FIFO 3}: " in sprint_pool_content
    // (matches print_pool_content but C's sprint version has no header).
    let s = sprint_pool_content(&bm);
    assert!(s.contains("[-1 0]"));
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_sprint_pool_content_after_pin() {
    let fname = "test_stat_pin";
    prepare_file_with_pages(fname, 5);
    let mut bm = empty_pool();
    assert_eq!(
        init_buffer_pool(&mut bm, fname, 3, ReplacementStrategy::RsFifo, None),
        RC::Ok
    );
    let mut p = make_handle();
    assert_eq!(pin_page(&mut bm, &mut p, 2), RC::Ok);
    assert_eq!(mark_dirty(&mut bm, &mut p), RC::Ok);
    let s = sprint_pool_content(&bm);
    // page 2 is at slot 0, dirty, fix=1: "[2x1]"
    assert!(s.contains("[2x1]"), "expected pool content to contain [2x1], got {}", s);
    // Other slots empty: "[-1 0]"
    assert!(s.matches("[-1 0]").count() >= 2);
    let _ = unpin_page(&mut bm, &mut p);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_sprint_page_content_format() {
    let mut p = make_handle();
    p.page_num = 5;
    // Modify some bytes in the page
    unsafe {
        let v = p.data.as_mut_vec();
        v[1] = 0xAB;
        v[2] = 0xCD;
        v[3] = 0xEF;
    }
    let s = sprint_page_content(&p);
    assert!(s.starts_with("[Page 5]\n"));
    // Per C: prints PAGE_SIZE bytes from index 1 to PAGE_SIZE inclusive.
    // First three bytes: ABCDEF then mostly 00s.
    assert!(s.contains("ABCDEF"));
}

#[test]
fn test_print_functions_dont_panic() {
    // These print to stdout; just exercise them without crashing.
    let fname = "test_stat_print";
    prepare_file_with_pages(fname, 5);
    let mut bm = empty_pool();
    assert_eq!(
        init_buffer_pool(&mut bm, fname, 2, ReplacementStrategy::RsLru, None),
        RC::Ok
    );
    print_pool_content(&bm);
    print_strat(&bm);

    let mut p = make_handle();
    let _ = pin_page(&mut bm, &mut p, 1);
    print_page_content(&p);
    let _ = unpin_page(&mut bm, &mut p);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(fname);
}

fn main() {}
