use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};

pub fn create_file(path: &str) -> io::Result<File> {
    // Equivalent to fopen(path, "w") - truncates if exists, creates if missing.
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}

pub fn get_file(path: &str, mode: &str) -> io::Result<File> {
    // Translate the C-style fopen mode string to Rust OpenOptions.
    let mut opts = OpenOptions::new();
    match mode {
        "r" => {
            opts.read(true);
        }
        "r+" => {
            opts.read(true).write(true);
        }
        "w" => {
            opts.write(true).create(true).truncate(true);
        }
        "w+" => {
            opts.read(true).write(true).create(true).truncate(true);
        }
        "a" => {
            opts.append(true).create(true);
        }
        "a+" => {
            opts.read(true).append(true).create(true);
        }
        _ => {
            opts.read(true);
        }
    }
    opts.open(path)
}

pub fn write_to_file(file: &mut File, content: &str) -> io::Result<()> {
    file.write_all(content.as_bytes())?;
    file.flush()?;
    // C version calls rewind(file) after writing.
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}

pub fn delete_file(path: &str) -> io::Result<()> {
    std::fs::remove_file(path)
}

pub fn close_file(_file: File) -> io::Result<()> {
    // Dropping the File closes it.
    Ok(())
}

/// Reads the next character from the file. Returns the special character
/// '\0' if EOF has been reached (matching the C cast of EOF (-1) to char).
pub fn next(file: &mut File) -> io::Result<char> {
    let mut buf = [0u8; 1];
    match file.read(&mut buf) {
        Ok(0) => Ok('\u{FFFF}'), // sentinel: end-of-file
        Ok(_) => Ok(buf[0] as char),
        Err(e) => Err(e),
    }
}
