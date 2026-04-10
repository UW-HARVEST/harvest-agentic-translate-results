use std::fs;
use std::os::fd;
use std::io::Result;
#[cfg(target_os = "windows")]
pub const FS_OPEN_READ: &str = "rb";
#[cfg(target_os = "windows")]
pub const FS_OPEN_WRITE: &str = "wb";
#[cfg(target_os = "windows")]
pub const FS_OPEN_READWRITE: &str = "rwb";
#[cfg(not(target_os = "windows"))]
pub const FS_OPEN_READ: &str = "r";
#[cfg(not(target_os = "windows"))]
pub const FS_OPEN_WRITE: &str = "w";
#[cfg(not(target_os = "windows"))]
pub const FS_OPEN_READWRITE: &str = "rw";

use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;

fn borrow_as_file(fd: &fd::OwnedFd) -> Result<fs::File> {
    let cloned = fd.try_clone()?;
    Ok(fs::File::from(cloned))
}

pub fn fs_error(s: &str) {
    eprintln!("fs: {}: error", s);
}
pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let mut opts = fs::OpenOptions::new();
    match flags {
        "r" | "rb" => { opts.read(true); }
        "w" | "wb" => { opts.write(true).create(true).truncate(true); }
        "rw" | "rwb" => { opts.read(true).write(true); }
        _ => { opts.read(true); }
    }
    let file = opts.open(path).ok()?;
    Some(fd::OwnedFd::from(file))
}
pub fn fs_close(fd: fd::OwnedFd) -> Result<()> {
    drop(fd);
    Ok(())
}
pub fn fs_rename(from: &str, to: &str) -> Result<()> {
    fs::rename(from, to)
}
pub fn fs_stat(path: &str) -> Result<fs::Metadata> {
    fs::metadata(path)
}
pub fn fs_fstat(fd: &fd::OwnedFd) -> Result<fs::Metadata> {
    borrow_as_file(fd)?.metadata()
}
pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}
pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    borrow_as_file(fd)?.set_len(length)
}
pub fn fs_truncate(path: &str, length: u64) -> Result<()> {
    let f = fs::OpenOptions::new().write(true).open(path)?;
    f.set_len(length)
}
pub fn fs_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::chown(path, Some(uid), Some(gid))
}
pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    let proc_path = format!("/proc/self/fd/{}", fd.as_raw_fd());
    std::os::unix::fs::chown(proc_path, Some(uid), Some(gid))
}
pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::lchown(path, Some(uid), Some(gid))
}
pub fn fs_size(path: &str) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}
pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    Ok(borrow_as_file(fd)?.metadata()?.len())
}
pub fn fs_read(path: &str) -> Result<Vec<u8>> {
    fs::read(path)
}
pub fn fs_nread(path: &str, len: u64) -> Result<Vec<u8>> {
    let mut f = fs::File::open(path)?;
    let mut buf = vec![0u8; len as usize];
    let n = f.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}
pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let mut f = borrow_as_file(fd)?;
    let size = f.metadata()?.len();
    f.seek(SeekFrom::Start(0))?;
    let mut buf = vec![0u8; size as usize];
    let n = f.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}
pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let mut f = borrow_as_file(fd)?;
    let mut buf = vec![0u8; len as usize];
    let n = f.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}
pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs::write(path, data)?;
    Ok(data.len() as u64)
}
pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let to_write = &data[..std::cmp::min(len as usize, data.len())];
    let mut f = fs::File::create(path)?;
    f.write_all(to_write)?;
    Ok(to_write.len() as u64)
}
pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    let mut f = borrow_as_file(fd)?;
    let n = f.write(data)?;
    Ok(n as u64)
}
pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let to_write = &data[..std::cmp::min(len as usize, data.len())];
    let mut f = borrow_as_file(fd)?;
    let n = f.write(to_write)?;
    Ok(n as u64)
}
pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(mode).create(path)
}
pub fn fs_rmdir(path: &str) -> Result<()> {
    fs::remove_dir(path)
}
pub fn fs_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}
