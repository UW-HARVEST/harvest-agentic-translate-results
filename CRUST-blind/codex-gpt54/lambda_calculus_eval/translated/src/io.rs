use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use crate::common;
pub fn create_file(path: &str) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .read(true)
        .open(path)
}
pub fn get_file(path: &str, mode: &str) -> io::Result<File> {
    let mut options = OpenOptions::new();
    match mode {
        "r" => {
            options.read(true);
        }
        "r+" => {
            options.read(true).write(true);
        }
        "w" => {
            options.write(true).create(true).truncate(true);
        }
        "w+" => {
            options.read(true).write(true).create(true).truncate(true);
        }
        "a" => {
            options.append(true).create(true);
        }
        "a+" => {
            options.read(true).append(true).create(true);
        }
        _ => {
            common::error("ERROR: unsupported file mode.", file!(), line!() as i32, "get_file");
        }
    }
    options.open(path)
}
pub fn write_to_file(file: &mut File, content: &str) -> io::Result<()> {
    file.write_all(content.as_bytes())?;
    file.flush()?;
    file.rewind()
}
pub fn delete_file(path: &str) -> io::Result<()> {
    std::fs::remove_file(path)
}
pub fn close_file(file: File) -> io::Result<()> {
    drop(file);
    Ok(())
}
pub fn next(file: &mut File) -> io::Result<char> {
    let mut buf = [0_u8; 1];
    match file.read(&mut buf)? {
        0 => Ok('\0'),
        _ => Ok(buf[0] as char),
    }
}
