use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut buf = [0_u8; 127];
    let mut one = [0_u8; 1];

    loop {
        let mut len = 0usize;

        while len < buf.len() {
            match input.read(&mut one) {
                Ok(0) => break,
                Ok(1) => {
                    buf[len] = one[0];
                    len += 1;
                    if one[0] == b'\n' {
                        break;
                    }
                }
                Ok(_) => unreachable!(),
                Err(_) => return,
            }
        }

        if len == 0 {
            break;
        }

        if output.write_all(&buf[..len]).is_err() {
            return;
        }
    }
}
