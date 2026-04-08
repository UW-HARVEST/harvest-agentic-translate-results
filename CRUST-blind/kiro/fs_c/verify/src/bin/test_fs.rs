use fs_c::fs;

const ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyz\n";

fn tmp_path(name: &str) -> String {
    format!("tmp/{}", name)
}

// --- fs_open / fs_close ---

#[test]
fn test_fs_open_existing_file() {
    let fd = fs::fs_open(&tmp_path("file"), fs::FS_OPEN_READ);
    assert!(fd.is_some());
    fs::fs_close(fd.unwrap()).unwrap();
}

#[test]
fn test_fs_open_nonexistent_returns_none() {
    let fd = fs::fs_open("/root/foo", fs::FS_OPEN_WRITE);
    assert!(fd.is_none());
}

#[test]
fn test_fs_close_returns_ok() {
    let fd = fs::fs_open(&tmp_path("file"), fs::FS_OPEN_READ).unwrap();
    assert!(fs::fs_close(fd).is_ok());
}

// --- fs_exists ---

#[test]
fn test_fs_exists_existing() {
    assert!(fs::fs_exists(&tmp_path("file")));
}

#[test]
fn test_fs_exists_nonexistent() {
    assert!(!fs::fs_exists("tmp/a file that doesn't exist"));
}

// --- fs_stat ---

#[test]
fn test_fs_stat_existing() {
    let meta = fs::fs_stat(&tmp_path("file"));
    assert!(meta.is_ok());
}

#[test]
fn test_fs_stat_nonexistent() {
    let meta = fs::fs_stat("tmp/a file that doesn't exist");
    assert!(meta.is_err());
}

// --- fs_fstat ---

#[test]
fn test_fs_fstat_valid_fd() {
    let fd = fs::fs_open(&tmp_path("file"), fs::FS_OPEN_READ).unwrap();
    let meta = fs::fs_fstat(&fd);
    assert!(meta.is_ok());
    fs::fs_close(fd).unwrap();
}

// --- fs_lstat ---

#[test]
fn test_fs_lstat_existing() {
    let meta = fs::fs_lstat(&tmp_path("file"));
    assert!(meta.is_ok());
}

// --- fs_size ---

#[test]
fn test_fs_size_known_file() {
    // tmp/file contains alpha string = 27 bytes
    let sz = fs::fs_size(&tmp_path("file")).unwrap();
    assert_eq!(sz, 27);
}

// --- fs_fsize ---

#[test]
fn test_fs_fsize_known_file() {
    let fd = fs::fs_open(&tmp_path("file"), fs::FS_OPEN_READ).unwrap();
    let sz = fs::fs_fsize(&fd).unwrap();
    assert_eq!(sz, 27);
    fs::fs_close(fd).unwrap();
}

// --- fs_write ---

#[test]
fn test_fs_write_returns_bytes_written() {
    let n = fs::fs_write(&tmp_path("test_write"), ALPHA).unwrap();
    assert_eq!(n, 27);
    std::fs::remove_file(tmp_path("test_write")).ok();
}

#[test]
fn test_fs_write_content_matches() {
    fs::fs_write(&tmp_path("test_write2"), ALPHA).unwrap();
    let buf = fs::fs_read(&tmp_path("test_write2")).unwrap();
    assert_eq!(buf, ALPHA);
    std::fs::remove_file(tmp_path("test_write2")).ok();
}

// --- fs_nwrite ---

#[test]
fn test_fs_nwrite_partial() {
    let n = fs::fs_nwrite(&tmp_path("test_nwrite"), ALPHA, 9).unwrap();
    assert_eq!(n, 9);
    let buf = fs::fs_read(&tmp_path("test_nwrite")).unwrap();
    assert_eq!(buf, b"abcdefghi");
    std::fs::remove_file(tmp_path("test_nwrite")).ok();
}

// --- fs_fwrite ---

#[test]
fn test_fs_fwrite_content() {
    let fd = fs::fs_open(&tmp_path("test_fwrite"), fs::FS_OPEN_WRITE).unwrap();
    let n = fs::fs_fwrite(&fd, ALPHA).unwrap();
    assert_eq!(n, 27);
    fs::fs_close(fd).unwrap();

    let buf = fs::fs_read(&tmp_path("test_fwrite")).unwrap();
    assert_eq!(buf, ALPHA);
    std::fs::remove_file(tmp_path("test_fwrite")).ok();
}

// --- fs_fnwrite ---

#[test]
fn test_fs_fnwrite_partial() {
    let fd = fs::fs_open(&tmp_path("test_fnwrite"), fs::FS_OPEN_WRITE).unwrap();
    let n = fs::fs_fnwrite(&fd, ALPHA, 9).unwrap();
    assert_eq!(n, 9);
    fs::fs_close(fd).unwrap();

    let buf = fs::fs_read(&tmp_path("test_fnwrite")).unwrap();
    assert_eq!(buf, b"abcdefghi");
    std::fs::remove_file(tmp_path("test_fnwrite")).ok();
}

// --- fs_read ---

#[test]
fn test_fs_read_existing() {
    let buf = fs::fs_read(&tmp_path("file")).unwrap();
    assert_eq!(buf, ALPHA);
}

#[test]
fn test_fs_read_nonexistent() {
    let result = fs::fs_read("tmp/nonexistent");
    assert!(result.is_err());
}

// --- fs_nread ---

#[test]
fn test_fs_nread_partial() {
    fs::fs_write(&tmp_path("test_nread"), ALPHA).unwrap();
    let buf = fs::fs_nread(&tmp_path("test_nread"), 9).unwrap();
    assert_eq!(buf, b"abcdefghi");
    std::fs::remove_file(tmp_path("test_nread")).ok();
}

// --- fs_fread ---

#[test]
fn test_fs_fread_content() {
    fs::fs_write(&tmp_path("test_fread"), ALPHA).unwrap();
    let fd = fs::fs_open(&tmp_path("test_fread"), fs::FS_OPEN_READ).unwrap();
    let buf = fs::fs_fread(&fd).unwrap();
    assert_eq!(buf, ALPHA);
    fs::fs_close(fd).unwrap();
    std::fs::remove_file(tmp_path("test_fread")).ok();
}

// --- fs_fnread ---

#[test]
fn test_fs_fnread_partial() {
    fs::fs_write(&tmp_path("test_fnread"), ALPHA).unwrap();
    let fd = fs::fs_open(&tmp_path("test_fnread"), fs::FS_OPEN_READ).unwrap();
    let buf = fs::fs_fnread(&fd, 9).unwrap();
    assert_eq!(buf, b"abcdefghi");
    fs::fs_close(fd).unwrap();
    std::fs::remove_file(tmp_path("test_fnread")).ok();
}

// --- fs_truncate ---

#[test]
fn test_fs_truncate() {
    fs::fs_write(&tmp_path("test_trunc"), ALPHA).unwrap();
    fs::fs_truncate(&tmp_path("test_trunc"), 9).unwrap();
    let buf = fs::fs_read(&tmp_path("test_trunc")).unwrap();
    assert_eq!(buf, b"abcdefghi");
    std::fs::remove_file(tmp_path("test_trunc")).ok();
}

// --- fs_ftruncate ---

#[test]
fn test_fs_ftruncate() {
    fs::fs_write(&tmp_path("test_ftrunc"), ALPHA).unwrap();
    let fd = fs::fs_open(&tmp_path("test_ftrunc"), fs::FS_OPEN_READWRITE).unwrap();
    fs::fs_ftruncate(&fd, 9).unwrap();
    fs::fs_close(fd).unwrap();
    let buf = fs::fs_read(&tmp_path("test_ftrunc")).unwrap();
    assert_eq!(buf, b"abcdefghi");
    std::fs::remove_file(tmp_path("test_ftrunc")).ok();
}

// --- fs_rename ---

#[test]
fn test_fs_rename() {
    fs::fs_write(&tmp_path("test_rename_src"), ALPHA).unwrap();
    fs::fs_rename(&tmp_path("test_rename_src"), &tmp_path("test_rename_dst")).unwrap();
    assert!(!fs::fs_exists(&tmp_path("test_rename_src")));
    let buf = fs::fs_read(&tmp_path("test_rename_dst")).unwrap();
    assert_eq!(buf, ALPHA);
    std::fs::remove_file(tmp_path("test_rename_dst")).ok();
}

// --- fs_mkdir / fs_rmdir ---

#[test]
fn test_fs_mkdir_and_rmdir() {
    let _ = fs::fs_rmdir(&tmp_path("test_dir"));
    fs::fs_mkdir(&tmp_path("test_dir"), 0o777).unwrap();
    assert!(fs::fs_exists(&tmp_path("test_dir")));
    fs::fs_rmdir(&tmp_path("test_dir")).unwrap();
    assert!(!fs::fs_exists(&tmp_path("test_dir")));
}

// --- fs_error (just ensure it doesn't panic) ---

#[test]
fn test_fs_error_no_panic() {
    fs::fs_error("test");
}

// --- constants ---

#[test]
fn test_open_mode_constants() {
    assert_eq!(fs::FS_OPEN_READ, "r");
    assert_eq!(fs::FS_OPEN_WRITE, "w");
    assert_eq!(fs::FS_OPEN_READWRITE, "rw");
}

fn main() {}
