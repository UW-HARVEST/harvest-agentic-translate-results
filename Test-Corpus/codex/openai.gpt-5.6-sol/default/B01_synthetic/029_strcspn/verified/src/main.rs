use std::io::{self, Read, Write};

const BUFFER_SIZE: usize = 100;

fn fgets<R: Read>(reader: &mut R, buffer: &mut [u8; BUFFER_SIZE]) {
    let mut length = 0;

    while length < buffer.len() - 1 {
        let mut byte = [0_u8; 1];
        match reader.read(&mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                buffer[length] = byte[0];
                length += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
        }
    }

    if length != 0 {
        buffer[length] = 0;
    }
}

fn strlen(buffer: &[u8; BUFFER_SIZE]) -> usize {
    buffer
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(buffer.len())
}

fn remove_last_visible_byte(buffer: &mut [u8; BUFFER_SIZE]) {
    if let Some(index) = strlen(buffer).checked_sub(1) {
        buffer[index] = 0;
    }
}

fn strcspn(s1: &[u8; BUFFER_SIZE], s2: &[u8; BUFFER_SIZE]) -> usize {
    let mut rejected = [false; 256];
    for &byte in s2.iter().take_while(|&&byte| byte != 0) {
        rejected[byte as usize] = true;
    }

    s1.iter()
        .take_while(|&&byte| byte != 0)
        .position(|&byte| rejected[byte as usize])
        .unwrap_or_else(|| strlen(s1))
}

fn main() {
    let mut s1 = [0_u8; BUFFER_SIZE];
    let mut s2 = [0_u8; BUFFER_SIZE];
    let stdin = io::stdin();
    let mut input = stdin.lock();

    fgets(&mut input, &mut s1);
    fgets(&mut input, &mut s2);

    remove_last_visible_byte(&mut s1);
    remove_last_visible_byte(&mut s2);

    let stdout = io::stdout();
    let mut output = stdout.lock();
    let _ = writeln!(output, "{}", strcspn(&s1, &s2));
}
