// Translation of c_src/src/main.c to Rust.
// Replicates C's fgets(text, 128, stdin) / fputs(text, stdout) loop:
// reads up to 127 bytes per chunk, stopping early on a newline (which is
// kept) or on EOF, and writes each chunk back to stdout. Loop ends when
// fgets would return NULL (EOF reached with no bytes read).

use std::io::{Read, Write};

fn fgets_like<R: Read>(reader: &mut R, buf: &mut Vec<u8>, max_minus_one: usize) -> bool {
    // Mimic fgets: read up to (max_minus_one) bytes, stop at newline (kept),
    // or at EOF. Return true if at least one byte was read (i.e. fgets would
    // return non-NULL); false on immediate EOF.
    buf.clear();
    let mut byte = [0u8; 1];
    while buf.len() < max_minus_one {
        match reader.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    !buf.is_empty()
}

fn main() {
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let mut buf: Vec<u8> = Vec::with_capacity(128);
    // C buffer is char text[128]; fgets reads up to 127 bytes plus a NUL.
    while fgets_like(&mut stdin, &mut buf, 127) {
        // fputs writes the string contents (no added newline).
        let _ = stdout.write_all(&buf);
    }
    let _ = stdout.flush();
    std::process::exit(0);
}
