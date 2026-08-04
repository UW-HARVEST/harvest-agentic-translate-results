use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Result, Write};
use std::os::fd;

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
    let err = std::io::Error::last_os_error();
    eprintln!("fs: {}: error: {}", s, err);
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
        "rw" | "rwb" => {
            opts.read(true).write(true).create(true);
        }
        _ => {
            opts.read(true);
        }
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

pub fn fs_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::chown(path, Some(uid), Some(gid))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, uid, gid);
        Err(std::io::Error::from_raw_os_error(38))
    }
}

pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        let cloned = fd.try_clone()?;
        let file = File::from(cloned);
        std::os::unix::fs::fchown(&file, Some(uid), Some(gid))
    }
    #[cfg(not(unix))]
    {
        let _ = (fd, uid, gid);
        Err(std::io::Error::from_raw_os_error(38))
    }
}

pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::lchown(path, Some(uid), Some(gid))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, uid, gid);
        Err(std::io::Error::from_raw_os_error(38))
    }
}

pub fn fs_size(path: &str) -> Result<u64> {
    fs::metadata(path).map(|m| m.len())
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
    let mut file = File::open(path)?;
    let mut buffer = Vec::with_capacity(len as usize);
    std::io::Read::by_ref(&mut file)
        .take(len)
        .read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    let mut buffer = Vec::with_capacity(len as usize);
    std::io::Read::by_ref(&mut file)
        .take(len)
        .read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs::write(path, data)?;
    Ok(data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let len_usize = len as usize;
    let slice = if len_usize <= data.len() {
        &data[..len_usize]
    } else {
        data
    };
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(slice)?;
    Ok(slice.len() as u64)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    file.write_all(data)?;
    Ok(data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let len_usize = len as usize;
    let slice = if len_usize <= data.len() {
        &data[..len_usize]
    } else {
        data
    };
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    file.write_all(slice)?;
    Ok(slice.len() as u64)
}

pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(mode).create(path)
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
        fs::create_dir(path)
    }
}

pub fn fs_rmdir(path: &str) -> Result<()> {
    fs::remove_dir(path)
}

pub fn fs_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}
