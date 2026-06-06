use std::fs;
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

/// Convert a borrowed `OwnedFd` reference into a `File`, by duplicating the
/// underlying file descriptor. The returned `File` shares the kernel file
/// table entry with the original, so seek/read/write operations will behave
/// the same as if we had used the original descriptor directly.
fn fd_to_file(fd: &fd::OwnedFd) -> Result<fs::File> {
    let cloned = fd.try_clone()?;
    Ok(fs::File::from(cloned))
}

pub fn fs_error(s: &str) {
    let err = std::io::Error::last_os_error();
    eprintln!("fs: {}: error: {}", s, err);
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    // Mimic fopen-style mode parsing. The first character determines the
    // base mode, and a `+` modifier upgrades to read+write. The `b` (binary)
    // modifier is a no-op on POSIX and we treat it the same here.
    let first = flags.chars().next()?;
    let plus = flags.contains('+');
    let mut opts = fs::OpenOptions::new();
    match first {
        'r' => {
            opts.read(true);
            if plus {
                opts.write(true);
            }
        }
        'w' => {
            opts.write(true).create(true).truncate(true);
            if plus {
                opts.read(true);
            }
        }
        'a' => {
            opts.append(true).create(true);
            if plus {
                opts.read(true);
            }
        }
        _ => return None,
    }
    let file = opts.open(path).ok()?;
    Some(fd::OwnedFd::from(file))
}

pub fn fs_close(fd: fd::OwnedFd) -> Result<()> {
    // Dropping the OwnedFd closes the underlying descriptor.
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
    let file = fd_to_file(fd)?;
    file.metadata()
}

pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    let file = fd_to_file(fd)?;
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
            "chown is not supported on this platform",
        ))
    }
}

pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        std::os::unix::fs::fchown(fd.as_fd(), Some(uid), Some(gid))
    }
    #[cfg(not(unix))]
    {
        let _ = (fd, uid, gid);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "fchown is not supported on this platform",
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
            "lchown is not supported on this platform",
        ))
    }
}

pub fn fs_size(path: &str) -> Result<u64> {
    fs::metadata(path).map(|m| m.len())
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    // Mirror the C implementation: preserve the current position, seek to the
    // end to discover the size, then restore the original position.
    let mut file = fd_to_file(fd)?;
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
    read_n(&mut file, len)
}

pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let size = fs_fsize(fd)?;
    fs_fnread(fd, size)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let mut file = fd_to_file(fd)?;
    read_n(&mut file, len)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    let mut file = fs::File::create(path)?;
    file.write_all(data)?;
    Ok(data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let mut file = fs::File::create(path)?;
    write_n(&mut file, data, len)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    let mut file = fd_to_file(fd)?;
    file.write_all(data)?;
    Ok(data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let mut file = fd_to_file(fd)?;
    write_n(&mut file, data, len)
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

/// Helper: read up to `len` bytes from `file` starting at the current position.
/// Returns whatever was actually read (which may be fewer than `len` bytes
/// at end-of-file). This mirrors `fread`'s semantics from the C version.
fn read_n<R: Read>(file: &mut R, len: u64) -> Result<Vec<u8>> {
    let cap = len as usize;
    let mut buf = vec![0u8; cap];
    let mut total = 0usize;
    while total < cap {
        match file.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    buf.truncate(total);
    Ok(buf)
}

/// Helper: write up to `len` bytes from `data` to `file`. If `data` is shorter
/// than `len`, only `data.len()` bytes will be written. Returns the number of
/// bytes actually written.
fn write_n<W: Write>(file: &mut W, data: &[u8], len: u64) -> Result<u64> {
    let to_write = std::cmp::min(len as usize, data.len());
    file.write_all(&data[..to_write])?;
    Ok(to_write as u64)
}
