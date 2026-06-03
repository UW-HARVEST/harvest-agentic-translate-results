use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
pub fn create_file(path: &str) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}
pub fn get_file(path: &str, mode: &str) -> io::Result<File> {
    match mode {
        "r" => OpenOptions::new().read(true).open(path),
        "r+" => OpenOptions::new().read(true).write(true).open(path),
        "w" => OpenOptions::new().write(true).create(true).truncate(true).open(path),
        "w+" => OpenOptions::new().read(true).write(true).create(true).truncate(true).open(path),
        "a" => OpenOptions::new().write(true).create(true).append(true).open(path),
        "a+" => OpenOptions::new().read(true).write(true).create(true).append(true).open(path),
        _ => OpenOptions::new().read(true).open(path),
    }
}
pub fn write_to_file(file: &mut File, content: &str) -> io::Result<()> {
    file.write_all(content.as_bytes())?;
    file.flush()?;
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}
pub fn delete_file(path: &str) -> io::Result<()> {
    std::fs::remove_file(path)
}
pub fn close_file(_file: File) -> io::Result<()> {
    // dropping `_file` closes it.
    Ok(())
}
pub fn next(file: &mut File) -> io::Result<char> {
    let mut buf = [0u8; 1];
    let n = file.read(&mut buf)?;
    if n == 0 {
        // EOF: return char with value 0xFF (similar to (char)EOF in C with unsigned char)
        // Use null character as marker for EOF in our Rust port
        Ok('\u{FFFF}')
    } else {
        Ok(buf[0] as char)
    }
}
