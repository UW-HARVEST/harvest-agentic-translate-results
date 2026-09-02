//! Byte-faithful stand-ins for the C stdio calls used by the driver.

use std::io::{self, Read, Write};

/// `fgets(buffer, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping *after* the first newline, and
/// never consuming input beyond it. Returns `None` exactly when C's `fgets`
/// returns `NULL`: end-of-file reached with no bytes stored, or a read error.
///
/// The returned vector holds the stored bytes, newline included, without the
/// terminating NUL that C appends.
#[allow(dead_code)] // `cruntime::fgets` is the implementation in use.
pub fn fgets(size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }
    let max = size - 1;
    let mut out: Vec<u8> = Vec::with_capacity(max);

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];

    while out.len() < max {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// `printf("%s\n", line)`
pub fn printf_line(line: &str) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(line.as_bytes());
    let _ = handle.write_all(b"\n");
}

/// `printf("%d\n", intNumber)`
pub fn printf_int_line(int_number: i32) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(int_number.to_string().as_bytes());
    let _ = handle.write_all(b"\n");
}

/// Flush stdout, mirroring the implicit flush at C `exit`.
pub fn flush() {
    let _ = io::stdout().flush();
}
