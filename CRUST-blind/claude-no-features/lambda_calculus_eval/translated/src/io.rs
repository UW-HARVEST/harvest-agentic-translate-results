use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};

pub fn create_file(path: &str) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .truncate(true)
        .open(path)
}

pub fn get_file(path: &str, mode: &str) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    match mode {
        "r" => {
            opts.read(true);
        }
        "w" => {
            opts.create(true).write(true).truncate(true);
        }
        "a" => {
            opts.create(true).append(true);
        }
        "r+" => {
            opts.read(true).write(true);
        }
        "w+" => {
            opts.create(true).read(true).write(true).truncate(true);
        }
        "a+" => {
            opts.create(true).read(true).append(true);
        }
        _ => {
            opts.read(true);
        }
    }
    opts.open(path)
}

pub fn write_to_file(file: &mut File, content: &str) -> io::Result<()> {
    file.write_all(content.as_bytes())?;
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}

pub fn delete_file(path: &str) -> io::Result<()> {
    std::fs::remove_file(path)
}

pub fn close_file(file: File) -> io::Result<()> {
    drop(file);
    Ok(())
}

pub fn next(file: &mut File) -> io::Result<char> {
    let mut buf = [0u8; 1];
    match file.read(&mut buf)? {
        0 => Ok('\u{FFFF}'), // EOF sentinel
        _ => Ok(buf[0] as char),
    }
}
