use std::fs;
use std::fs::File;
use std::io::{Read, Result, Write};
use std::os::fd;
use std::os::fd::AsFd;
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
    eprintln!("fs: {}: error: {}", s, std::io::Error::last_os_error());
}
pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let mut opts = fs::OpenOptions::new();
    match flags {
        "r" | "rb" => {
            opts.read(true);
        }
        "w" | "wb" => {
            opts.write(true).create(true).truncate(true);
        }
        "rw" | "rwb" => {
            opts.read(true).write(true);
        }
        "r+" | "rb+" | "r+b" => {
            opts.read(true).write(true);
        }
        "w+" | "wb+" | "w+b" => {
            opts.read(true).write(true).create(true).truncate(true);
        }
        "a" | "ab" => {
            opts.append(true).create(true);
        }
        "a+" | "ab+" | "a+b" => {
            opts.read(true).append(true).create(true);
        }
        _ => return None,
    }
    match opts.open(path) {
        Ok(file) => Some(file.into()),
        Err(_) => None,
    }
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
    let cloned = fd.try_clone()?;
    let file = File::from(cloned);
    file.metadata()
}
pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}
pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    let cloned = fd.try_clone()?;
    let file = File::from(cloned);
    file.set_len(length)
}
pub fn fs_truncate(path: &str, length: u64) -> Result<()> {
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_len(length)
}
pub fn fs_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::chown(path, Some(uid), Some(gid))
}
pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::fchown(fd.as_fd(), Some(uid), Some(gid))
}
pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::lchown(path, Some(uid), Some(gid))
}
pub fn fs_size(path: &str) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}
pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let file = File::from(cloned);
    Ok(file.metadata()?.len())
}
pub fn fs_read(path: &str) -> Result<Vec<u8>> {
    fs::read(path)
}
pub fn fs_nread(path: &str, len: u64) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut buf = Vec::with_capacity(len as usize);
    file.take(len).read_to_end(&mut buf)?;
    Ok(buf)
}
pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}
pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let file = File::from(cloned);
    let mut buf = Vec::with_capacity(len as usize);
    file.take(len).read_to_end(&mut buf)?;
    Ok(buf)
}
pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let n = file.write(data)?;
    Ok(n as u64)
}
pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let to_write = std::cmp::min(len as usize, data.len());
    let n = file.write(&data[..to_write])?;
    Ok(n as u64)
}
pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    let n = file.write(data)?;
    Ok(n as u64)
}
pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    let to_write = std::cmp::min(len as usize, data.len());
    let n = file.write(&data[..to_write])?;
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
