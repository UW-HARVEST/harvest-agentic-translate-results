use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut buf = [0u8; 127]; // fgets(text, 128, stdin) reads up to 127 bytes + null
    let mut inp = stdin.lock();
    loop {
        let mut i = 0;
        loop {
            let mut byte = [0u8; 1];
            match inp.read(&mut byte) {
                Ok(0) | Err(_) => {
                    if i == 0 {
                        return;
                    }
                    let _ = out.write_all(&buf[..i]);
                    return;
                }
                Ok(_) => {
                    buf[i] = byte[0];
                    i += 1;
                    if byte[0] == b'\n' || i == 127 {
                        break;
                    }
                }
            }
        }
        let _ = out.write_all(&buf[..i]);
    }
}
