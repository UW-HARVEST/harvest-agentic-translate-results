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

use std::os::unix::io::{AsRawFd, FromRawFd};
use std::io::{Read, Seek, SeekFrom, Write};

pub fn fs_error(s: &str) {
    eprintln!("fs: {}: error: {}", s, std::io::Error::last_os_error());
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    use std::fs::OpenOptions;
    let mut opts = OpenOptions::new();
    match flags {
        "r" | "rb" => { opts.read(true); }
        "w" | "wb" => { opts.write(true).create(true).truncate(true); }
        "rw" | "rwb" => { opts.read(true).write(true); }
        _ => { opts.read(true); }
    }
    match opts.open(path) {
        Ok(file) => {
            let raw_fd = file.as_raw_fd();
            std::mem::forget(file); // prevent closing
            Some(unsafe { fd::OwnedFd::from_raw_fd(raw_fd) })
        }
        Err(_) => None,
    }
}

pub fn fs_close(fd: fd::OwnedFd) -> Result<()> {
    drop(fd); // OwnedFd closes on drop
    Ok(())
}

pub fn fs_rename(from: &str, to: &str) -> Result<()> {
    fs::rename(from, to)
}

pub fn fs_stat(path: &str) -> Result<fs::Metadata> {
    fs::metadata(path)
}

pub fn fs_fstat(fd: &fd::OwnedFd) -> Result<fs::Metadata> {
    use std::fs::File;
    let file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    let meta = file.metadata();
    std::mem::forget(file); // don't close the fd
    meta
}

pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    use std::fs::File;
    let file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    let result = file.set_len(length);
    std::mem::forget(file);
    result
}

pub fn fs_truncate(path: &str, length: u64) -> Result<()> {
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_len(length)?;
    Ok(())
}

pub fn fs_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::chown(path, Some(uid), Some(gid))
}

pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::fchown(fd, Some(uid), Some(gid))
}

pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::lchown(path, Some(uid), Some(gid))
}

pub fn fs_size(path: &str) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    use std::fs::File;
    let file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    let meta = file.metadata();
    std::mem::forget(file);
    Ok(meta?.len())
}

pub fn fs_read(path: &str) -> Result<Vec<u8>> {
    fs::read(path)
}

pub fn fs_nread(path: &str, len: u64) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0u8; len as usize];
    let n = file.read(&mut buffer)?;
    buffer.truncate(n);
    Ok(buffer)
}

pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    use std::fs::File;
    let mut file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    let size = file.seek(SeekFrom::End(0))? as usize;
    file.seek(SeekFrom::Start(0))?;
    let mut buffer = vec![0u8; size];
    let n = file.read(&mut buffer)?;
    buffer.truncate(n);
    std::mem::forget(file);
    Ok(buffer)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    use std::fs::File;
    let mut file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    let mut buffer = vec![0u8; len as usize];
    let n = file.read(&mut buffer)?;
    buffer.truncate(n);
    std::mem::forget(file);
    Ok(buffer)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    let mut file = fs::File::create(path)?;
    let n = file.write(data)?;
    Ok(n as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let mut file = fs::File::create(path)?;
    let to_write = &data[..std::cmp::min(len as usize, data.len())];
    let n = file.write(to_write)?;
    Ok(n as u64)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    use std::fs::File;
    let mut file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    let n = file.write(data)?;
    std::mem::forget(file);
    Ok(n as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    use std::fs::File;
    let mut file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    let to_write = &data[..std::cmp::min(len as usize, data.len())];
    let n = file.write(to_write)?;
    std::mem::forget(file);
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
