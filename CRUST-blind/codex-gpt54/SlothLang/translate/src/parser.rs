use crate::slothvm::SlothProgram;
use std::collections::HashMap;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::FileExt;
use std::sync::{Mutex, OnceLock};

fn line_offsets() -> &'static Mutex<HashMap<i32, usize>> {
    static OFFSETS: OnceLock<Mutex<HashMap<i32, usize>>> = OnceLock::new();
    OFFSETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn read_file_bytes(file: &File) -> Vec<u8> {
    let len = match file.metadata() {
        Ok(metadata) => metadata.len() as usize,
        Err(_) => return Vec::new(),
    };

    let mut buf = vec![0_u8; len];
    let mut read = 0usize;
    while read < len {
        match file.read_at(&mut buf[read..], read as u64) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => return Vec::new(),
        }
    }
    buf.truncate(read);
    buf
}

pub fn parse(filename: &str) -> Option<SlothProgram> {
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            eprint!("[ERROR] File could not be opened.");
            return None;
        }
    };

    let _ = prog_len(&file);

    let mut byte_code = Vec::new();
    while let Some(line) = readline(&file) {
        if line.is_empty() {
            continue;
        }

        let mut count = 0usize;
        let bytes = line.as_bytes();
        let mut current_code = 0_u8;
        let mut had_token = false;

        while count < bytes.len() {
            if bytes[count] == b'#' {
                break;
            }

            let rest = &bytes[count..];
            if rest.starts_with(b"slothy") {
                byte_code.push(0x01);
                count += 6;
                had_token = true;
            } else if rest.starts_with(b"sloth") {
                current_code = current_code.wrapping_add(1);
                count += 5;
                had_token = true;
            } else if rest.starts_with(b"and") {
                byte_code.push(current_code);
                current_code = 0;
                count += 3;
                had_token = true;
            } else if rest.starts_with(b"nap") {
                byte_code.push(0x00);
                count += 3;
                had_token = true;
            } else {
                count += 1;
            }
        }

        if had_token {
            byte_code.push(current_code);
        }
    }

    Some(SlothProgram { codes: byte_code, pc: 0 })
}
pub fn free_program(_program: Option<SlothProgram>) {
    drop(_program);
}
pub fn readline(file: &std::fs::File) -> Option<String> {
    let fd = file.as_raw_fd();
    let bytes = read_file_bytes(file);
    let mut offsets = line_offsets().lock().ok()?;
    let offset = offsets.entry(fd).or_insert(0);

    if *offset >= bytes.len() {
        return None;
    }

    let start = *offset;
    let mut end = start;
    while end < bytes.len() && bytes[end] != b'\n' {
        end += 1;
    }

    *offset = if end < bytes.len() { end + 1 } else { end };
    Some(String::from_utf8_lossy(&bytes[start..end]).into_owned())
}
pub fn prog_len(file: &std::fs::File) -> usize {
    if let Ok(mut offsets) = line_offsets().lock() {
        offsets.insert(file.as_raw_fd(), 0);
    }
    read_file_bytes(file).iter().filter(|&&b| b == b'\n').count() * 3
}
