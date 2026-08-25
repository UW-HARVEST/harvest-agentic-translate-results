use std::io::{self, BufRead};

const BUFFER_SIZE: usize = 100;

fn fgets<R: BufRead>(input: &mut R, buffer: &mut [u8; BUFFER_SIZE]) {
    let mut written = 0;

    while written < buffer.len() - 1 {
        let available = match input.fill_buf() {
            Ok(bytes) => bytes,
            Err(_) => return,
        };
        if available.is_empty() {
            return;
        }

        let remaining = buffer.len() - 1 - written;
        let count = available
            .iter()
            .take(remaining)
            .position(|&byte| byte == b'\n')
            .map_or(available.len().min(remaining), |position| position + 1);
        let saw_newline = available[count - 1] == b'\n';

        buffer[written..written + count].copy_from_slice(&available[..count]);
        input.consume(count);
        written += count;

        if saw_newline {
            break;
        }
    }

    buffer[written] = 0;
}

fn c_strlen(buffer: &[u8]) -> usize {
    buffer.iter().position(|&byte| byte == 0).unwrap_or(buffer.len())
}

fn remove_last_c_byte(buffer: &mut [u8; BUFFER_SIZE]) {
    let length = c_strlen(buffer);
    if length != 0 {
        buffer[length - 1] = 0;
    }
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut s1 = [0_u8; BUFFER_SIZE];
    let mut s2 = [0_u8; BUFFER_SIZE];

    fgets(&mut input, &mut s1);
    fgets(&mut input, &mut s2);

    remove_last_c_byte(&mut s1);
    remove_last_c_byte(&mut s2);

    let s1_end = c_strlen(&s1);
    let s2_end = c_strlen(&s2);
    let span = s1[..s1_end]
        .iter()
        .position(|byte| s2[..s2_end].contains(byte))
        .unwrap_or(s1_end);

    println!("{span}");
}
