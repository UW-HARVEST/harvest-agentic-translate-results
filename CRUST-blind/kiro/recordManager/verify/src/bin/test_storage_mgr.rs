use recordManager::storage_mgr::*;
use recordManager::dberror::RC;
use std::fs;

fn unique_file(name: &str) -> String {
    format!("/tmp/test_sm_{}", name)
}

#[test]
fn test_create_and_destroy_page_file() {
    let f = unique_file("create_destroy");
    let _ = fs::remove_file(&f);
    let rc = create_page_file(&f);
    assert_eq!(rc, RC::Ok);
    assert!(fs::metadata(&f).is_ok());
    let rc = destroy_page_file(&f);
    assert_eq!(rc, RC::Ok);
    assert!(fs::metadata(&f).is_err());
}

#[test]
fn test_open_and_close_page_file() {
    let f = unique_file("open_close");
    let _ = fs::remove_file(&f);
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    };
    let rc = open_page_file(&f, &mut fh);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.file_name, f);
    let rc = close_page_file(&mut fh);
    assert_eq!(rc, RC::Ok);
    destroy_page_file(&f);
}

#[test]
fn test_open_nonexistent_file() {
    let mut fh = SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    };
    let rc = open_page_file("/tmp/nonexistent_sm_test_file_xyz", &mut fh);
    assert_eq!(rc, RC::FileNotFound);
}

#[test]
fn test_get_block_pos() {
    let f = unique_file("block_pos");
    let _ = fs::remove_file(&f);
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(),
        total_num_pages: 0,
        cur_page_pos: 0,
        mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    assert_eq!(get_block_pos(&fh), 0);
    close_page_file(&mut fh);
    destroy_page_file(&f);
}

#[test]
fn test_init_storage_manager() {
    // Should not panic
    init_storage_manager();
}

fn main() {}
