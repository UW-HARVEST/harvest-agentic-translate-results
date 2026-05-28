// Translation of c_src/src/main.c to Rust.
// Reproduces fgets(text, 128, stdin) / fputs(text, stdout) behavior:
//   - Each "line" is up to 127 bytes or until a newline (inclusive)
//   - The C code loops until fgets returns NULL (EOF / read error with no bytes)

use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    // fgets reads up to size-1 characters (here 127), stopping early on newline.
    // We emulate that by accumulating bytes one at a time.
    let mut buf: Vec<u8> = Vec::with_capacity(128);
    let mut byte = [0u8; 1];

    loop {
        buf.clear();

        // Read up to 127 bytes, stopping after a newline.
        loop {
            if buf.len() >= 127 {
                break;
            }
            match input.read(&mut byte) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    buf.push(byte[0]);
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        // fgets returns NULL if nothing was read; mirror that to break the loop.
        if buf.is_empty() {
            break;
        }

        // fputs: write bytes up to (but not including) the first NUL.
        // fgets stores a terminating NUL after the read bytes, and fputs
        // stops at the first NUL it encounters, so embedded NULs in the
        // input truncate the output for that fgets chunk.
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        if output.write_all(&buf[..end]).is_err() {
            break;
        }
    }

    // Match C's implicit fflush at exit.
    let _ = output.flush();
}
