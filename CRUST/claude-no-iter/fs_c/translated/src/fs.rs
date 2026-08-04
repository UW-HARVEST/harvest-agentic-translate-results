use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Result, Write};
use std::os::fd;
use std::os::fd::OwnedFd;
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

/// Prints the last error to stderr with a given prefix.
pub fn fs_error(s: &str) {
    eprintln!("fs: {}: error: {}", s, std::io::Error::last_os_error());
}

/// Opens a file with the given fopen-like flags and returns an OwnedFd.
pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let mut options = OpenOptions::new();
    match flags {
        "r" | "rb" => {
            options.read(true);
        }
        "w" | "wb" => {
            options.write(true).create(true).truncate(true);
        }
        "rw" | "rwb" => {
            options.read(true).write(true).create(true);
        }
        _ => return None,
    };
    options.open(path).ok().map(OwnedFd::from)
}

/// Closes the given file descriptor (consumes the OwnedFd).
pub fn fs_close(fd: fd::OwnedFd) -> Result<()> {
    drop(fd);
    Ok(())
}

/// Renames a file from one path to another.
pub fn fs_rename(from: &str, to: &str) -> Result<()> {
    fs::rename(from, to)
}

/// Returns metadata for a path (follows symlinks).
pub fn fs_stat(path: &str) -> Result<fs::Metadata> {
    fs::metadata(path)
}

/// Returns metadata for a file descriptor.
pub fn fs_fstat(fd: &fd::OwnedFd) -> Result<fs::Metadata> {
    let cloned = fd.try_clone()?;
    let file: File = cloned.into();
    file.metadata()
}

/// Returns metadata for a path without following symlinks.
pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

/// Truncates a file by file descriptor to the given length.
pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    let cloned = fd.try_clone()?;
    let file: File = cloned.into();
    file.set_len(length)
}

/// Truncates a file by path to the given length.
pub fn fs_truncate(path: &str, length: u64) -> Result<()> {
    let file = OpenOptions::new().write(true).open(path)?;
    file.set_len(length)
}

/// Changes ownership of a file at the given path.
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
            "fs_chown not supported on this platform",
        ))
    }
}

/// Changes ownership of a file by file descriptor.
pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::fchown(fd, Some(uid), Some(gid))
    }
    #[cfg(not(unix))]
    {
        let _ = (fd, uid, gid);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "fs_fchown not supported on this platform",
        ))
    }
}

/// Changes ownership of a symbolic link without following it.
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
            "fs_lchown not supported on this platform",
        ))
    }
}

/// Returns the size of a file at the given path.
pub fn fs_size(path: &str) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}

/// Returns the size of a file given a file descriptor.
pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let file: File = cloned.into();
    Ok(file.metadata()?.len())
}

/// Reads the entire contents of a file at the given path.
pub fn fs_read(path: &str) -> Result<Vec<u8>> {
    fs::read(path)
}

/// Reads up to `len` bytes from the file at the given path.
pub fn fs_nread(path: &str, len: u64) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut buf = Vec::with_capacity(len as usize);
    file.take(len).read_to_end(&mut buf)?;
    Ok(buf)
}

/// Reads the entire contents of a file given a file descriptor.
pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let mut file: File = cloned.into();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Reads up to `len` bytes from a file descriptor.
pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let file: File = cloned.into();
    let mut buf = Vec::with_capacity(len as usize);
    file.take(len).read_to_end(&mut buf)?;
    Ok(buf)
}

/// Writes the given buffer to a file at the given path.
pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs::write(path, data)?;
    Ok(data.len() as u64)
}

/// Writes the first `len` bytes of `data` to a file at the given path.
pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let n = std::cmp::min(len as usize, data.len());
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(&data[..n])?;
    Ok(n as u64)
}

/// Writes the buffer to the given file descriptor.
pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file: File = cloned.into();
    file.write_all(data)?;
    Ok(data.len() as u64)
}

/// Writes the first `len` bytes of `data` to the given file descriptor.
pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let n = std::cmp::min(len as usize, data.len());
    let cloned = fd.try_clone()?;
    let mut file: File = cloned.into();
    file.write_all(&data[..n])?;
    Ok(n as u64)
}

/// Creates a directory at the given path with the specified mode.
pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        fs::DirBuilder::new().mode(mode).create(path)
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
        fs::create_dir(path)
    }
}

/// Removes the directory at the given path.
pub fn fs_rmdir(path: &str) -> Result<()> {
    fs::remove_dir(path)
}

/// Returns true if a path exists.
pub fn fs_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}
