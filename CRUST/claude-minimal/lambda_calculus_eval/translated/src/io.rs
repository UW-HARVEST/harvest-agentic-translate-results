use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};

pub fn create_file(path: &str) -> io::Result<File> {
    File::create(path)
}

pub fn get_file(path: &str, mode: &str) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    match mode {
        "r" => {
            opts.read(true);
        }
        "w" => {
            opts.write(true).create(true).truncate(true);
        }
        "a" => {
            opts.append(true).create(true);
        }
        "r+" => {
            opts.read(true).write(true);
        }
        "w+" => {
            opts.read(true).write(true).create(true).truncate(true);
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
    match file.read(&mut buf) {
        Ok(0) => Ok('\u{FFFF}'),
        Ok(_) => Ok(buf[0] as char),
        Err(e) => Err(e),
    }
}
