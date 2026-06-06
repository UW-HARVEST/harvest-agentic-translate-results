use std::fs;
use std::fs::OpenOptions;
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
    eprintln!("fs: {}: error", s);
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let has_r = flags.contains('r');
    let has_w = flags.contains('w');
    let has_a = flags.contains('a');
    let has_plus = flags.contains('+');

    let mut opts = OpenOptions::new();
    if has_a {
        opts.append(true).create(true);
        if has_plus || has_r {
            opts.read(true);
        }
    } else if has_w && has_r {
        // "rw" or similar: open for both read and write, create if missing
        opts.read(true).write(true).create(true);
    } else if has_w {
        opts.write(true).create(true).truncate(true);
        if has_plus {
            opts.read(true);
        }
    } else if has_r {
        opts.read(true);
        if has_plus {
            opts.write(true);
        }
    } else {
        // default: read
        opts.read(true);
    }

    match opts.open(path) {
        Ok(f) => Some(fd::OwnedFd::from(f)),
        Err(_) => None,
    }
}

pub fn fs_close(fd: fd::OwnedFd) -> Result<()> {
    // Dropping the OwnedFd closes the underlying file descriptor.
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
    let file = fs::File::from(cloned);
    file.metadata()
}

pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    let cloned = fd.try_clone()?;
    let file = fs::File::from(cloned);
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
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "chown not supported on this platform",
        ))
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
    let m = fs::metadata(path)?;
    Ok(m.len())
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = fs::File::from(cloned);
    // Save current position, seek to end to get size, then restore.
    let pos = file.stream_position()?;
    let size = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(pos))?;
    Ok(size)
}

pub fn fs_read(path: &str) -> Result<Vec<u8>> {
    fs::read(path)
}

pub fn fs_nread(path: &str, len: u64) -> Result<Vec<u8>> {
    let file = OpenOptions::new().read(true).open(path)?;
    let mut handle = file.take(len);
    let mut buf: Vec<u8> = Vec::with_capacity(len as usize);
    handle.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let mut file = fs::File::from(cloned);
    let mut buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let file = fs::File::from(cloned);
    let mut handle = file.take(len);
    let mut buf: Vec<u8> = Vec::with_capacity(len as usize);
    handle.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs_nwrite(path, data, data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    let n = (len as usize).min(data.len());
    file.write_all(&data[..n])?;
    Ok(n as u64)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    fs_fnwrite(fd, data, data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = fs::File::from(cloned);
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
        fs::DirBuilder::new().create(path)
    }
}

pub fn fs_rmdir(path: &str) -> Result<()> {
    fs::remove_dir(path)
}

pub fn fs_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}
