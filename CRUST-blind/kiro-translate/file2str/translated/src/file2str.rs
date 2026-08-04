/// Returns the string contents of the file at `path` and sets the file length in `len`.
/// Returns `None` on error.
pub fn file2strl(path: &str, len: &mut u32) -> Option<String> {
    let contents = std::fs::read(path).ok()?;
    // C version sets len to file_len + 1 (includes null terminator)
    *len = (contents.len() + 1) as u32;
    Some(String::from_utf8_lossy(&contents).into_owned())
}
/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let contents = std::fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&contents).into_owned())
}