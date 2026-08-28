use std::io::{self, ErrorKind, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut text = [0_u8; 127];

    loop {
        let mut len = 0;

        while len < text.len() {
            match input.read(&mut text[len..len + 1]) {
                Ok(0) => break,
                Ok(_) => {
                    len += 1;
                    if text[len - 1] == b'\n' {
                        break;
                    }
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(_) => return,
            }
        }

        if len == 0 {
            return;
        }

        let output_len = text[..len]
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(len);

        if output.write_all(&text[..output_len]).is_err() {
            return;
        }
    }
}
