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

pub fn fs_error(s: &str) {
    eprintln!("fs: {}: error", s);
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    // Mimic fopen: only the leading mode chars matter ("r", "w", "a"),
    // a "+" anywhere means read+write.
    let has_plus = flags.contains('+');
    let mode = flags.chars().next()?;

    let mut opts = fs::OpenOptions::new();
    match mode {
        'r' => {
            opts.read(true);
            if has_plus {
                opts.write(true);
            }
        }
        'w' => {
            opts.write(true).create(true).truncate(true);
            if has_plus {
                opts.read(true);
            }
        }
        'a' => {
            opts.append(true).create(true);
            if has_plus {
                opts.read(true);
            }
        }
        _ => return None,
    }

    opts.open(path).ok().map(fd::OwnedFd::from)
}

pub fn fs_close(fd: fd::OwnedFd) -> Result<()> {
    // Dropping OwnedFd closes the underlying file descriptor.
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
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_len(length)
}

#[cfg(unix)]
pub fn fs_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::chown(path, Some(uid), Some(gid))
}

#[cfg(not(unix))]
pub fn fs_chown(_path: &str, _uid: u32, _gid: u32) -> Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "chown is not supported on this platform",
    ))
}

#[cfg(unix)]
pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::fchown(fd, Some(uid), Some(gid))
}

#[cfg(not(unix))]
pub fn fs_fchown(_fd: &fd::OwnedFd, _uid: u32, _gid: u32) -> Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "fchown is not supported on this platform",
    ))
}

#[cfg(unix)]
pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::lchown(path, Some(uid), Some(gid))
}

#[cfg(not(unix))]
pub fn fs_lchown(_path: &str, _uid: u32, _gid: u32) -> Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "lchown is not supported on this platform",
    ))
}

pub fn fs_size(path: &str) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = fs::File::from(cloned);
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
    read_n(&mut file, len)
}

pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let mut file = fs::File::from(cloned);
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let cloned = fd.try_clone()?;
    let mut file = fs::File::from(cloned);
    read_n(&mut file, len)
}

fn read_n<R: Read>(reader: &mut R, len: u64) -> Result<Vec<u8>> {
    let cap = len as usize;
    let mut buf = vec![0u8; cap];
    let mut total = 0;
    while total < cap {
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
    let mut file = fs::File::create(path)?;
    write_n(&mut file, data, len)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    fs_fnwrite(fd, data, data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let cloned = fd.try_clone()?;
    let mut file = fs::File::from(cloned);
    write_n(&mut file, data, len)
}

fn write_n<W: Write>(writer: &mut W, data: &[u8], len: u64) -> Result<u64> {
    let to_write = std::cmp::min(len as usize, data.len());
    writer.write_all(&data[..to_write])?;
    Ok(to_write as u64)
}

#[cfg(unix)]
pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
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
    std::path::Path::new(path).exists()
}
