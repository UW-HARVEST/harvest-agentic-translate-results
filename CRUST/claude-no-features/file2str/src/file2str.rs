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

    let mut contents = String::new();
    if file.read_to_string(&mut contents).is_err() {
        eprintln!("Read error");
        return None;
    }

    // Mirror the C behavior: the C code allocates file_len + 1 bytes
    // (for a trailing '\0') and sets *file_len_out = file_len + 1.
    *len = (contents.len() as u32) + 1;

    Some(contents)
}

/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut dummy: u32 = 0;
    file2strl(path, &mut dummy)
}
