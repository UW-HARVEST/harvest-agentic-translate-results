// Translation of c_src/src/main.c
// Mirrors C's fgets(text, 128, stdin) which reads up to 127 bytes, stopping
// at and including a newline if one is encountered, then writes the chunk to
// stdout via fputs.

use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    // Buffer mimicking `char text[128]`: at most 127 bytes plus C's null
    // terminator. fgets stops on newline (kept) or after n-1 bytes or at EOF.
    const CAP: usize = 127;
    let mut buf: Vec<u8> = Vec::with_capacity(CAP);

    loop {
        buf.clear();
        let mut byte = [0u8; 1];
        let mut got_any = false;

        // Read up to CAP bytes, stopping if we encounter a newline.
        while buf.len() < CAP {
            match stdin.read(&mut byte) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    got_any = true;
                    buf.push(byte[0]);
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }

        if !got_any {
            // fgets returned NULL with nothing read — exit the loop.
            break;
        }

        // fputs writes the buffer (sans null terminator) to stdout.
        if stdout.write_all(&buf).is_err() {
            break;
        }
    }

    let _ = stdout.flush();
}
