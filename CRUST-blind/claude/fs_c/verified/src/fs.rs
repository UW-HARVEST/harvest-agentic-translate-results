use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
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

/// Helper: borrow an `OwnedFd` as a `File` without taking ownership of the
/// underlying file descriptor. We `try_clone` (which `dup`s the fd) so that
/// dropping the `File` does not close the caller's fd.
fn file_from_borrowed_fd(fd: &fd::OwnedFd) -> Result<fs::File> {
    let cloned = fd.try_clone()?;
    Ok(fs::File::from(cloned))
}

pub fn fs_error(s: &str) {
    eprintln!("fs: {}: error: {}", s, std::io::Error::last_os_error());
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let read = flags.contains('r');
    let write = flags.contains('w');
    if !read && !write {
        return None;
    }
    let mut opts = fs::OpenOptions::new();
    opts.read(read).write(write);
    if write && !read {
        // "w" semantics: truncate to zero length / create file for writing
        opts.create(true).truncate(true);
    }
    match opts.open(path) {
        Ok(file) => Some(fd::OwnedFd::from(file)),
        Err(_) => None,
    }
}

pub fn fs_close(fd: fd::OwnedFd) -> Result<()> {
    // Dropping an OwnedFd closes the underlying file descriptor.
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
    let file = file_from_borrowed_fd(fd)?;
    file.metadata()
}

pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    let file = file_from_borrowed_fd(fd)?;
    file.set_len(length)
}

pub fn fs_truncate(path: &str, length: u64) -> Result<()> {
    let file = fs::OpenOptions::new().write(true).open(path)?;
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
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "chown not supported on this platform",
        ))
    }
}

pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        let file = file_from_borrowed_fd(fd)?;
        std::os::unix::fs::fchown(&file, Some(uid), Some(gid))
    }
    #[cfg(not(unix))]
    {
        let _ = (fd, uid, gid);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "fchown not supported on this platform",
        ))
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
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "lchown not supported on this platform",
        ))
    }
}

pub fn fs_size(path: &str) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    // Mirror the C version which preserves the original cursor position
    // by seeking to the end, recording the position, then seeking back.
    let mut file = file_from_borrowed_fd(fd)?;
    let pos = file.stream_position()?;
    let size = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(pos))?;
    Ok(size)
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
    let mut file = file_from_borrowed_fd(fd)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let mut file = file_from_borrowed_fd(fd)?;
    let mut buffer = vec![0u8; len as usize];
    let n = file.read(&mut buffer)?;
    buffer.truncate(n);
    Ok(buffer)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs_nwrite(path, data, data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let mut file = fs::File::create(path)?;
    let len_usize = (len as usize).min(data.len());
    let n = file.write(&data[..len_usize])?;
    Ok(n as u64)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    fs_fnwrite(fd, data, data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let mut file = file_from_borrowed_fd(fd)?;
    let len_usize = (len as usize).min(data.len());
    let n = file.write(&data[..len_usize])?;
    Ok(n as u64)
}

pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder.mode(mode);
        builder.create(path)
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
    std::path::Path::new(path).exists()
}
