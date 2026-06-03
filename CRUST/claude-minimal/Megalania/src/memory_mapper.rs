use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Read a file and return its contents as a Vec<u8>. The C version mmaps the
/// file; this function reads the file into a heap-allocated buffer, which
/// provides equivalent semantics for read-only access in safe Rust.
pub fn map_file<P: AsRef<Path>>(filename: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(filename)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    Ok(data)
}

/// Allocate a zeroed buffer of the requested size. The C version mmaps
/// anonymous memory; the Rust equivalent is a zero-initialized Vec.
pub fn map_anonymous(data_size: usize) -> io::Result<Vec<u8>> {
    Ok(vec![0u8; data_size])
}

/// Drops the buffer. Provided for API symmetry with the C version's
/// `unmap`; in Rust, dropping the Vec frees its memory.
pub fn unmap(_data: Vec<u8>) -> io::Result<()> {
    Ok(())
}
