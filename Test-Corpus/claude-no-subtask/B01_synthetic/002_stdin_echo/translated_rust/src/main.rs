// Translation of c_src/src/main.c to Rust
// interactive echo; ignores arguments, copies stdin to stdout

use std::io::{Read, Write};
use std::process::ExitCode;

/// Mimic C's `fgets(buf, 128, stdin)`:
/// Reads up to 127 bytes from stdin into `buf`, stopping when:
/// - a newline is read (the newline is included in `buf`),
/// - EOF is encountered, or
/// - 127 bytes have been read.
///
/// Returns `Ok(Some(n))` where `n` is the number of bytes read into `buf`,
/// or `Ok(None)` if EOF was reached before any byte was read (fgets returns NULL).
fn fgets_like<R: Read>(reader: &mut R, buf: &mut Vec<u8>) -> std::io::Result<Option<usize>> {
    buf.clear();
    let max_chars: usize = 127; // 128 - 1 (room for null terminator in C)
    let mut byte = [0u8; 1];
    while buf.len() < max_chars {
        match reader.read(&mut byte) {
            Ok(0) => {
                // EOF
                if buf.is_empty() {
                    return Ok(None);
                } else {
                    return Ok(Some(buf.len()));
                }
            }
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    return Ok(Some(buf.len()));
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
        }
    }
    Ok(Some(buf.len()))
}

fn main() -> ExitCode {
    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let mut buf: Vec<u8> = Vec::with_capacity(128);

    loop {
        match fgets_like(&mut stdin_lock, &mut buf) {
            Ok(Some(_)) => {
                if let Err(_) = stdout_lock.write_all(&buf) {
                    return ExitCode::from(1);
                }
            }
            Ok(None) => break,
            Err(_) => return ExitCode::from(1),
        }
    }

    let _ = stdout_lock.flush();
    ExitCode::from(0)
}
