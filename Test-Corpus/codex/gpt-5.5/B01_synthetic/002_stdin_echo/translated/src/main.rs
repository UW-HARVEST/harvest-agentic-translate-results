use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    let mut text = [0_u8; 127];

    loop {
        let mut len = 0;

        while len < text.len() {
            let mut byte = [0_u8; 1];
            match input.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    text[len] = byte[0];
                    len += 1;
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(_) => return,
            }
        }

        if len == 0 {
            break;
        }

        let output_len = text[..len].iter().position(|&byte| byte == 0).unwrap_or(len);
        if output.write_all(&text[..output_len]).is_err() {
            return;
        }
    }
}
