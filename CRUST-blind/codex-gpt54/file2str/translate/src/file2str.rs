/// Returns the string contents of the file at `path` and sets the file length in `len`.
/// Returns `None` on error.
pub fn file2strl(path: &str, len: &mut u32) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    *len = bytes.len().wrapping_add(1) as u32;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}
/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut ignored_len = 0;
    file2strl(path, &mut ignored_len)
}
