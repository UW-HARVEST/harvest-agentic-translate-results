use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut buf = [0u8; 127];
    let mut inp = stdin.lock();
    // Mimic fgets(text, 128, stdin): read up to 127 bytes, stop at \n (inclusive)
    loop {
        let mut i = 0;
        loop {
            let mut byte = [0u8; 1];
            match inp.read(&mut byte) {
                Ok(0) => {
                    // EOF
                    if i > 0 {
                        let _ = out.write_all(&buf[..i]);
                    }
                    return;
                }
                Ok(_) => {
                    buf[i] = byte[0];
                    i += 1;
                    if byte[0] == b'\n' || i == 127 {
                        break;
                    }
                }
                Err(_) => return,
            }
        }
        let _ = out.write_all(&buf[..i]);
    }
}
