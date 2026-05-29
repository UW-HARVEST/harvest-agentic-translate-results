use recordManager::buffer_mgr::{
    force_flush_pool, force_page, get_dirty_flags, get_fix_counts, get_frame_contents,
    get_num_read_io, get_num_write_io, init_buffer_pool, mark_dirty, pin_page, shutdown_buffer_pool,
    unpin_page, BM_BufferPool, BM_PageHandle, ReplacementStrategy, NO_PAGE,
};
use recordManager::dberror::{PAGE_SIZE, RC};
use recordManager::storage_mgr::{create_page_file, destroy_page_file, ensure_capacity, open_page_file, close_page_file, SM_FileHandle};

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
fn test_replacement_strategy_codes() {
    assert_eq!(ReplacementStrategy::RsFifo as i32, 0);
    assert_eq!(ReplacementStrategy::RsLru as i32, 1);
    assert_eq!(ReplacementStrategy::RsClock as i32, 2);
    assert_eq!(ReplacementStrategy::RsLfu as i32, 3);
    assert_eq!(ReplacementStrategy::RsLruK as i32, 4);
}

#[test]
fn test_no_page_constant() {
    assert_eq!(NO_PAGE, -1);
}

#[test]
fn test_init_and_shutdown_buffer_pool() {
    let fname = "test_bm_init";
    prepare_file_with_pages(fname, 5);
    let mut bm = empty_pool();
    let rc = init_buffer_pool(&mut bm, fname, 3, ReplacementStrategy::RsFifo, None);
    assert_eq!(rc, RC::Ok);
    assert_eq!(bm.num_pages, 3);
    assert_eq!(bm.page_file, fname);

    // Initial: no pages used yet -> getFrameContents returns NO_PAGE for all
    let frames = get_frame_contents(&bm);
    assert_eq!(frames, vec![NO_PAGE; 3]);
    let dirty = get_dirty_flags(&bm);
    assert_eq!(dirty, vec![false; 3]);
    let fixes = get_fix_counts(&bm);
    assert_eq!(fixes, vec![0; 3]);
    assert_eq!(get_num_read_io(&bm), 0);
    assert_eq!(get_num_write_io(&bm), 0);

    let rc = shutdown_buffer_pool(&mut bm);
    assert_eq!(rc, RC::Ok);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_pin_page_and_stats() {
    let fname = "test_bm_pin";
    prepare_file_with_pages(fname, 5);
    let mut bm = empty_pool();
    assert_eq!(
        init_buffer_pool(&mut bm, fname, 3, ReplacementStrategy::RsFifo, None),
        RC::Ok
    );

    let mut p = make_handle();
    let rc = pin_page(&mut bm, &mut p, 0);
    assert_eq!(rc, RC::Ok);
    assert_eq!(p.page_num, 0);

    let frames = get_frame_contents(&bm);
    assert_eq!(frames[0], 0);
    assert_eq!(frames[1], NO_PAGE);
    assert_eq!(frames[2], NO_PAGE);

    let fixes = get_fix_counts(&bm);
    assert_eq!(fixes[0], 1);
    assert_eq!(fixes[1], 0);
    assert_eq!(fixes[2], 0);

    assert_eq!(get_num_read_io(&bm), 1);
    assert_eq!(get_num_write_io(&bm), 0);

    // Pin same page again -> fix count increments
    let mut p2 = make_handle();
    let rc = pin_page(&mut bm, &mut p2, 0);
    assert_eq!(rc, RC::Ok);
    let fixes = get_fix_counts(&bm);
    assert_eq!(fixes[0], 2);

    // unpin
    let _ = unpin_page(&mut bm, &mut p);
    let fixes = get_fix_counts(&bm);
    assert_eq!(fixes[0], 1);
    let _ = unpin_page(&mut bm, &mut p2);
    let fixes = get_fix_counts(&bm);
    assert_eq!(fixes[0], 0);

    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_mark_dirty_and_force_page() {
    let fname = "test_bm_dirty";
    prepare_file_with_pages(fname, 5);
    let mut bm = empty_pool();
    assert_eq!(
        init_buffer_pool(&mut bm, fname, 3, ReplacementStrategy::RsFifo, None),
        RC::Ok
    );

    let mut p = make_handle();
    let _ = pin_page(&mut bm, &mut p, 0);

    let rc = mark_dirty(&mut bm, &mut p);
    assert_eq!(rc, RC::Ok);
    let dirty = get_dirty_flags(&bm);
    assert_eq!(dirty[0], true);

    let rc = force_page(&mut bm, &mut p);
    assert_eq!(rc, RC::Ok);
    let dirty = get_dirty_flags(&bm);
    assert_eq!(dirty[0], false);
    assert_eq!(get_num_write_io(&bm), 1);

    let _ = unpin_page(&mut bm, &mut p);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_pin_multiple_pages_fifo() {
    let fname = "test_bm_multi";
    prepare_file_with_pages(fname, 10);
    let mut bm = empty_pool();
    assert_eq!(
        init_buffer_pool(&mut bm, fname, 3, ReplacementStrategy::RsFifo, None),
        RC::Ok
    );

    // Pin 3 different pages: 0, 1, 2 (fills the pool).
    let mut p0 = make_handle();
    let mut p1 = make_handle();
    let mut p2 = make_handle();
    assert_eq!(pin_page(&mut bm, &mut p0, 0), RC::Ok);
    assert_eq!(pin_page(&mut bm, &mut p1, 1), RC::Ok);
    assert_eq!(pin_page(&mut bm, &mut p2, 2), RC::Ok);

    let frames = get_frame_contents(&bm);
    assert_eq!(frames, vec![0, 1, 2]);

    let _ = unpin_page(&mut bm, &mut p0);
    let _ = unpin_page(&mut bm, &mut p1);
    let _ = unpin_page(&mut bm, &mut p2);

    // Now the pool is full but all unpinned. Pin page 3 -> should evict
    // page 0 (FIFO).
    let mut p3 = make_handle();
    assert_eq!(pin_page(&mut bm, &mut p3, 3), RC::Ok);
    let frames = get_frame_contents(&bm);
    // FIFO: order was 0,1,2 -> after eviction the new page 3 takes slot 0
    // and updated_order shifts so the new entry is at position 2.
    assert_eq!(frames[0], 3);
    let _ = unpin_page(&mut bm, &mut p3);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_force_flush_pool() {
    let fname = "test_bm_flush";
    prepare_file_with_pages(fname, 5);
    let mut bm = empty_pool();
    assert_eq!(
        init_buffer_pool(&mut bm, fname, 3, ReplacementStrategy::RsFifo, None),
        RC::Ok
    );
    let mut p = make_handle();
    let _ = pin_page(&mut bm, &mut p, 0);
    let _ = mark_dirty(&mut bm, &mut p);
    let _ = unpin_page(&mut bm, &mut p);
    let rc = force_flush_pool(&mut bm);
    assert_eq!(rc, RC::Ok);
    let dirty = get_dirty_flags(&bm);
    assert_eq!(dirty[0], false);
    assert_eq!(get_num_write_io(&bm), 1);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(fname);
}

fn main() {}
