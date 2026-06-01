use std::fs;

/// Returns the string contents of the file at `path` and sets the file length in `len`.
/// Returns `None` on error.
pub fn file2strl(path: &str, len: &mut u32) -> Option<String> {
    match fs::read(path) {
        Ok(bytes) => {
            let file_len = bytes.len();
            let contents = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("Read error");
                    return None;
                }
            };
            *len = (file_len + 1) as u32;
            Some(contents)
        }
        Err(_) => {
            eprintln!("Unable to open file {}", path);
            None
        }
    }
}
/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut len: u32 = 0;
    file2strl(path, &mut len)
}
