use recordManager::dberror::{PAGE_SIZE, RC};
use recordManager::storage_mgr::{
    append_empty_block, close_page_file, create_page_file, destroy_page_file, ensure_capacity,
    get_block_pos, init_storage_manager, open_page_file, read_block, read_current_block,
    read_first_block, read_last_block, read_next_block, read_previous_block, write_block,
    write_current_block, SM_FileHandle, SM_PageHandle,
};

fn empty_handle() -> SM_FileHandle {
    SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    }
}

fn make_zero_page() -> SM_PageHandle {
    unsafe { String::from_utf8_unchecked(vec![0u8; PAGE_SIZE as usize]) }
}

#[test]
fn test_init_storage_manager() {
    init_storage_manager();
}

#[test]
fn test_create_open_close_destroy() {
    let fname = "test_create_open_close.bin";
    let _ = destroy_page_file(fname);
    assert_eq!(create_page_file(fname), RC::Ok);

    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::Ok);
    // After open, totalNumPages from atoi("\0\0...") = 0
    assert_eq!(fh.total_num_pages, 0);
    assert_eq!(fh.cur_page_pos, 0);
    assert_eq!(fh.file_name, fname);

    assert_eq!(close_page_file(&mut fh), RC::Ok);
    assert_eq!(destroy_page_file(fname), RC::Ok);
}

#[test]
fn test_open_nonexistent() {
    let fname = "nonexistent_test_file.bin";
    let _ = destroy_page_file(fname);
    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::FileNotFound);
}

#[test]
fn test_read_and_write_block() {
    let fname = "test_rw_block.bin";
    let _ = destroy_page_file(fname);
    assert_eq!(create_page_file(fname), RC::Ok);
    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::Ok);

    // ensure 1 page exists then write to it
    let _ = ensure_capacity(1, &mut fh);
    assert_eq!(fh.total_num_pages, 1);

    let mut buf = make_zero_page();
    unsafe {
        let v = buf.as_mut_vec();
        v[..12].copy_from_slice(b"Hello, World");
    }
    let rc = write_block(0, &mut fh, &buf);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.cur_page_pos, 0);

    // Read it back
    let mut rbuf = make_zero_page();
    let rc = read_block(0, &mut fh, &mut rbuf);
    assert_eq!(rc, RC::Ok);
    let bytes = rbuf.as_bytes();
    assert_eq!(&bytes[..12], b"Hello, World");

    assert_eq!(get_block_pos(&fh), 0);

    let _ = close_page_file(&mut fh);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_append_empty_block() {
    let fname = "test_append.bin";
    let _ = destroy_page_file(fname);
    assert_eq!(create_page_file(fname), RC::Ok);
    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::Ok);

    assert_eq!(fh.total_num_pages, 0);
    let rc = append_empty_block(&mut fh);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.total_num_pages, 1);
    assert_eq!(fh.cur_page_pos, 0);

    let rc = append_empty_block(&mut fh);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.total_num_pages, 2);
    assert_eq!(fh.cur_page_pos, 1);

    let _ = close_page_file(&mut fh);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_ensure_capacity() {
    let fname = "test_ensure.bin";
    let _ = destroy_page_file(fname);
    assert_eq!(create_page_file(fname), RC::Ok);
    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::Ok);

    // initial total = 0, ensure 5 -> append 5 blocks
    let rc = ensure_capacity(5, &mut fh);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.total_num_pages, 5);
    // cur position is total_num_pages-1 after the last append
    assert_eq!(fh.cur_page_pos, 4);

    // ensure capacity for fewer pages -> no-op
    let rc = ensure_capacity(3, &mut fh);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.total_num_pages, 5);
    assert_eq!(fh.cur_page_pos, 4);

    let _ = close_page_file(&mut fh);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_read_navigation() {
    let fname = "test_nav.bin";
    let _ = destroy_page_file(fname);
    assert_eq!(create_page_file(fname), RC::Ok);
    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::Ok);
    let _ = ensure_capacity(5, &mut fh);
    assert_eq!(fh.total_num_pages, 5);

    let mut buf = make_zero_page();
    let rc = read_first_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.cur_page_pos, 0);

    let rc = read_last_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.cur_page_pos, 4);

    // read_next at last -> non-existing
    let rc = read_next_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::ReadNonExistingPage);

    let rc = read_previous_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.cur_page_pos, 3);

    let rc = read_current_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.cur_page_pos, 3);

    // read out-of-range
    let rc = read_block(100, &mut fh, &mut buf);
    assert_eq!(rc, RC::ReadNonExistingPage);

    let _ = close_page_file(&mut fh);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_write_current_block() {
    let fname = "test_wcurrent.bin";
    let _ = destroy_page_file(fname);
    assert_eq!(create_page_file(fname), RC::Ok);
    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::Ok);
    let _ = ensure_capacity(2, &mut fh);
    assert_eq!(fh.total_num_pages, 2);
    // After ensure_capacity, cur_page_pos is 1 (last page)
    let mut buf = make_zero_page();
    unsafe {
        let v = buf.as_mut_vec();
        v[..4].copy_from_slice(b"data");
    }
    let rc = write_current_block(&mut fh, &buf);
    assert_eq!(rc, RC::Ok);

    // Reading it back
    let mut rbuf = make_zero_page();
    let rc = read_block(1, &mut fh, &mut rbuf);
    assert_eq!(rc, RC::Ok);
    assert_eq!(&rbuf.as_bytes()[..4], b"data");

    let _ = close_page_file(&mut fh);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_read_previous_at_first() {
    let fname = "test_rprev.bin";
    let _ = destroy_page_file(fname);
    assert_eq!(create_page_file(fname), RC::Ok);
    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::Ok);
    let _ = ensure_capacity(2, &mut fh);
    let mut buf = make_zero_page();
    let _ = read_first_block(&mut fh, &mut buf);
    assert_eq!(fh.cur_page_pos, 0);
    let rc = read_previous_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::ReadNonExistingPage);

    let _ = close_page_file(&mut fh);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_write_negative_page() {
    let fname = "test_wneg.bin";
    let _ = destroy_page_file(fname);
    assert_eq!(create_page_file(fname), RC::Ok);
    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::Ok);
    let buf = make_zero_page();
    let rc = write_block(-1, &mut fh, &buf);
    assert_eq!(rc, RC::WriteFailed);
    let _ = close_page_file(&mut fh);
    let _ = destroy_page_file(fname);
}

#[test]
fn test_read_last_empty() {
    let fname = "test_rl_empty.bin";
    let _ = destroy_page_file(fname);
    assert_eq!(create_page_file(fname), RC::Ok);
    let mut fh = empty_handle();
    assert_eq!(open_page_file(fname, &mut fh), RC::Ok);
    // total_num_pages = 0, so last block = -1 -> non-existing
    let mut buf = make_zero_page();
    let rc = read_last_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::ReadNonExistingPage);
    let _ = close_page_file(&mut fh);
    let _ = destroy_page_file(fname);
}

fn main() {}
