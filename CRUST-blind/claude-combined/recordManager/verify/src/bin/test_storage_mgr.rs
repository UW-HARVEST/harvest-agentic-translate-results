use recordManager::dberror::{RC, PAGE_SIZE};
use recordManager::storage_mgr::{
    SM_FileHandle, init_storage_manager, create_page_file, open_page_file,
    close_page_file, destroy_page_file, read_block, get_block_pos, read_first_block,
    read_last_block, read_next_block, read_previous_block, read_current_block,
    write_block, write_current_block, append_empty_block, ensure_capacity,
};

fn new_handle() -> SM_FileHandle {
    SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    }
}

#[test]
fn test_init_storage_manager() {
    init_storage_manager();
}

#[test]
fn test_create_open_close_destroy() {
    let path = "/tmp/sm_test_basic.bin";
    let _ = std::fs::remove_file(path);
    let rc = create_page_file(path);
    assert!(rc == RC::Ok);
    let mut h = new_handle();
    let rc = open_page_file(path, &mut h);
    assert!(rc == RC::Ok);
    assert_eq!(h.total_num_pages, 0);
    assert_eq!(h.cur_page_pos, 0);
    assert_eq!(h.file_name, path);
    let rc = close_page_file(&mut h);
    assert!(rc == RC::Ok);
    let rc = destroy_page_file(path);
    assert!(rc == RC::Ok);
}

#[test]
fn test_read_block_non_existing() {
    let path = "/tmp/sm_test_read.bin";
    let _ = std::fs::remove_file(path);
    let _ = create_page_file(path);
    let mut h = new_handle();
    let _ = open_page_file(path, &mut h);
    let mut page = String::new();
    let rc = read_block(0, &mut h, &mut page);
    assert!(rc == RC::ReadNonExistingPage);
    let _ = close_page_file(&mut h);
    let _ = destroy_page_file(path);
}

#[test]
fn test_append_and_read() {
    let path = "/tmp/sm_test_append.bin";
    let _ = std::fs::remove_file(path);
    let _ = create_page_file(path);
    let mut h = new_handle();
    let _ = open_page_file(path, &mut h);
    assert_eq!(h.total_num_pages, 0);
    let rc = append_empty_block(&mut h);
    assert!(rc == RC::Ok);
    assert_eq!(h.total_num_pages, 1);
    assert_eq!(h.cur_page_pos, 0);
    let mut page = String::new();
    let rc = read_block(0, &mut h, &mut page);
    assert!(rc == RC::Ok);
    assert_eq!(page.len(), PAGE_SIZE as usize);
    // All bytes should be zero
    for c in page.chars() {
        assert_eq!(c as u8, 0);
    }
    let _ = close_page_file(&mut h);
    let _ = destroy_page_file(path);
}

#[test]
fn test_ensure_capacity() {
    let path = "/tmp/sm_test_capacity.bin";
    let _ = std::fs::remove_file(path);
    let _ = create_page_file(path);
    let mut h = new_handle();
    let _ = open_page_file(path, &mut h);
    let rc = ensure_capacity(5, &mut h);
    assert!(rc == RC::Ok);
    assert_eq!(h.total_num_pages, 5);
    let rc = ensure_capacity(3, &mut h);
    assert!(rc == RC::Ok);
    assert_eq!(h.total_num_pages, 5);
    let _ = close_page_file(&mut h);
    let _ = destroy_page_file(path);
}

#[test]
fn test_get_block_pos() {
    let path = "/tmp/sm_test_blockpos.bin";
    let _ = std::fs::remove_file(path);
    let _ = create_page_file(path);
    let mut h = new_handle();
    let _ = open_page_file(path, &mut h);
    let _ = ensure_capacity(3, &mut h);
    h.cur_page_pos = 2;
    assert_eq!(get_block_pos(&h), 2);
    let _ = close_page_file(&mut h);
    let _ = destroy_page_file(path);
}

#[test]
fn test_write_then_read_block() {
    let path = "/tmp/sm_test_write.bin";
    let _ = std::fs::remove_file(path);
    let _ = create_page_file(path);
    let mut h = new_handle();
    let _ = open_page_file(path, &mut h);
    let _ = ensure_capacity(2, &mut h);

    // Build payload with PAGE_SIZE chars in latin-1 range
    let payload: String = (0..PAGE_SIZE as usize).map(|i| (i as u8) as char).collect();
    assert_eq!(payload.chars().count(), PAGE_SIZE as usize);
    let rc = write_block(0, &mut h, &payload);
    assert!(rc == RC::Ok);
    assert_eq!(h.cur_page_pos, 0);

    let mut readback = String::new();
    let rc = read_block(0, &mut h, &mut readback);
    assert!(rc == RC::Ok);
    let p_bytes: Vec<u8> = payload.chars().map(|c| c as u8).collect();
    let r_bytes: Vec<u8> = readback.chars().map(|c| c as u8).collect();
    assert_eq!(p_bytes, r_bytes);

    let _ = close_page_file(&mut h);
    let _ = destroy_page_file(path);
}

#[test]
fn test_read_first_last_next_prev_current() {
    let path = "/tmp/sm_test_navigation.bin";
    let _ = std::fs::remove_file(path);
    let _ = create_page_file(path);
    let mut h = new_handle();
    let _ = open_page_file(path, &mut h);
    let _ = ensure_capacity(3, &mut h);

    let mut p = String::new();
    let rc = read_first_block(&mut h, &mut p);
    assert!(rc == RC::Ok);
    assert_eq!(h.cur_page_pos, 0);

    let rc = read_next_block(&mut h, &mut p);
    assert!(rc == RC::Ok);
    assert_eq!(h.cur_page_pos, 1);

    let rc = read_current_block(&mut h, &mut p);
    assert!(rc == RC::Ok);
    assert_eq!(h.cur_page_pos, 1);

    let rc = read_previous_block(&mut h, &mut p);
    assert!(rc == RC::Ok);
    assert_eq!(h.cur_page_pos, 0);

    let rc = read_last_block(&mut h, &mut p);
    assert!(rc == RC::Ok);
    assert_eq!(h.cur_page_pos, 2);

    let _ = close_page_file(&mut h);
    let _ = destroy_page_file(path);
}

#[test]
fn test_write_current_block() {
    let path = "/tmp/sm_test_writecur.bin";
    let _ = std::fs::remove_file(path);
    let _ = create_page_file(path);
    let mut h = new_handle();
    let _ = open_page_file(path, &mut h);
    let _ = ensure_capacity(2, &mut h);
    h.cur_page_pos = 1;
    let payload: String = (0..PAGE_SIZE as usize).map(|i| (i as u8 ^ 0x55) as char).collect();
    assert_eq!(payload.chars().count(), PAGE_SIZE as usize);
    let rc = write_current_block(&mut h, &payload);
    assert!(rc == RC::Ok);
    let mut readback = String::new();
    let _ = read_block(1, &mut h, &mut readback);
    let p_bytes: Vec<u8> = payload.chars().map(|c| c as u8).collect();
    let r_bytes: Vec<u8> = readback.chars().map(|c| c as u8).collect();
    assert_eq!(p_bytes, r_bytes);
    let _ = close_page_file(&mut h);
    let _ = destroy_page_file(path);
}

fn main() {}
