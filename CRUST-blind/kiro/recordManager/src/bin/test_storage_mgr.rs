use recordManager::dberror::RC;
use recordManager::storage_mgr::*;
use std::fs;

fn unique_file(name: &str) -> String {
    format!("/tmp/test_smgr_{}_{}", name, std::process::id())
}

#[test]
fn test_init_storage_manager() {
    init_storage_manager(); // should not panic
}

#[test]
fn test_create_page_file() {
    let f = unique_file("create");
    let rc = create_page_file(&f);
    assert_eq!(rc, RC::Ok);
    let meta = fs::metadata(&f).unwrap();
    assert_eq!(meta.len(), 4096);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_open_close_page_file() {
    let f = unique_file("openclose");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    let rc = open_page_file(&f, &mut fh);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.cur_page_pos, 0);

    let rc = close_page_file(&mut fh);
    assert_eq!(rc, RC::Ok);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_destroy_page_file() {
    let f = unique_file("destroy");
    create_page_file(&f);
    assert!(fs::metadata(&f).is_ok());
    let rc = destroy_page_file(&f);
    assert_eq!(rc, RC::Ok);
    assert!(fs::metadata(&f).is_err());
}

#[test]
fn test_destroy_nonexistent() {
    let f = unique_file("noexist");
    let rc = destroy_page_file(&f);
    assert_eq!(rc, RC::DestroyFailed);
}

#[test]
fn test_open_nonexistent() {
    let f = unique_file("nofile");
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    let rc = open_page_file(&f, &mut fh);
    assert_eq!(rc, RC::FileNotFound);
}

#[test]
fn test_write_read_block() {
    let f = unique_file("writeread");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);

    // Append a block so we have page 0 to write to
    let rc = append_empty_block(&mut fh);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.total_num_pages, 1);

    // Write data to page 0
    let mut data = String::from("Hello, page!");
    data.push_str(&"\0".repeat(4096 - data.len()));
    let rc = write_block(0, &mut fh, &data);
    assert_eq!(rc, RC::Ok);

    // Read it back
    let mut read_buf = String::new();
    let rc = read_block(0, &mut fh, &mut read_buf);
    assert_eq!(rc, RC::Ok);
    assert!(read_buf.starts_with("Hello, page!"));

    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_read_non_existing_page() {
    let f = unique_file("readnon");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    let mut buf = String::new();
    let rc = read_block(99, &mut fh, &mut buf);
    assert_eq!(rc, RC::ReadNonExistingPage);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_read_negative_page() {
    let f = unique_file("readneg");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    let mut buf = String::new();
    let rc = read_block(-1, &mut fh, &mut buf);
    assert_eq!(rc, RC::ReadNonExistingPage);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_append_empty_block() {
    let f = unique_file("append");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    let initial = fh.total_num_pages;
    let rc = append_empty_block(&mut fh);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.total_num_pages, initial + 1);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_ensure_capacity() {
    let f = unique_file("capacity");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    let rc = ensure_capacity(3, &mut fh);
    assert_eq!(rc, RC::Ok);
    assert!(fh.total_num_pages >= 3);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_ensure_capacity_already_met() {
    let f = unique_file("capmet");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    ensure_capacity(3, &mut fh);
    let pages = fh.total_num_pages;
    let rc = ensure_capacity(2, &mut fh);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.total_num_pages, pages);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_get_block_pos() {
    let f = unique_file("blockpos");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    assert_eq!(get_block_pos(&fh), 0);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_write_block_negative_page() {
    let f = unique_file("writeneg");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    let data = "\0".repeat(4096);
    let rc = write_block(-1, &mut fh, &data);
    assert_eq!(rc, RC::WriteFailed);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_read_first_block() {
    let f = unique_file("readfirst");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    append_empty_block(&mut fh);
    let mut buf = String::new();
    let rc = read_first_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.cur_page_pos, 0);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_read_last_block() {
    let f = unique_file("readlast");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    append_empty_block(&mut fh);
    append_empty_block(&mut fh);
    let mut buf = String::new();
    let rc = read_last_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::Ok);
    assert_eq!(fh.cur_page_pos, fh.total_num_pages - 1);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

#[test]
fn test_read_last_block_no_pages() {
    let f = unique_file("readlastnone");
    create_page_file(&f);
    let mut fh = SM_FileHandle {
        file_name: String::new(), total_num_pages: 0,
        cur_page_pos: 0, mgmt_info: None,
    };
    open_page_file(&f, &mut fh);
    // total_num_pages is 0, so last page = -1
    let mut buf = String::new();
    let rc = read_last_block(&mut fh, &mut buf);
    assert_eq!(rc, RC::ReadNonExistingPage);
    close_page_file(&mut fh);
    let _ = fs::remove_file(&f);
}

fn main() {}
