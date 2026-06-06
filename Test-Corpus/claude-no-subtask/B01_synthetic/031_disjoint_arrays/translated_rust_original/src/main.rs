use std::io::{self, Read, Write, BufWriter};

/// Reads a single byte from the given iterator-like state, returning None on EOF.
struct StdinReader {
    buf: Vec<u8>,
    pos: usize,
}

impl StdinReader {
    fn new() -> Self {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).ok();
        StdinReader { buf, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn unget(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }
}

/// Mimic scanf("%d", &x). Returns Some(value) if a value was parsed,
/// or None if EOF or a matching failure occurred.
fn scanf_d(r: &mut StdinReader) -> Option<i32> {
    // Skip whitespace
    loop {
        match r.peek() {
            Some(b) if (b as char).is_ascii_whitespace() => {
                r.next();
            }
            Some(_) => break,
            None => return None,
        }
    }

    // Optional sign
    let mut sign: i64 = 1;
    let mut had_sign = false;
    match r.peek() {
        Some(b'+') => {
            r.next();
            had_sign = true;
        }
        Some(b'-') => {
            r.next();
            sign = -1;
            had_sign = true;
        }
        _ => {}
    }

    // Need at least one digit
    let mut has_digit = false;
    let mut value: i64 = 0;
    loop {
        match r.peek() {
            Some(b) if b.is_ascii_digit() => {
                r.next();
                has_digit = true;
                value = value.wrapping_mul(10).wrapping_add((b - b'0') as i64);
            }
            _ => break,
        }
    }

    if !has_digit {
        // Matching failure: push back the sign character if we consumed one
        if had_sign {
            r.unget();
        }
        return None;
    }

    let result = sign.wrapping_mul(value);
    Some(result as i32)
}

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut out = vec![0i32; len];
    let mut ones = vec![0i32; len];
    let mut zeros = vec![0i32; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

fn main() {
    let mut reader = StdinReader::new();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut data = [0i32; 100];
    let mut i: usize = 0;
    while i < 100 {
        match scanf_d(&mut reader) {
            Some(v) => {
                data[i] = v;
                i += 1;
            }
            None => break,
        }
    }

    let result = call_fma(&data, i);
    writeln!(out, "{}", result).unwrap();
}
