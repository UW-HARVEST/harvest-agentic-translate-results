use recordManager::dberror::{RC, PAGE_SIZE};
use recordManager::buffer_mgr::{
    BM_BufferPool, BM_PageHandle, ReplacementStrategy,
    init_buffer_pool, shutdown_buffer_pool, pin_page, unpin_page, mark_dirty,
    force_page, force_flush_pool,
    get_frame_contents, get_dirty_flags, get_fix_counts, get_num_read_io, get_num_write_io,
    NO_PAGE,
};
use recordManager::storage_mgr::{create_page_file, destroy_page_file, ensure_capacity, open_page_file, close_page_file, SM_FileHandle};

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

fn empty_handle() -> BM_PageHandle {
    BM_PageHandle { page_num: -1, data: String::new() }
}

fn empty_pool() -> BM_BufferPool {
    BM_BufferPool {
        page_file: String::new(),
        num_pages: 0,
        strategy: ReplacementStrategy::RsFifo,
        mgmt_data: None,
    }
}

#[test]
fn test_init_and_shutdown() {
    let path = "/tmp/bm_test_init.bin";
    make_test_file(path, 1);
    let mut bm = empty_pool();
    let rc = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    assert!(rc == RC::Ok);
    assert_eq!(bm.num_pages, 3);
    let rc = shutdown_buffer_pool(&mut bm);
    assert!(rc == RC::Ok);
    let _ = destroy_page_file(path);
}

#[test]
fn test_pin_page_and_unpin() {
    let path = "/tmp/bm_test_pin.bin";
    make_test_file(path, 3);
    let mut bm = empty_pool();
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    let mut p = empty_handle();
    let rc = pin_page(&mut bm, &mut p, 0);
    assert!(rc == RC::Ok);
    assert_eq!(p.page_num, 0);
    assert_eq!(p.data.chars().count(), PAGE_SIZE as usize);
    let frames = get_frame_contents(&bm);
    assert_eq!(frames[0], 0);
    assert_eq!(frames[1], NO_PAGE);
    assert_eq!(frames[2], NO_PAGE);
    let fix = get_fix_counts(&bm);
    assert_eq!(fix[0], 1);
    let rc = unpin_page(&mut bm, &mut p);
    assert!(rc == RC::Ok);
    let fix = get_fix_counts(&bm);
    assert_eq!(fix[0], 0);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

#[test]
fn test_get_num_read_write_io() {
    let path = "/tmp/bm_test_io_counts.bin";
    make_test_file(path, 3);
    let mut bm = empty_pool();
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    assert_eq!(get_num_read_io(&bm), 0);
    assert_eq!(get_num_write_io(&bm), 0);
    let mut p = empty_handle();
    let _ = pin_page(&mut bm, &mut p, 0);
    assert_eq!(get_num_read_io(&bm), 1);
    let _ = pin_page(&mut bm, &mut p, 1);
    assert_eq!(get_num_read_io(&bm), 2);
    let _ = unpin_page(&mut bm, &mut p);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

#[test]
fn test_mark_dirty_and_dirty_flags() {
    let path = "/tmp/bm_test_dirty.bin";
    make_test_file(path, 3);
    let mut bm = empty_pool();
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    let mut p = empty_handle();
    let _ = pin_page(&mut bm, &mut p, 0);
    let dirty = get_dirty_flags(&bm);
    assert_eq!(dirty[0], false);
    let _ = mark_dirty(&mut bm, &mut p);
    let dirty = get_dirty_flags(&bm);
    assert_eq!(dirty[0], true);
    let _ = unpin_page(&mut bm, &mut p);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

#[test]
fn test_force_page_writes() {
    let path = "/tmp/bm_test_force.bin";
    make_test_file(path, 2);
    let mut bm = empty_pool();
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    let mut p = empty_handle();
    let _ = pin_page(&mut bm, &mut p, 0);
    let _ = mark_dirty(&mut bm, &mut p);
    let initial_writes = get_num_write_io(&bm);
    let rc = force_page(&mut bm, &mut p);
    assert!(rc == RC::Ok);
    assert_eq!(get_num_write_io(&bm), initial_writes + 1);
    let dirty = get_dirty_flags(&bm);
    assert_eq!(dirty[0], false);
    let _ = unpin_page(&mut bm, &mut p);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

#[test]
fn test_force_flush_pool() {
    let path = "/tmp/bm_test_forceflush.bin";
    make_test_file(path, 2);
    let mut bm = empty_pool();
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    let mut p = empty_handle();
    let _ = pin_page(&mut bm, &mut p, 0);
    let _ = mark_dirty(&mut bm, &mut p);
    let _ = unpin_page(&mut bm, &mut p);
    let initial_writes = get_num_write_io(&bm);
    let rc = force_flush_pool(&mut bm);
    assert!(rc == RC::Ok);
    assert_eq!(get_num_write_io(&bm), initial_writes + 1);
    let dirty = get_dirty_flags(&bm);
    assert_eq!(dirty[0], false);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

#[test]
fn test_no_page_constant() {
    assert_eq!(NO_PAGE, -1);
}

#[test]
fn test_pin_persists_after_shutdown() {
    let path = "/tmp/bm_test_persist.bin";
    make_test_file(path, 2);
    let mut bm = empty_pool();
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    let mut p = empty_handle();
    let _ = pin_page(&mut bm, &mut p, 0);
    // Modify the page data
    let mut chars: Vec<char> = p.data.chars().collect();
    chars[0] = 'H' as char;
    chars[1] = 'i' as char;
    p.data = chars.into_iter().collect();
    let _ = mark_dirty(&mut bm, &mut p);
    let _ = unpin_page(&mut bm, &mut p);
    let _ = shutdown_buffer_pool(&mut bm);

    // Reopen and read back
    let mut bm = empty_pool();
    let _ = init_buffer_pool(&mut bm, path, 3, ReplacementStrategy::RsFifo, None);
    let mut p = empty_handle();
    let _ = pin_page(&mut bm, &mut p, 0);
    let chars: Vec<char> = p.data.chars().collect();
    assert_eq!(chars[0], 'H');
    assert_eq!(chars[1], 'i');
    let _ = unpin_page(&mut bm, &mut p);
    let _ = shutdown_buffer_pool(&mut bm);
    let _ = destroy_page_file(path);
}

fn main() {}
