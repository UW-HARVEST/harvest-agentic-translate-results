use fs_c::fs::*;
use std::fs as stdfs;
use std::path::Path;

const ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyz\n";

fn setup_dir() -> String {
    // Use a unique tmp directory for our tests
    let dir = format!(
        "./test_tmp_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let _ = stdfs::create_dir_all(&dir);
    dir
}

fn cleanup_dir(dir: &str) {
    let _ = stdfs::remove_dir_all(dir);
}

#[test]
fn test_fs_open_success() {
    let dir = setup_dir();
    let path = format!("{}/file_open", dir);
    stdfs::write(&path, b"hello").unwrap();

    let fd = fs_open(&path, FS_OPEN_READ);
    assert!(fd.is_some(), "fs_open should succeed for existing file");

    cleanup_dir(&dir);
}

#[test]
fn test_fs_open_failure() {
    // Try to open non-existent path for reading
    let fd = fs_open("/root/nonexistent_path_xyz_123", FS_OPEN_READ);
    assert!(fd.is_none(), "fs_open should fail for unwritable/missing path");
}

#[test]
fn test_fs_close_returns_ok() {
    let dir = setup_dir();
    let path = format!("{}/file_close", dir);
    stdfs::write(&path, b"data").unwrap();

    let fd = fs_open(&path, FS_OPEN_READ).expect("open");
    let r = fs_close(fd);
    assert!(r.is_ok(), "fs_close should return Ok");

    cleanup_dir(&dir);
}

#[test]
fn test_fs_rename() {
    let dir = setup_dir();
    let from = format!("{}/from", dir);
    let to = format!("{}/to", dir);
    stdfs::write(&from, ALPHA).unwrap();

    let r = fs_rename(&from, &to);
    assert!(r.is_ok());
    assert!(!fs_exists(&from));
    assert!(fs_exists(&to));

    let buf = fs_read(&to).unwrap();
    assert_eq!(buf, ALPHA);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_stat_success() {
    let dir = setup_dir();
    let path = format!("{}/stat_file", dir);
    stdfs::write(&path, ALPHA).unwrap();

    let m = fs_stat(&path).expect("stat");
    assert_eq!(m.len(), ALPHA.len() as u64);
    assert!(m.is_file());

    cleanup_dir(&dir);
}

#[test]
fn test_fs_stat_failure() {
    let r = fs_stat("./nonexistent_xyz_path_test");
    assert!(r.is_err());
}

#[test]
fn test_fs_fstat_success() {
    let dir = setup_dir();
    let path = format!("{}/fstat_file", dir);
    stdfs::write(&path, ALPHA).unwrap();

    let fd = fs_open(&path, FS_OPEN_READ).expect("open");
    let m = fs_fstat(&fd).expect("fstat");
    assert_eq!(m.len(), ALPHA.len() as u64);
    assert!(m.is_file());
    let _ = fs_close(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_lstat() {
    let dir = setup_dir();
    let path = format!("{}/lstat_file", dir);
    stdfs::write(&path, ALPHA).unwrap();

    let m = fs_lstat(&path).expect("lstat");
    assert_eq!(m.len(), ALPHA.len() as u64);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_lstat_symlink() {
    #[cfg(unix)]
    {
        let dir = setup_dir();
        let target = format!("{}/lstat_target", dir);
        let link = format!("{}/lstat_link", dir);
        stdfs::write(&target, ALPHA).unwrap();
        // Use a relative path that's relative to the link's containing
        // directory; "lstat_target" is in the same dir as "lstat_link".
        std::os::unix::fs::symlink("lstat_target", &link).unwrap();

        let m_lstat = fs_lstat(&link).expect("lstat");
        assert!(m_lstat.file_type().is_symlink());

        let m_stat = fs_stat(&link).expect("stat");
        assert!(!m_stat.file_type().is_symlink());
        assert!(m_stat.is_file());
        assert_eq!(m_stat.len(), ALPHA.len() as u64);

        cleanup_dir(&dir);
    }
}

#[test]
fn test_fs_truncate() {
    let dir = setup_dir();
    let path = format!("{}/trunc_file", dir);
    fs_write(&path, ALPHA).unwrap();
    let r = fs_truncate(&path, 9);
    assert!(r.is_ok());

    let buf = fs_read(&path).unwrap();
    assert_eq!(buf, b"abcdefghi");
    assert_eq!(buf.len(), 9);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_ftruncate() {
    let dir = setup_dir();
    let path = format!("{}/ftrunc_file", dir);
    fs_write(&path, ALPHA).unwrap();

    let fd = fs_open(&path, "rw").expect("open");
    let r = fs_ftruncate(&fd, 9);
    assert!(r.is_ok());
    let _ = fs_close(fd);

    let buf = fs_read(&path).unwrap();
    assert_eq!(buf, b"abcdefghi");

    cleanup_dir(&dir);
}

#[test]
fn test_fs_size() {
    let dir = setup_dir();
    let path = format!("{}/size_file", dir);
    stdfs::write(&path, ALPHA).unwrap();

    let s = fs_size(&path).expect("size");
    assert_eq!(s, ALPHA.len() as u64);
    assert_eq!(s, 27);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fsize() {
    let dir = setup_dir();
    let path = format!("{}/fsize_file", dir);
    stdfs::write(&path, ALPHA).unwrap();

    let fd = fs_open(&path, FS_OPEN_READ).expect("open");
    let s = fs_fsize(&fd).expect("fsize");
    assert_eq!(s, ALPHA.len() as u64);
    assert_eq!(s, 27);
    let _ = fs_close(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_read() {
    let dir = setup_dir();
    let path = format!("{}/read_file", dir);
    stdfs::write(&path, ALPHA).unwrap();

    let buf = fs_read(&path).expect("read");
    assert_eq!(buf, ALPHA);
    assert_eq!(buf.len(), 27);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_read_failure() {
    let r = fs_read("./nonexistent_read_xyz");
    assert!(r.is_err());
}

#[test]
fn test_fs_nread() {
    let dir = setup_dir();
    let path = format!("{}/nread_file", dir);
    fs_write(&path, ALPHA).unwrap();

    let buf = fs_nread(&path, 9).expect("nread");
    assert_eq!(buf, b"abcdefghi");
    assert_eq!(buf.len(), 9);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fread() {
    let dir = setup_dir();
    let path = format!("{}/fread_file", dir);
    fs_write(&path, ALPHA).unwrap();

    let fd = fs_open(&path, FS_OPEN_READ).expect("open");
    let buf = fs_fread(&fd).expect("fread");
    assert_eq!(buf, ALPHA);
    let _ = fs_close(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fnread() {
    let dir = setup_dir();
    let path = format!("{}/fnread_file", dir);
    fs_write(&path, ALPHA).unwrap();

    let fd = fs_open(&path, FS_OPEN_READ).expect("open");
    let buf = fs_fnread(&fd, 9).expect("fnread");
    assert_eq!(buf, b"abcdefghi");
    assert_eq!(buf.len(), 9);
    let _ = fs_close(fd);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_write() {
    let dir = setup_dir();
    let path = format!("{}/write_file", dir);

    let n = fs_write(&path, ALPHA).expect("write");
    assert_eq!(n, ALPHA.len() as u64);
    assert_eq!(n, 27);

    let buf = fs_read(&path).unwrap();
    assert_eq!(buf, ALPHA);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_nwrite() {
    let dir = setup_dir();
    let path = format!("{}/nwrite_file", dir);

    let n = fs_nwrite(&path, ALPHA, 9).expect("nwrite");
    assert_eq!(n, 9);

    let buf = fs_read(&path).unwrap();
    assert_eq!(buf, b"abcdefghi");
    assert_eq!(buf.len(), 9);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fwrite() {
    let dir = setup_dir();
    let path = format!("{}/fwrite_file", dir);

    let fd = fs_open(&path, FS_OPEN_WRITE).expect("open");
    let n = fs_fwrite(&fd, ALPHA).expect("fwrite");
    assert_eq!(n, ALPHA.len() as u64);
    assert_eq!(n, 27);
    let _ = fs_close(fd);

    let buf = fs_read(&path).unwrap();
    assert_eq!(buf, ALPHA);

    cleanup_dir(&dir);
}

#[test]
fn test_fs_fnwrite() {
    let dir = setup_dir();
    let path = format!("{}/fnwrite_file", dir);

    let fd = fs_open(&path, FS_OPEN_WRITE).expect("open");
    let n = fs_fnwrite(&fd, ALPHA, 9).expect("fnwrite");
    assert_eq!(n, 9);
    let _ = fs_close(fd);

    let buf = fs_read(&path).unwrap();
    assert_eq!(buf, b"abcdefghi");

    cleanup_dir(&dir);
}

#[test]
fn test_fs_mkdir_rmdir() {
    let dir = setup_dir();
    let subdir = format!("{}/dir_test", dir);
    let _ = fs_rmdir(&subdir);

    let r = fs_mkdir(&subdir, 0o777);
    assert!(r.is_ok());
    assert!(fs_exists(&subdir));
    assert!(Path::new(&subdir).is_dir());

    let r2 = fs_rmdir(&subdir);
    assert!(r2.is_ok());
    assert!(!fs_exists(&subdir));

    cleanup_dir(&dir);
}

#[test]
fn test_fs_exists_true() {
    let dir = setup_dir();
    let path = format!("{}/exists_file", dir);
    stdfs::write(&path, b"x").unwrap();

    assert!(fs_exists(&path));
    assert!(fs_exists(&dir));

    cleanup_dir(&dir);
}

#[test]
fn test_fs_exists_false() {
    assert!(!fs_exists("./this_path_does_not_exist_xyz_123"));
}

#[test]
fn test_fs_error_does_not_panic() {
    // Just confirms the function executes without panic; output goes to stderr.
    fs_error("test_prefix");
}

#[cfg(unix)]
fn current_uid_gid(path: &str) -> (u32, u32) {
    use std::os::unix::fs::MetadataExt;
    let m = stdfs::metadata(path).unwrap();
    (m.uid(), m.gid())
}

#[test]
fn test_fs_chown_self() {
    #[cfg(unix)]
    {
        let dir = setup_dir();
        let path = format!("{}/chown_file", dir);
        stdfs::write(&path, b"x").unwrap();

        // chown to current uid/gid should succeed (file already owned by us).
        let (uid, gid) = current_uid_gid(&path);
        let r = fs_chown(&path, uid, gid);
        assert!(r.is_ok(), "chown to self should succeed: {:?}", r);

        cleanup_dir(&dir);
    }
}

#[test]
fn test_fs_fchown_self() {
    #[cfg(unix)]
    {
        let dir = setup_dir();
        let path = format!("{}/fchown_file", dir);
        stdfs::write(&path, b"x").unwrap();

        let (uid, gid) = current_uid_gid(&path);
        let fd = fs_open(&path, "rw").expect("open");
        let r = fs_fchown(&fd, uid, gid);
        assert!(r.is_ok(), "fchown to self should succeed: {:?}", r);
        let _ = fs_close(fd);

        cleanup_dir(&dir);
    }
}

#[test]
fn test_fs_lchown_self() {
    #[cfg(unix)]
    {
        let dir = setup_dir();
        let path = format!("{}/lchown_file", dir);
        stdfs::write(&path, b"x").unwrap();

        let (uid, gid) = current_uid_gid(&path);
        let r = fs_lchown(&path, uid, gid);
        assert!(r.is_ok(), "lchown to self should succeed: {:?}", r);

        cleanup_dir(&dir);
    }
}

fn main() {}
