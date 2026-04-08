use fs_c::fs;
use std::os::unix::fs::MetadataExt;

#[test]
fn test_fs_exists() {
    assert!(fs::fs_exists("src/tmp/file"));
    assert!(!fs::fs_exists("src/tmp/nonexistent"));
}

#[test]
fn test_fs_open_valid() {
    let fd = fs::fs_open("src/tmp/file", fs::FS_OPEN_READ);
    assert!(fd.is_some());
}

#[test]
fn test_fs_open_invalid() {
    let fd = fs::fs_open("/root/foo", fs::FS_OPEN_WRITE);
    assert!(fd.is_none());
}

#[test]
fn test_fs_close() {
    let fd = fs::fs_open("src/tmp/file", fs::FS_OPEN_READ).unwrap();
    assert!(fs::fs_close(fd).is_ok());
}

#[test]
fn test_fs_size() {
    let size = fs::fs_size("src/tmp/file").unwrap();
    assert_eq!(size, 27);
}

#[test]
fn test_fs_stat_valid() {
    let meta = fs::fs_stat("src/tmp/file");
    assert!(meta.is_ok());
    let meta = meta.unwrap();
    assert_eq!(meta.size(), 27);
}

#[test]
fn test_fs_stat_invalid() {
    let meta = fs::fs_stat("src/tmp/nonexistent");
    assert!(meta.is_err());
}

#[test]
fn test_fs_fstat() {
    let fd = fs::fs_open("src/tmp/file", fs::FS_OPEN_READ).unwrap();
    let meta = fs::fs_fstat(&fd);
    assert!(meta.is_ok());
    assert_eq!(meta.unwrap().size(), 27);
}

#[test]
fn test_fs_lstat() {
    let meta = fs::fs_lstat("src/tmp/file.link");
    assert!(meta.is_ok());
    let meta = meta.unwrap();
    // lstat on a symlink returns the link's metadata, not the target's
    assert!(meta.file_type().is_symlink());
}

#[test]
fn test_fs_write_and_read() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let path = "src/tmp/test_write_read";
    let n = fs::fs_write(path, alpha).unwrap();
    assert_eq!(n, 27);
    let buf = fs::fs_read(path).unwrap();
    assert_eq!(buf, alpha);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_fs_nwrite_and_read() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let path = "src/tmp/test_nwrite";
    let n = fs::fs_nwrite(path, alpha, 9).unwrap();
    assert_eq!(n, 9);
    let buf = fs::fs_read(path).unwrap();
    assert_eq!(buf, b"abcdefghi");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_fs_nread() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let path = "src/tmp/test_nread";
    fs::fs_write(path, alpha).unwrap();
    let buf = fs::fs_nread(path, 9).unwrap();
    assert_eq!(buf, b"abcdefghi");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_fs_fread() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let path = "src/tmp/test_fread";
    fs::fs_write(path, alpha).unwrap();
    let fd = fs::fs_open(path, fs::FS_OPEN_READ).unwrap();
    let buf = fs::fs_fread(&fd).unwrap();
    assert_eq!(buf, alpha);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_fs_fnread() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let path = "src/tmp/test_fnread";
    fs::fs_write(path, alpha).unwrap();
    let fd = fs::fs_open(path, fs::FS_OPEN_READ).unwrap();
    let buf = fs::fs_fnread(&fd, 5).unwrap();
    assert_eq!(buf, b"abcde");
}

#[test]
fn test_fs_fwrite() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let path = "src/tmp/test_fwrite";
    let fd = fs::fs_open(path, fs::FS_OPEN_WRITE).unwrap();
    let n = fs::fs_fwrite(&fd, alpha).unwrap();
    assert_eq!(n, 27);
    drop(fd);
    let buf = fs::fs_read(path).unwrap();
    assert_eq!(buf, alpha);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_fs_fnwrite() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let path = "src/tmp/test_fnwrite";
    let fd = fs::fs_open(path, fs::FS_OPEN_WRITE).unwrap();
    let n = fs::fs_fnwrite(&fd, alpha, 5).unwrap();
    assert_eq!(n, 5);
    drop(fd);
    let buf = fs::fs_read(path).unwrap();
    assert_eq!(buf, b"abcde");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_fs_truncate() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let path = "src/tmp/test_truncate";
    fs::fs_write(path, alpha).unwrap();
    fs::fs_truncate(path, 9).unwrap();
    let buf = fs::fs_read(path).unwrap();
    assert_eq!(buf, b"abcdefghi");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_fs_ftruncate() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let path = "src/tmp/test_ftruncate";
    fs::fs_write(path, alpha).unwrap();
    let fd = fs::fs_open(path, fs::FS_OPEN_READWRITE).unwrap();
    fs::fs_ftruncate(&fd, 9).unwrap();
    drop(fd);
    let buf = fs::fs_read(path).unwrap();
    assert_eq!(buf, b"abcdefghi");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_fs_rename() {
    let alpha = b"abcdefghijklmnopqrstuvwxyz\n";
    let src = "src/tmp/test_rename_src";
    let dst = "src/tmp/test_rename_dst";
    fs::fs_write(src, alpha).unwrap();
    fs::fs_rename(src, dst).unwrap();
    assert!(!fs::fs_exists(src));
    let buf = fs::fs_read(dst).unwrap();
    assert_eq!(buf, alpha);
    std::fs::remove_file(dst).ok();
}

#[test]
fn test_fs_mkdir_and_rmdir() {
    let path = "src/tmp/test_dir";
    let _ = fs::fs_rmdir(path);
    fs::fs_mkdir(path, 0o777).unwrap();
    assert!(fs::fs_exists(path));
    fs::fs_rmdir(path).unwrap();
    assert!(!fs::fs_exists(path));
}

#[test]
fn test_fs_fsize() {
    let fd = fs::fs_open("src/tmp/file", fs::FS_OPEN_READ).unwrap();
    let size = fs::fs_fsize(&fd).unwrap();
    assert_eq!(size, 27);
}

#[test]
fn test_fs_read_link_matches_target() {
    let f1 = fs::fs_read("src/tmp/file").unwrap();
    let f2 = fs::fs_read("src/tmp/file.link").unwrap();
    assert_eq!(f1, f2);
}

fn main() {}
