use std::fs;
use std::io;
use std::path::Path;

/// Reads the entire contents of a file as a byte vector. The C version
/// memory-maps the file with `mmap`; for safety and idiomatic-ness we
/// read the file into a `Vec<u8>` instead.
pub fn map_file<P: AsRef<Path>>(filename: P) -> io::Result<Vec<u8>> {
    fs::read(filename)
}

/// Allocates a zero-initialized buffer of the requested size. The C
/// version uses `mmap` with `MAP_ANONYMOUS`; for safety we use a `Vec<u8>`.
pub fn map_anonymous(data_size: usize) -> io::Result<Vec<u8>> {
    Ok(vec![0u8; data_size])
}

/// Frees a previously-allocated buffer. In Rust this is a no-op since
/// the `Vec<u8>` is dropped when the function returns.
pub fn unmap(_data: Vec<u8>) -> io::Result<()> {
    Ok(())
}
