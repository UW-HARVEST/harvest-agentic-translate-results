#![allow(dead_code, unused_imports)]

use fs_c::fs;

const ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyz\n";

fn setup_dir(name: &str) -> String {
    let dir = format!("./tmp_test_{}", name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create test dir");
    dir
}

fn cleanup_dir(dir: &str) {
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_open_existing_for_read() {
    let dir = setup_dir("open_existing_read");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let fd = fs::fs_open(&path, fs::FS_OPEN_READ);
    assert!(fd.is_some(), "fs_open of existing file for read should succeed");
    drop(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_open_for_write_creates_file() {
    let dir = setup_dir("open_write_create");
    let path = format!("{}/created", dir);
    assert!(!fs::fs_exists(&path));

    let fd = fs::fs_open(&path, fs::FS_OPEN_WRITE);
    assert!(fd.is_some(), "fs_open with write should create the file");
    drop(fd);

    assert!(fs::fs_exists(&path));
    let meta = std::fs::metadata(&path).unwrap();
    assert_eq!(meta.len(), 0, "fs_open with write truncates to zero");

    cleanup_dir(&dir);
}

#[test]
fn test_fs_open_nonexistent_for_read_returns_none() {
    let path = "./tmp_test_nonexistent/no_such_file_xyz";
    assert!(fs::fs_open(path, fs::FS_OPEN_READ).is_none());
}

#[test]
fn test_fs_open_inaccessible_returns_none() {
    // /root/foo is not writable for non-root users
    let fd = fs::fs_open("/root/foo", fs::FS_OPEN_WRITE);
    assert!(fd.is_none(), "should not be able to open /root/foo for write");
}

#[test]
fn test_fs_close_drops_fd() {
    let dir = setup_dir("close");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let fd = fs::fs_open(&path, fs::FS_OPEN_READ).expect("open");
    let res = fs::fs_close(fd);
    assert!(res.is_ok());

    cleanup_dir(&dir);
}

#[test]
fn test_fs_rename() {
    let dir = setup_dir("rename");
    let from = format!("{}/from", dir);
    let to = format!("{}/to", dir);
    std::fs::write(&from, ALPHA).unwrap();

    let res = fs::fs_rename(&from, &to);
    assert!(res.is_ok());
    assert!(!fs::fs_exists(&from));
    assert!(fs::fs_exists(&to));
    let buf = std::fs::read(&to).unwrap();
    assert_eq!(buf, ALPHA);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_rename_nonexistent_errors() {
    let dir = setup_dir("rename_err");
    let from = format!("{}/missing", dir);
    let to = format!("{}/dest", dir);
    let res = fs::fs_rename(&from, &to);
    assert!(res.is_err());
    cleanup_dir(&dir);
}

#[test]
fn test_fs_stat_existing() {
    let dir = setup_dir("stat");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let meta = fs::fs_stat(&path).expect("stat");
    assert_eq!(meta.len(), ALPHA.len() as u64);
    assert!(meta.is_file());
    assert!(!meta.is_dir());

    cleanup_dir(&dir);
}

#[test]
fn test_fs_stat_nonexistent() {
    let res = fs::fs_stat("./tmp_test_stat/no_such_path_xyz");
    assert!(res.is_err());
}

#[test]
fn test_fs_fstat_existing() {
    let dir = setup_dir("fstat");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let fd = fs::fs_open(&path, fs::FS_OPEN_READ).expect("open");
    let meta = fs::fs_fstat(&fd).expect("fstat");
    assert_eq!(meta.len(), ALPHA.len() as u64);
    assert!(meta.is_file());
    drop(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_lstat_regular_file() {
    let dir = setup_dir("lstat");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let meta = fs::fs_lstat(&path).expect("lstat");
    assert_eq!(meta.len(), ALPHA.len() as u64);
    assert!(meta.is_file());

    cleanup_dir(&dir);
}

#[test]
fn test_fs_lstat_symlink() {
    let dir = setup_dir("lstat_symlink");
    let target = format!("{}/file", dir);
    let link = format!("{}/link", dir);
    std::fs::write(&target, ALPHA).unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let meta = fs::fs_lstat(&link).expect("lstat");
    // lstat does NOT follow symlinks - should report it as a symlink
    assert!(meta.file_type().is_symlink());

    cleanup_dir(&dir);
}

#[test]
fn test_fs_ftruncate() {
    let dir = setup_dir("ftruncate");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let fd = fs::fs_open(&path, fs::FS_OPEN_READWRITE).expect("open");
    let res = fs::fs_ftruncate(&fd, 9);
    assert!(res.is_ok());
    drop(fd);

    let buf = std::fs::read(&path).unwrap();
    assert_eq!(buf, b"abcdefghi");
    assert_eq!(buf.len(), 9);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_truncate() {
    let dir = setup_dir("truncate");
    let path = format!("{}/alpha", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let res = fs::fs_truncate(&path, 9);
    assert!(res.is_ok());

    let buf = std::fs::read(&path).unwrap();
    assert_eq!(buf, b"abcdefghi");

    cleanup_dir(&dir);
}

#[test]
fn test_fs_truncate_nonexistent_errors() {
    let res = fs::fs_truncate("./tmp_test_truncate/no_such", 0);
    assert!(res.is_err());
}

#[test]
fn test_fs_chown_to_self() {
    let dir = setup_dir("chown");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let uid = unsafe { libc_getuid() };
    let gid = unsafe { libc_getgid() };
    let res = fs::fs_chown(&path, uid, gid);
    assert!(res.is_ok(), "chown to self should succeed: {:?}", res);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fchown_to_self() {
    let dir = setup_dir("fchown");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();
    let fd = fs::fs_open(&path, fs::FS_OPEN_READWRITE).expect("open");
    let uid = unsafe { libc_getuid() };
    let gid = unsafe { libc_getgid() };
    let res = fs::fs_fchown(&fd, uid, gid);
    assert!(res.is_ok());
    drop(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_lchown_to_self() {
    let dir = setup_dir("lchown");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();
    let uid = unsafe { libc_getuid() };
    let gid = unsafe { libc_getgid() };
    let res = fs::fs_lchown(&path, uid, gid);
    assert!(res.is_ok());

    cleanup_dir(&dir);
}

#[test]
fn test_fs_size() {
    let dir = setup_dir("size");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let s = fs::fs_size(&path).expect("size");
    assert_eq!(s, ALPHA.len() as u64);
    assert_eq!(s, 27);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_size_nonexistent_errors() {
    let res = fs::fs_size("./tmp_test_size/no_such");
    assert!(res.is_err());
}

#[test]
fn test_fs_fsize_preserves_position() {
    let dir = setup_dir("fsize");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let fd = fs::fs_open(&path, fs::FS_OPEN_READ).expect("open");
    let s = fs::fs_fsize(&fd).expect("fsize");
    assert_eq!(s, ALPHA.len() as u64);
    drop(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_read() {
    let dir = setup_dir("read");
    let path = format!("{}/alpha", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let buf = fs::fs_read(&path).expect("read");
    assert_eq!(buf, ALPHA);
    assert_eq!(buf.len(), 27);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_read_nonexistent_errors() {
    let res = fs::fs_read("./tmp_test_read/no_such");
    assert!(res.is_err());
}

#[test]
fn test_fs_nread() {
    let dir = setup_dir("nread");
    let path = format!("{}/alpha", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let buf = fs::fs_nread(&path, 9).expect("nread");
    assert_eq!(buf, b"abcdefghi");
    assert_eq!(buf.len(), 9);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_nread_zero() {
    let dir = setup_dir("nread_zero");
    let path = format!("{}/alpha", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let buf = fs::fs_nread(&path, 0).expect("nread");
    assert_eq!(buf.len(), 0);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fread() {
    let dir = setup_dir("fread");
    let path = format!("{}/alpha", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let fd = fs::fs_open(&path, fs::FS_OPEN_READ).expect("open");
    let buf = fs::fs_fread(&fd).expect("fread");
    assert_eq!(buf, ALPHA);
    drop(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fnread() {
    let dir = setup_dir("fnread");
    let path = format!("{}/alpha", dir);
    std::fs::write(&path, ALPHA).unwrap();

    let fd = fs::fs_open(&path, fs::FS_OPEN_READ).expect("open");
    let buf = fs::fs_fnread(&fd, 5).expect("fnread");
    assert_eq!(buf, b"abcde");
    assert_eq!(buf.len(), 5);
    drop(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_write_returns_byte_count() {
    let dir = setup_dir("write");
    let path = format!("{}/alpha", dir);

    let n = fs::fs_write(&path, ALPHA).expect("write");
    assert_eq!(n, ALPHA.len() as u64);

    let buf = std::fs::read(&path).unwrap();
    assert_eq!(buf, ALPHA);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_nwrite_partial() {
    let dir = setup_dir("nwrite");
    let path = format!("{}/alpha", dir);

    let n = fs::fs_nwrite(&path, ALPHA, 9).expect("nwrite");
    assert_eq!(n, 9);

    let buf = std::fs::read(&path).unwrap();
    assert_eq!(buf, b"abcdefghi");

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fwrite() {
    let dir = setup_dir("fwrite");
    let path = format!("{}/alpha", dir);

    let fd = fs::fs_open(&path, fs::FS_OPEN_WRITE).expect("open");
    let n = fs::fs_fwrite(&fd, ALPHA).expect("fwrite");
    assert_eq!(n, ALPHA.len() as u64);
    drop(fd);

    let buf = std::fs::read(&path).unwrap();
    assert_eq!(buf, ALPHA);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fnwrite_partial() {
    let dir = setup_dir("fnwrite");
    let path = format!("{}/alpha", dir);

    let fd = fs::fs_open(&path, fs::FS_OPEN_WRITE).expect("open");
    let n = fs::fs_fnwrite(&fd, ALPHA, 9).expect("fnwrite");
    assert_eq!(n, 9);
    drop(fd);

    let buf = std::fs::read(&path).unwrap();
    assert_eq!(buf, b"abcdefghi");

    cleanup_dir(&dir);
}

#[test]
fn test_fs_mkdir_and_rmdir() {
    let dir = setup_dir("mkdir");
    let sub = format!("{}/dir", dir);

    let res = fs::fs_mkdir(&sub, 0o777);
    assert!(res.is_ok());
    assert!(fs::fs_exists(&sub));

    let res = fs::fs_rmdir(&sub);
    assert!(res.is_ok());
    assert!(!fs::fs_exists(&sub));

    cleanup_dir(&dir);
}

#[test]
fn test_fs_mkdir_existing_errors() {
    let dir = setup_dir("mkdir_exists");
    let res = fs::fs_mkdir(&dir, 0o777);
    assert!(res.is_err(), "mkdir on existing dir should error");
    cleanup_dir(&dir);
}

#[test]
fn test_fs_rmdir_nonexistent_errors() {
    let res = fs::fs_rmdir("./tmp_test_rmdir/no_such_dir_xyz");
    assert!(res.is_err());
}

#[test]
fn test_fs_exists_true() {
    let dir = setup_dir("exists_true");
    let path = format!("{}/file", dir);
    std::fs::write(&path, ALPHA).unwrap();

    assert!(fs::fs_exists(&path));
    assert!(fs::fs_exists(&dir));

    cleanup_dir(&dir);
}

#[test]
fn test_fs_exists_false() {
    assert!(!fs::fs_exists("./tmp_test_exists/no_such_path_xyz_quux"));
    assert!(!fs::fs_exists("/this/path/should/not/exist/in/the/world/xyz123"));
}

#[test]
fn test_fs_error_does_not_panic() {
    // Just ensure it runs without panicking; output goes to stderr.
    fs::fs_error("test_prefix");
}

// Local libc bindings to avoid pulling in a crate dependency.
extern "C" {
    fn getuid() -> u32;
    fn getgid() -> u32;
}
unsafe fn libc_getuid() -> u32 {
    getuid()
}
unsafe fn libc_getgid() -> u32 {
    getgid()
}

fn main() {}
