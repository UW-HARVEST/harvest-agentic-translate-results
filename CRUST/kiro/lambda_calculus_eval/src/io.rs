use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
pub fn create_file(path: &str) -> io::Result<File> {
    File::create(path)
}
pub fn get_file(path: &str, mode: &str) -> io::Result<File> {
    if mode.contains('w') {
        File::create(path)
    } else if mode.contains('+') {
        OpenOptions::new().read(true).write(true).open(path)
    } else {
        File::open(path)
    }
}
pub fn write_to_file(file: &mut File, content: &str) -> io::Result<()> {
    file.write_all(content.as_bytes())?;
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}
pub fn delete_file(path: &str) -> io::Result<()> {
    std::fs::remove_file(path)
}
pub fn close_file(_file: File) -> io::Result<()> {
    Ok(())
}
pub fn next(file: &mut File) -> io::Result<char> {
    let mut buf = [0u8; 1];
    file.read_exact(&mut buf)?;
    Ok(buf[0] as char)
}
