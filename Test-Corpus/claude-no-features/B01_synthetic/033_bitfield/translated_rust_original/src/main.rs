// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces the behavior of the original C program
// including bit-field truncation semantics.

use std::io::{self, Read};

struct Foo {
    // Stored as full-width values, but the public accessors truncate to the
    // bit-field widths used in the original C struct:
    //   unsigned int x : 2;   -> 2 bits
    //   unsigned int y : 3;   -> 3 bits
    //   bool b : 1;           -> 1 bit
    //   int z;                -> full int (32 bits)
    x: u32,
    y: u32,
    b: bool,
    z: i32,
}

impl Foo {
    fn new(x: u32, y: u32, b: bool, z: i32) -> Self {
        // Mimic C bit-field assignment truncation behavior.
        Foo {
            x: x & 0x3,
            y: y & 0x7,
            // bool : 1 -- a bool is already 0 or 1.
            b,
            z,
        }
    }
}

fn print_foo(foo: &Foo) {
    // In C: printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
    // foo->b is a bool printed with %d -> prints 0 or 1.
    println!(
        "{} {} {} {}",
        foo.x,
        foo.y,
        if foo.b { 1 } else { 0 },
        foo.z
    );
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo::new(x, y, b, z);
    print_foo(&foo);
}

// scanf-like whitespace-delimited token reader.
struct TokenReader {
    data: Vec<u8>,
    pos: usize,
}

impl TokenReader {
    fn from_stdin() -> Self {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).expect("failed to read stdin");
        TokenReader { data: buf, pos: 0 }
    }

    fn next_token(&mut self) -> Option<&[u8]> {
        // Skip whitespace (matches C isspace for ASCII whitespace).
        while self.pos < self.data.len() && is_c_space(self.data[self.pos]) {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.data.len() && !is_c_space(self.data[self.pos]) {
            self.pos += 1;
        }
        Some(&self.data[start..self.pos])
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

fn parse_u32(tok: &[u8]) -> u32 {
    // Mimic scanf %u: skip leading whitespace already done; accept optional
    // sign; parse decimal digits. We rely on the token being non-empty and
    // produce 0 if no digits, matching the program's pre-initialized x/y=0
    // when scanf fails (the C program does not check scanf return values).
    let s = std::str::from_utf8(tok).unwrap_or("");
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut val: u32 = 0;
    let mut any = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        any = true;
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as u32);
        i += 1;
    }
    if !any {
        return 0;
    }
    if neg {
        // C %u with negative: it actually accepts a sign and stores the
        // negation as unsigned (modulo 2^32).
        0u32.wrapping_sub(val)
    } else {
        val
    }
}

fn parse_i32(tok: &[u8]) -> i32 {
    let s = std::str::from_utf8(tok).unwrap_or("");
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut val: i32 = 0;
    let mut any = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        any = true;
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i32);
        i += 1;
    }
    if !any {
        return 0;
    }
    if neg { val.wrapping_neg() } else { val }
}

fn main() {
    let mut reader = TokenReader::from_stdin();

    // unsigned int x = 0, y = 0; int b = 0, z = 0;
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    if let Some(tok) = reader.next_token() {
        x = parse_u32(tok);
    }
    if let Some(tok) = reader.next_token() {
        y = parse_u32(tok);
    }
    if let Some(tok) = reader.next_token() {
        b = parse_i32(tok);
    }
    if let Some(tok) = reader.next_token() {
        z = parse_i32(tok);
    }

    // driver(x, y, !!b, z);
    driver(x, y, b != 0, z);
}
