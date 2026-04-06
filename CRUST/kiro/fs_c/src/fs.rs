use std::fs;
use std::os::fd;
use std::io::Result;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::fs::PermissionsExt;

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
    opts.open(path).ok().map(|f| fd::OwnedFd::from(f))
}

pub fn fs_close(_fd: fd::OwnedFd) -> Result<()> {
    // OwnedFd drops and closes automatically
    Ok(())
}

pub fn fs_rename(from: &str, to: &str) -> Result<()> {
    fs::rename(from, to)
}

pub fn fs_stat(path: &str) -> Result<fs::Metadata> {
    fs::metadata(path)
}

pub fn fs_fstat(fd: &fd::OwnedFd) -> Result<fs::Metadata> {
    let f = fd_to_file_ref(fd);
    let meta = f.metadata();
    std::mem::forget(f);
    meta
}

pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    let f = fd_to_file_ref(fd);
    let res = f.set_len(length);
    std::mem::forget(f);
    res
}

pub fn fs_truncate(path: &str, length: u64) -> Result<()> {
    let f = fs::OpenOptions::new().write(true).open(path)?;
    f.set_len(length)
}

pub fn fs_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    use std::ffi::CString;
    let c_path = CString::new(path).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { nix_chown(c_path.as_ptr(), uid, gid) };
    if ret == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}

pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    let ret = unsafe { nix_fchown(fd.as_raw_fd(), uid, gid) };
    if ret == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}

pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    use std::ffi::CString;
    let c_path = CString::new(path).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    let ret = unsafe { nix_lchown(c_path.as_ptr(), uid, gid) };
    if ret == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}

pub fn fs_size(path: &str) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    let mut f = fd_to_file_ref(fd);
    let pos = f.stream_position()?;
    let size = f.seek(SeekFrom::End(0))?;
    f.seek(SeekFrom::Start(pos))?;
    std::mem::forget(f);
    Ok(size)
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
    let size = fs_fsize(fd)?;
    fs_fnread(fd, size)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let mut f = fd_to_file_ref(fd);
    let mut buf = vec![0u8; len as usize];
    let n = f.read(&mut buf)?;
    buf.truncate(n);
    std::mem::forget(f);
    Ok(buf)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs::write(path, data)?;
    Ok(data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let to_write = &data[..len as usize];
    fs::write(path, to_write)?;
    Ok(to_write.len() as u64)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    fs_fnwrite(fd, data, data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let mut f = fd_to_file_ref(fd);
    let n = f.write(&data[..len as usize])?;
    std::mem::forget(f);
    Ok(n as u64)
}

pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    fs::DirBuilder::new().create(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

pub fn fs_rmdir(path: &str) -> Result<()> {
    fs::remove_dir(path)
}

pub fn fs_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

// Helper: borrow an OwnedFd as a File without taking ownership
fn fd_to_file_ref(fd: &fd::OwnedFd) -> fs::File {
    unsafe { fs::File::from_raw_fd(fd.as_raw_fd()) }
}

// Minimal extern declarations for chown/fchown/lchown (no libc crate needed)
extern "C" {
    fn chown(path: *const std::ffi::c_char, owner: u32, group: u32) -> i32;
    fn fchown(fd: i32, owner: u32, group: u32) -> i32;
    fn lchown(path: *const std::ffi::c_char, owner: u32, group: u32) -> i32;
}

unsafe fn nix_chown(path: *const std::ffi::c_char, uid: u32, gid: u32) -> i32 {
    chown(path, uid, gid)
}

unsafe fn nix_fchown(fd: i32, uid: u32, gid: u32) -> i32 {
    fchown(fd, uid, gid)
}

unsafe fn nix_lchown(path: *const std::ffi::c_char, uid: u32, gid: u32) -> i32 {
    lchown(path, uid, gid)
}
