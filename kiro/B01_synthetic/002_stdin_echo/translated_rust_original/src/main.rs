use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut input = stdin.lock();
    // fgets(text, 128, stdin) reads up to 127 bytes, stopping at \n (inclusive) or EOF.
    let mut buf = [0u8; 127];
    loop {
        let mut i = 0;
        loop {
            if i >= 127 {
                break;
            }
            let mut byte = [0u8; 1];
            match input.read(&mut byte) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    buf[i] = byte[0];
                    i += 1;
                    if byte[0] == b'\n' {
                        break;
                    }
                }
            }
        }
        if i == 0 {
            break;
        }
        let _ = out.write_all(&buf[..i]);
    }
}
