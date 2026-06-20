use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Result, Seek, SeekFrom, Write};
use std::os::fd;
#[cfg(unix)]
use std::os::unix::fs::{chown, fchown, lchown, DirBuilderExt};
use std::path::Path;

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

fn clone_file(fd: &fd::OwnedFd) -> Result<File> {
    Ok(File::from(fd.try_clone()?))
}

fn u64_to_usize(len: u64) -> Result<usize> {
    usize::try_from(len)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "length exceeds usize"))
}

pub fn fs_error(s: &str) {
    eprintln!("fs: {s}: error: {}", io::Error::last_os_error());
}

pub fn fs_open(path: &str, flags: &str) -> Option<fd::OwnedFd> {
    let file = match flags {
        FS_OPEN_READ => OpenOptions::new().read(true).open(path),
        FS_OPEN_WRITE => OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path),
        FS_OPEN_READWRITE => OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path),
        _ => return None,
    };

    file.ok().map(Into::into)
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
    clone_file(fd)?.metadata()
}

pub fn fs_lstat(path: &str) -> Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

pub fn fs_ftruncate(fd: &fd::OwnedFd, length: u64) -> Result<()> {
    clone_file(fd)?.set_len(length)
}

pub fn fs_truncate(path: &str, length: u64) -> Result<()> {
    #[cfg(target_os = "windows")]
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)?;

    #[cfg(not(target_os = "windows"))]
    let file = OpenOptions::new().write(true).open(path)?;

    file.set_len(length)
}

pub fn fs_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        chown(path, Some(uid), Some(gid))
    }

    #[cfg(not(unix))]
    {
        let _ = (path, uid, gid);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "chown is not supported on this platform",
        ))
    }
}

pub fn fs_fchown(fd: &fd::OwnedFd, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        fchown(fd, Some(uid), Some(gid))
    }

    #[cfg(not(unix))]
    {
        let _ = (fd, uid, gid);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "fchown is not supported on this platform",
        ))
    }
}

pub fn fs_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        lchown(path, Some(uid), Some(gid))
    }

    #[cfg(not(unix))]
    {
        let _ = (path, uid, gid);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "lchown is not supported on this platform",
        ))
    }
}

pub fn fs_size(path: &str) -> Result<u64> {
    Ok(fs::metadata(path)?.len())
}

pub fn fs_fsize(fd: &fd::OwnedFd) -> Result<u64> {
    let mut file = clone_file(fd)?;
    let pos = file.stream_position()?;
    let size = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(pos))?;
    Ok(size)
}

pub fn fs_read(path: &str) -> Result<Vec<u8>> {
    fs::read(path)
}

pub fn fs_nread(path: &str, len: u64) -> Result<Vec<u8>> {
    let fd = fs_open(path, FS_OPEN_READ)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "failed to open file"))?;
    fs_fnread(&fd, len)
}

pub fn fs_fread(fd: &fd::OwnedFd) -> Result<Vec<u8>> {
    let len = fs_fsize(fd)?;
    fs_fnread(fd, len)
}

pub fn fs_fnread(fd: &fd::OwnedFd, len: u64) -> Result<Vec<u8>> {
    let mut file = clone_file(fd)?;
    let mut buffer = vec![0; u64_to_usize(len)?];
    let n = file.read(&mut buffer)?;
    buffer.truncate(n);
    Ok(buffer)
}

pub fn fs_write(path: &str, data: &[u8]) -> Result<u64> {
    fs_nwrite(path, data, data.len() as u64)
}

pub fn fs_nwrite(path: &str, data: &[u8], len: u64) -> Result<u64> {
    let fd = fs_open(path, FS_OPEN_WRITE)
        .ok_or_else(|| io::Error::new(io::ErrorKind::PermissionDenied, "failed to open file"))?;
    fs_fnwrite(&fd, data, len)
}

pub fn fs_fwrite(fd: &fd::OwnedFd, data: &[u8]) -> Result<u64> {
    fs_fnwrite(fd, data, data.len() as u64)
}

pub fn fs_fnwrite(fd: &fd::OwnedFd, data: &[u8], len: u64) -> Result<u64> {
    let mut file = clone_file(fd)?;
    let len = u64_to_usize(len)?.min(data.len());
    file.write(&data[..len]).map(|n| n as u64)
}

pub fn fs_mkdir(path: &str, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        let mut builder = DirBuilder::new();
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
    fs::metadata(Path::new(path)).is_ok()
}
