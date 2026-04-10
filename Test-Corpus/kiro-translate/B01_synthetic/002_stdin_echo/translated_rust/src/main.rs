use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut input = stdin.lock();
    let mut buf = [0u8; 127];

    loop {
        // Mimic fgets(text, 128, stdin): read up to 127 bytes, stop at newline
        let mut pos = 0;
        loop {
            if pos >= 127 {
                break;
            }
            let mut byte = [0u8; 1];
            match input.read(&mut byte) {
                Ok(0) | Err(_) => {
                    if pos == 0 {
                        return;
                    }
                    break;
                }
                Ok(_) => {
                    buf[pos] = byte[0];
                    pos += 1;
                    if byte[0] == b'\n' {
                        break;
                    }
                }
            }
        }
        let _ = out.write_all(&buf[..pos]);
    }
}
