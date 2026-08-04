use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Result, Seek, SeekFrom, Write};
use std::os::fd::{self, OwnedFd};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

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
    eprintln!("fs: {}: error: {}", s, io::Error::last_os_error());
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let mut opts = OpenOptions::new();
    match flags {
        "r" | "rb" => {
            opts.read(true);
        }
        "w" | "wb" => {
            opts.write(true).create(true).truncate(true);
        }
        "rw" | "rwb" | "r+" | "rb+" | "r+b" => {
            opts.read(true).write(true).create(true);
        }
        _ => return None,
    }
    let file = opts.open(path).ok()?;
    Some(OwnedFd::from(file))
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
    let file = OpenOptions::new().write(true).open(path)?;
    file.set_len(length)
}

#[cfg(unix)]
pub fn fs_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::chown(path, Some(uid), Some(gid))
}

#[cfg(not(unix))]
pub fn fs_chown(_path: &str, _uid: u32, _gid: u32) -> Result<()> {
    Err(io::Error::from_raw_os_error(38)) // ENOSYS
}

#[cfg(unix)]
pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::fchown(fd, Some(uid), Some(gid))
}

#[cfg(not(unix))]
pub fn fs_fchown(_fd: &fd::OwnedFd, _uid: u32, _gid: u32) -> Result<()> {
    Err(io::Error::from_raw_os_error(38)) // ENOSYS
}

#[cfg(unix)]
pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::lchown(path, Some(uid), Some(gid))
}

#[cfg(not(unix))]
pub fn fs_lchown(_path: &str, _uid: u32, _gid: u32) -> Result<()> {
    Err(io::Error::from_raw_os_error(38)) // ENOSYS
}

pub fn fs_size(path: &str) -> Result<u64> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::End(0))
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    let pos = file.stream_position()?;
    let size = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(pos))?;
    Ok(size)
}

pub fn fs_read(path: &str) -> Result<Vec<u8>> {
    fs::read(path)
}

pub fn fs_nread(path: &str, len: u64) -> Result<Vec<u8>> {
    let mut file = File::open(path)?;
    read_up_to(&mut file, len as usize)
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
    let mut file = File::from(cloned);
    read_up_to(&mut file, len as usize)
}

fn read_up_to<R: Read>(reader: &mut R, len: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    let mut total = 0;
    while total < len {
        match reader.read(&mut buf[total..])? {
            0 => break,
            n => total += n,
        }
    }
    buf.truncate(total);
    Ok(buf)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs_nwrite(path, data, data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let mut file = File::create(path)?;
    write_n(&mut file, data, len as usize)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    fs_fnwrite(fd, data, data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    write_n(&mut file, data, len as usize)
}

fn write_n<W: Write>(writer: &mut W, data: &[u8], len: usize) -> Result<u64> {
    let to_write = &data[..len.min(data.len())];
    writer.write_all(to_write)?;
    Ok(to_write.len() as u64)
}

#[cfg(unix)]
pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    fs::DirBuilder::new().mode(mode).create(path)
}

#[cfg(not(unix))]
pub fn fs_mkdir(path: &str, _mode: u32) -> Result<()> {
    fs::create_dir(path)
}

pub fn fs_rmdir(path: &str) -> Result<()> {
    fs::remove_dir(path)
}

pub fn fs_exists(path: &str) -> bool {
    Path::new(path).exists()
}
