/// Returns the string contents of the file at `path` and sets the file length in `len`.
/// Returns `None` on error.
pub fn file2strl(path: &str, len: &mut u32) -> Option<String> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Unable to open file {}", path);
            return None;
        }
    };

    let file_len = bytes.len();
    let contents = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => {
            // Fall back to lossy conversion to mimic C's byte-oriented behavior.
            String::from_utf8_lossy(&e.into_bytes()).into_owned()
        }
    };

    *len = (file_len as u32).wrapping_add(1);
    Some(contents)
}

/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut _len: u32 = 0;
    file2strl(path, &mut _len)
}
