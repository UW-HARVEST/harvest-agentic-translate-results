use std::fs;
use std::io;
use std::path::Path;

/// Reads the entire contents of a file into a `Vec<u8>`. The C version uses
/// `mmap`; here we keep the data owned for simplicity and portability.
pub fn map_file<P: AsRef<Path>>(filename: P) -> io::Result<Vec<u8>> {
    fs::read(filename)
}

/// Allocates a zero-filled buffer of the requested size. The C version uses
/// `mmap` with `MAP_ANONYMOUS`; in Rust a `Vec<u8>` provides equivalent
/// functionality.
pub fn map_anonymous(data_size: usize) -> io::Result<Vec<u8>> {
    Ok(vec![0u8; data_size])
}

/// Releases the memory backing the given buffer.
pub fn unmap(_data: Vec<u8>) -> io::Result<()> {
    // Dropping the Vec frees its memory.
    Ok(())
}
