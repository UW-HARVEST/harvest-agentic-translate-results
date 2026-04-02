use std::io::{self, Read, Write};

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut input = stdin.lock();
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
        // fputs stops at the first null byte (C string semantics)
        let len = buf[..i].iter().position(|&b| b == 0).unwrap_or(i);
        let _ = out.write_all(&buf[..len]);
    }
    0
}
