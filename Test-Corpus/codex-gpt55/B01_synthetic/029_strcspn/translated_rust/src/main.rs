use std::io::{self, Read};

const BUF_SIZE: usize = 100;

fn fgets_like(input: &[u8], pos: &mut usize, buf: &mut [u8; BUF_SIZE]) {
    if *pos >= input.len() {
        return;
    }

    let mut written = 0;
    while written < BUF_SIZE - 1 && *pos < input.len() {
        let byte = input[*pos];
        *pos += 1;
        buf[written] = byte;
        written += 1;
        if byte == b'\n' {
            break;
        }
    }
    buf[written] = 0;
}

fn c_strlen(buf: &[u8; BUF_SIZE]) -> usize {
    buf.iter().position(|&byte| byte == 0).unwrap_or(BUF_SIZE)
}

fn strip_last_c_char(buf: &mut [u8; BUF_SIZE]) {
    let len = c_strlen(buf);
    if len > 0 {
        buf[len - 1] = 0;
    }
}

fn strcspn_like(s1: &[u8; BUF_SIZE], s2: &[u8; BUF_SIZE]) -> usize {
    let s1_len = c_strlen(s1);
    let s2_len = c_strlen(s2);

    for (idx, &byte) in s1[..s1_len].iter().enumerate() {
        if s2[..s2_len].contains(&byte) {
            return idx;
        }
    }

    s1_len
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let mut s1 = [0_u8; BUF_SIZE];
    let mut s2 = [0_u8; BUF_SIZE];
    let mut pos = 0;

    fgets_like(&input, &mut pos, &mut s1);
    fgets_like(&input, &mut pos, &mut s2);

    strip_last_c_char(&mut s1);
    strip_last_c_char(&mut s2);

    println!("{}", strcspn_like(&s1, &s2));
}
