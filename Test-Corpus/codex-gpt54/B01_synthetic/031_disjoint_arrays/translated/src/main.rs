use std::io::{self, Read};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }

    let mut out = vec![0_i32; len];
    let mut ones = vec![0_i32; len];
    let mut zeros = vec![0_i32; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

struct ScanfDecimal<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ScanfDecimal<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn next_i32(&mut self) -> Option<i32> {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }

        if self.pos >= self.bytes.len() {
            return None;
        }

        let start = self.pos;
        let mut negative = false;

        match self.bytes[self.pos] {
            b'+' => {
                self.pos += 1;
            }
            b'-' => {
                negative = true;
                self.pos += 1;
            }
            _ => {}
        }

        let digit_start = self.pos;
        let mut value: i32 = 0;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            value = value
                .wrapping_mul(10)
                .wrapping_add((self.bytes[self.pos] - b'0') as i32);
            self.pos += 1;
        }

        if self.pos == digit_start {
            self.pos = start;
            return None;
        }

        Some(if negative { value.wrapping_neg() } else { value })
    }
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();

    let mut scanner = ScanfDecimal::new(&input);
    let mut data = [0_i32; 100];
    let mut i = 0_usize;

    while i < 100 {
        match scanner.next_i32() {
            Some(value) => {
                data[i] = value;
                i += 1;
            }
            None => break,
        }
    }

    let result = call_fma(&data, i);
    println!("{result}");
}
