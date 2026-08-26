use std::io::{self, Read, Write};

fn fgets_like<R: Read>(reader: &mut R, buf: &mut [u8]) {
    if buf.is_empty() {
        return;
    }

    let mut i = 0usize;
    while i + 1 < buf.len() {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(1) => {
                buf[i] = byte[0];
                i += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Ok(_) => unreachable!(),
            Err(_) => break,
        }
    }

    buf[i] = 0;
}

fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn strcspn_bytes(s1: &[u8], s2: &[u8]) -> usize {
    let s1_len = c_strlen(s1);
    let s2_len = c_strlen(s2);

    for (idx, &byte) in s1[..s1_len].iter().enumerate() {
        if s2[..s2_len].contains(&byte) {
            return idx;
        }
    }

    s1_len
}

fn driver<W: Write>(out: &mut W, s1: &[u8], s2: &[u8]) -> io::Result<()> {
    writeln!(out, "{}", strcspn_bytes(s1, s2))
}

fn main() -> io::Result<()> {
    let mut s1 = [0u8; 100];
    let mut s2 = [0u8; 100];

    let stdin = io::stdin();
    let mut handle = stdin.lock();

    fgets_like(&mut handle, &mut s1);
    fgets_like(&mut handle, &mut s2);

    let s1_len = c_strlen(&s1);
    let s2_len = c_strlen(&s2);

    if s1_len > 0 {
        s1[s1_len - 1] = 0;
    }
    if s2_len > 0 {
        s2[s2_len - 1] = 0;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, &s1, &s2)
}
