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
    let mut contents = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => {
            // Fall back to lossy conversion to preserve content as best we can.
            String::from_utf8_lossy(&e.into_bytes()).into_owned()
        }
    };

    // Mirror the C behavior of appending a NUL terminator and reporting
    // the length including that terminator.
    contents.push('\0');
    *len = (file_len + 1) as u32;

    Some(contents)
}
/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut len: u32 = 0;
    file2strl(path, &mut len)
}
