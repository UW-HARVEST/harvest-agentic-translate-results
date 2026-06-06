// Translated from C to Rust. Reproduces the behavior of:
//   while (fgets(text, 128, stdin)) { fputs(text, stdout); }
//
// fgets reads at most n-1 bytes (here, 127), stopping on newline (included)
// or EOF. It returns NULL only when no characters have been read and EOF
// or error occurs.

use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    // Buffer mirrors C's `char text[128]` — capacity 127 chars + NUL terminator.
    // We don't need a NUL terminator in Rust, but we cap reads at 127 bytes
    // to match fgets behavior exactly.
    const MAX_CHARS: usize = 127;
    let mut buf: Vec<u8> = Vec::with_capacity(MAX_CHARS);

    loop {
        buf.clear();
        let mut byte = [0u8; 1];

        // Mimic fgets: read until newline (inclusive), EOF, or 127 chars.
        loop {
            match input.read(&mut byte) {
                Ok(0) => {
                    // EOF reached
                    break;
                }
                Ok(_) => {
                    buf.push(byte[0]);
                    if byte[0] == b'\n' {
                        break;
                    }
                    if buf.len() >= MAX_CHARS {
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(_) => {
                    // On error with no chars read, fgets returns NULL.
                    // Match by terminating the loop.
                    break;
                }
            }
        }

        if buf.is_empty() {
            // fgets returned NULL — exit the outer loop.
            break;
        }

        // fputs writes the string (without adding a newline). Match exactly.
        if output.write_all(&buf).is_err() {
            break;
        }
    }

    let _ = output.flush();
}
