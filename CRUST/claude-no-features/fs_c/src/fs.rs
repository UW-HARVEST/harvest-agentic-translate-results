use std::fs;
use std::io::{Read, Result, Write};
use std::os::fd::{self, AsFd};

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

fn parse_flags(flags: &str) -> fs::OpenOptions {
    let mut opts = fs::OpenOptions::new();
    // Filter out the 'b' (binary) modifier since it's irrelevant on Unix.
    let cleaned: String = flags.chars().filter(|c| *c != 'b').collect();
    let has_plus = cleaned.contains('+');
    let has_w = cleaned.contains('w');
    let has_r = cleaned.contains('r');
    let has_a = cleaned.contains('a');

    let first = cleaned.chars().next().unwrap_or('r');
    match first {
        'r' => {
            opts.read(true);
            if has_plus || has_w {
                opts.write(true);
            }
        }
        'w' => {
            opts.write(true).create(true).truncate(true);
            if has_plus || has_r {
                opts.read(true);
            }
        }
        'a' => {
            opts.append(true).create(true);
            if has_plus || has_r {
                opts.read(true);
            }
        }
        _ => {
            opts.read(true);
        }
    }
    let _ = has_a;
    opts
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let opts = parse_flags(flags);
    opts.open(path).ok().map(fd::OwnedFd::from)
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
    let cloned = fd.as_fd().try_clone_to_owned()?;
    let file = fs::File::from(cloned);
    file.metadata()
}

pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    let cloned = fd.as_fd().try_clone_to_owned()?;
    let file = fs::File::from(cloned);
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
    std::os::unix::fs::fchown(fd, Some(uid), Some(gid))
}

pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::lchown(path, Some(uid), Some(gid))
}

pub fn fs_size(path: &str) -> Result<u64> {
    fs::metadata(path).map(|m| m.len())
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    fs_fstat(fd).map(|m| m.len())
}

pub fn fs_read(path: &str) -> Result<Vec<u8>> {
    fs::read(path)
}

pub fn fs_nread(path: &str, len: u64) -> Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    let mut buf = Vec::with_capacity(len as usize);
    file.take(len).read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let cloned = fd.as_fd().try_clone_to_owned()?;
    let mut file = fs::File::from(cloned);
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let cloned = fd.as_fd().try_clone_to_owned()?;
    let file = fs::File::from(cloned);
    let mut buf = Vec::with_capacity(len as usize);
    file.take(len).read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs_nwrite(path, data, data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let mut file = fs::OpenOptions::new()
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
    let cloned = fd.as_fd().try_clone_to_owned()?;
    let mut file = fs::File::from(cloned);
    let n = (len as usize).min(data.len());
    file.write_all(&data[..n])?;
    Ok(n as u64)
}

pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    let mut builder = fs::DirBuilder::new();
    builder.mode(mode);
    builder.create(path)
}

pub fn fs_rmdir(path: &str) -> Result<()> {
    fs::remove_dir(path)
}

pub fn fs_exists(path: &str) -> bool {
    fs::symlink_metadata(path).is_ok()
}
