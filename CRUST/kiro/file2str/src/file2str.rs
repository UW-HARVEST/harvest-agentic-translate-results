/// Returns the string contents of the file at `path` and sets the file length in `len`.
/// Returns `None` on error.
pub fn file2strl(path: &str, len: &mut u32) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    *len = contents.len() as u32 + 1;
    Some(contents)
}
/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}