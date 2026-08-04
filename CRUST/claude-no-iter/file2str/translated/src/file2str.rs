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
    match file.read_to_string(&mut contents) {
        Ok(_) => {}
        Err(_) => {
            eprintln!("Read error");
            return None;
        }
    }

    *len = (contents.len() as u32) + 1;
    Some(contents)
}
/// Returns the string contents of the file at `path`, or `None` on error.
pub fn file2str(path: &str) -> Option<String> {
    let mut len: u32 = 0;
    file2strl(path, &mut len)
}
