use std::fs::File;
use std::io::Read;

/// Returns the string contents of the file at `path` and sets the file length in `len`.
/// Returns `None` on error.
pub fn file2strl(path: &str, len: &mut u32) -> Option<String> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Unable to open file {}", path);
            return None;
        }
    };

    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        eprintln!("Read error");
        return None;
    }

    let file_len = bytes.len() as u32;
    let contents = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return None,
    };

    *len = file_len + 1;

    Some(contents)
}

/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut _len: u32 = 0;
    file2strl(path, &mut _len)
}
