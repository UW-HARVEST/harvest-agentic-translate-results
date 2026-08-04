use std::io::{self, Read, Write};

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut input = stdin.lock();
    let mut buf = [0u8; 127];

    loop {
        let mut pos = 0;
        loop {
            if pos >= 127 {
                break;
            }
            let mut byte = [0u8; 1];
            match input.read(&mut byte) {
                Ok(0) | Err(_) => {
                    if pos == 0 {
                        let _ = out.flush();
                        return 0;
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
        // fputs stops at first null byte
        let len = buf[..pos].iter().position(|&b| b == 0).unwrap_or(pos);
        let _ = out.write_all(&buf[..len]);
    }
}
