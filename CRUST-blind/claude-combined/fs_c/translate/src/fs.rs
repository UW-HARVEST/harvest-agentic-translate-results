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
use std::os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd};

/// Convert flags string into OpenOptions
fn options_from_flags(flags: &str) -> fs::OpenOptions {
    let mut opts = fs::OpenOptions::new();
    // Strip a trailing 'b' for Windows-style binary mode
    let normalized: String = flags.chars().filter(|c| *c != 'b').collect();
    match normalized.as_str() {
        "r" => {
            opts.read(true);
        }
        "w" => {
            opts.write(true).create(true).truncate(true);
        }
        "rw" => {
            opts.read(true).write(true);
        }
        "a" => {
            opts.append(true).create(true);
        }
        "r+" => {
            opts.read(true).write(true);
        }
        "w+" => {
            opts.read(true).write(true).create(true).truncate(true);
        }
        "a+" => {
            opts.read(true).append(true).create(true);
        }
        _ => {
            // Default to read
            opts.read(true);
        }
    }
    opts
}

pub fn fs_error(s: &str) {
    let err = std::io::Error::last_os_error();
    eprintln!("fs: {}: error: {}", s, err);
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let opts = options_from_flags(flags);
    match opts.open(path) {
        Ok(file) => {
            let raw = file.into_raw_fd();
            // SAFETY: raw fd just produced from a valid File
            Some(unsafe { fd::OwnedFd::from_raw_fd(raw) })
        }
        Err(_) => None,
    }
}

pub fn fs_close(fd: fd::OwnedFd) -> Result<()> {
    // Dropping OwnedFd closes the file. Use into_raw_fd + close for explicit
    // result reporting, but Rust's drop also closes. To return a proper
    // Result, we can rely on Drop semantics — drop here.
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
    // Construct a borrowed File from the raw fd to call metadata().
    let raw = fd.as_fd().as_raw_fd();
    // SAFETY: we will not close this File on drop because we use ManuallyDrop.
    let file = unsafe { fs::File::from_raw_fd(raw) };
    let file = std::mem::ManuallyDrop::new(file);
    file.metadata()
}

pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    let raw = fd.as_fd().as_raw_fd();
    let file = unsafe { fs::File::from_raw_fd(raw) };
    let file = std::mem::ManuallyDrop::new(file);
    file.set_len(length)
}

pub fn fs_truncate(path: &str, length: u64) -> Result<()> {
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_len(length)
}

#[cfg(unix)]
pub fn fs_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::chown(path, Some(uid), Some(gid))
}

#[cfg(not(unix))]
pub fn fs_chown(_path: &str, _uid: u32, _gid: u32) -> Result<()> {
    Err(std::io::Error::from_raw_os_error(38)) // ENOSYS
}

#[cfg(unix)]
pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    let raw = fd.as_fd().as_raw_fd();
    let file = unsafe { fs::File::from_raw_fd(raw) };
    let file = std::mem::ManuallyDrop::new(file);
    std::os::unix::fs::fchown(&*file, Some(uid), Some(gid))
}

#[cfg(not(unix))]
pub fn fs_fchown(_fd: &fd::OwnedFd, _uid: u32, _gid: u32) -> Result<()> {
    Err(std::io::Error::from_raw_os_error(38))
}

#[cfg(unix)]
pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::lchown(path, Some(uid), Some(gid))
}

#[cfg(not(unix))]
pub fn fs_lchown(_path: &str, _uid: u32, _gid: u32) -> Result<()> {
    Err(std::io::Error::from_raw_os_error(38))
}

pub fn fs_size(path: &str) -> Result<u64> {
    let mut file = fs::File::open(path)?;
    let size = file.seek(SeekFrom::End(0))?;
    Ok(size)
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    let raw = fd.as_fd().as_raw_fd();
    let file = unsafe { fs::File::from_raw_fd(raw) };
    let mut file = std::mem::ManuallyDrop::new(file);
    let pos = file.stream_position()?;
    file.seek(SeekFrom::Start(0))?;
    let size = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(pos))?;
    Ok(size)
}

pub fn fs_read(path: &str) -> Result<Vec<u8>> {
    fs::read(path)
}

pub fn fs_nread(path: &str, len: u64) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut buf = vec![0u8; len as usize];
    let n = file.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}

pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let size = fs_fsize(fd)?;
    fs_fnread(fd, size)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let raw = fd.as_fd().as_raw_fd();
    let file = unsafe { fs::File::from_raw_fd(raw) };
    let mut file = std::mem::ManuallyDrop::new(file);
    let mut buf = vec![0u8; len as usize];
    let n = file.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs_nwrite(path, data, data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let len = len as usize;
    let slice = &data[..len.min(data.len())];
    file.write_all(slice)?;
    Ok(slice.len() as u64)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    fs_fnwrite(fd, data, data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let raw = fd.as_fd().as_raw_fd();
    let file = unsafe { fs::File::from_raw_fd(raw) };
    let mut file = std::mem::ManuallyDrop::new(file);
    let len = len as usize;
    let slice = &data[..len.min(data.len())];
    file.write_all(slice)?;
    Ok(slice.len() as u64)
}

#[cfg(unix)]
pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    let mut b = fs::DirBuilder::new();
    b.mode(mode);
    b.create(path)
}

#[cfg(not(unix))]
pub fn fs_mkdir(path: &str, _mode: u32) -> Result<()> {
    fs::create_dir(path)
}

pub fn fs_rmdir(path: &str) -> Result<()> {
    fs::remove_dir(path)
}

pub fn fs_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}
