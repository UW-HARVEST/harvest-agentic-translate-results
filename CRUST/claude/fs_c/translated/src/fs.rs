use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Result, Seek, SeekFrom, Write};
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
    eprintln!(
        "fs: {}: error: {}",
        s,
        std::io::Error::last_os_error()
    );
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let mut opts = OpenOptions::new();
    let mut chars = flags.chars();
    let mode = chars.next()?;
    let plus = flags.contains('+');
    match (mode, plus) {
        ('r', false) => {
            opts.read(true);
        }
        ('r', true) => {
            opts.read(true).write(true);
        }
        ('w', false) => {
            opts.write(true).create(true).truncate(true);
        }
        ('w', true) => {
            opts.read(true).write(true).create(true).truncate(true);
        }
        ('a', false) => {
            opts.append(true).create(true);
        }
        ('a', true) => {
            opts.read(true).append(true).create(true);
        }
        _ => return None,
    }
    let file = opts.open(path).ok()?;
    Some(fd::OwnedFd::from(file))
}

pub fn fs_close(fd: fd::OwnedFd) -> Result<()> {
    // Dropping the OwnedFd closes it.
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
        Err(std::io::Error::from_raw_os_error(38)) // ENOSYS
    }
}

pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::fchown(fd, Some(uid), Some(gid))
    }
    #[cfg(not(unix))]
    {
        let _ = (fd, uid, gid);
        Err(std::io::Error::from_raw_os_error(38)) // ENOSYS
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
        Err(std::io::Error::from_raw_os_error(38)) // ENOSYS
    }
}

pub fn fs_size(path: &str) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    // Save current position, seek to end to get size, then restore position.
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
    let mut buffer = Vec::with_capacity(len as usize);
    let mut handle = (&mut file).take(len);
    handle.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let size = fs_fsize(fd)?;
    fs_fnread(fd, size)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    let mut buffer = Vec::with_capacity(len as usize);
    let mut handle = (&mut file).take(len);
    handle.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs_nwrite(path, data, data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let n = (len as usize).min(data.len());
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(&data[..n])?;
    Ok(n as u64)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    fs_fnwrite(fd, data, data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = File::from(cloned);
    let n = (len as usize).min(data.len());
    file.write_all(&data[..n])?;
    Ok(n as u64)
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
