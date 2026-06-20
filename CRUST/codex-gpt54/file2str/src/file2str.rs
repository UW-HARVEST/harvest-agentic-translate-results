/// Returns the string contents of the file at `path` and sets the file length in `len`.
/// Returns `None` on error.
pub fn file2strl(path: &str, len: &mut u32) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    *len = bytes.len().checked_add(1)?.try_into().ok()?;

    Some(match String::from_utf8(bytes) {
        Ok(contents) => contents,
        Err(err) => String::from_utf8_lossy(&err.into_bytes()).into_owned(),
    })
}
/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut len = 0;
    file2strl(path, &mut len)
}
