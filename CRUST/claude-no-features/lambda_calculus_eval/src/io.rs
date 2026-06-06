use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};

pub fn create_file(path: &str) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .read(true)
        .open(path)
}

pub fn get_file(path: &str, mode: &str) -> io::Result<File> {
    match mode {
        "r" => OpenOptions::new().read(true).open(path),
        "w" => OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path),
        "a" => OpenOptions::new().append(true).create(true).open(path),
        "r+" => OpenOptions::new().read(true).write(true).open(path),
        "w+" => OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path),
        "a+" => OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path),
        _ => OpenOptions::new().read(true).open(path),
    }
}

pub fn write_to_file(file: &mut File, content: &str) -> io::Result<()> {
    file.write_all(content.as_bytes())?;
    file.flush()?;
    use std::io::Seek;
    file.seek(io::SeekFrom::Start(0))?;
    Ok(())
}

pub fn delete_file(path: &str) -> io::Result<()> {
    std::fs::remove_file(path)
}

pub fn close_file(_file: File) -> io::Result<()> {
    // File is closed when dropped
    Ok(())
}

pub fn next(file: &mut File) -> io::Result<char> {
    let mut buf = [0u8; 1];
    match file.read(&mut buf)? {
        0 => Ok('\u{FFFF}'), // EOF
        _ => Ok(buf[0] as char),
    }
}
