use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut text = [0_u8; 127];

    loop {
        let mut length = 0;
        let mut reached_eof_or_error = false;

        while length < text.len() {
            match input.read(&mut text[length..=length]) {
                Ok(0) | Err(_) => {
                    reached_eof_or_error = true;
                    break;
                }
                Ok(_) => {
                    let byte = text[length];
                    length += 1;
                    if byte == b'\n' {
                        break;
                    }
                }
            }
        }

        if length == 0 {
            break;
        }

        let visible_length = text[..length]
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(length);
        let _ = output.write_all(&text[..visible_length]);

        if reached_eof_or_error {
            break;
        }
    }
}
