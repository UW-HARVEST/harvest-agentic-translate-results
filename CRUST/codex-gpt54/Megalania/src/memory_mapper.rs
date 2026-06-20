use std::io;
use std::fs;
use std::path::Path;
pub fn map_file<P: AsRef<Path>>(filename: P) -> io::Result<Vec<u8>> {
    fs::read(filename)
}
pub fn map_anonymous(data_size: usize) -> io::Result<Vec<u8>> {
    Ok(vec![0; data_size])
}
pub fn unmap(_data: Vec<u8>) -> io::Result<()> {
    Ok(())
}
