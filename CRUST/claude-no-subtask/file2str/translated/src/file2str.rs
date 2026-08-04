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

    let mut contents = Vec::new();
    if let Err(_) = file.read_to_end(&mut contents) {
        eprintln!("Read error");
        return None;
    }

    let file_len = contents.len() as u32;

    let s = match String::from_utf8(contents) {
        Ok(s) => s,
        Err(e) => {
            // Fallback: lossy convert if not valid UTF-8
            String::from_utf8_lossy(&e.into_bytes()).into_owned()
        }
    };

    *len = file_len + 1;

    Some(s)
}

/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut dummy: u32 = 0;
    file2strl(path, &mut dummy)
}
