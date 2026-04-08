use std::fs;

/// Returns the string contents of the file at `path` and sets the file length in `len`.
/// Returns `None` on error.
pub fn file2strl(path: &str, len: &mut u32) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    *len = (contents.len() + 1) as u32;
    Some(contents)
}

/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut len = 0u32;
    file2strl(path, &mut len)
}
