use std::io::{self, Read};

struct Foo {
    // Bit-fields from the C source:
    // unsigned int x : 2;  (range 0..=3)
    // unsigned int y : 3;  (range 0..=7)
    // bool b : 1;          (range 0..=1)
    // int z;
    x: u32,
    y: u32,
    b: bool,
    z: i32,
}

impl Foo {
    fn new(x: u32, y: u32, b: bool, z: i32) -> Self {
        // Mimic C truncation behavior of bit-fields.
        Foo {
            x: x & 0b11,
            y: y & 0b111,
            b, // bool bit-field of width 1; values are 0 or 1.
            z,
        }
    }
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x, foo.y, foo.b as i32, foo.z);
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo::new(x, y, b, z);
    print_foo(&foo);
}

/// Read all of stdin into a string for tokenization (mimics scanf's
/// whitespace-delimited reading across newlines).
fn read_all_stdin() -> String {
    let mut s = String::new();
    let _ = io::stdin().read_to_string(&mut s);
    s
}

/// scanf-like next token (any whitespace separator). Returns the next
/// non-whitespace token, advancing the cursor.
fn next_token<'a>(input: &'a str, pos: &mut usize) -> Option<&'a str> {
    let bytes = input.as_bytes();
    // Skip leading whitespace.
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= bytes.len() {
        return None;
    }
    let start = *pos;
    while *pos < bytes.len() && !(bytes[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    Some(&input[start..*pos])
}

/// Mimic scanf("%u", ...): parse leading optional sign, then digits.
/// On success, returns the u32 (with C's wrapping for negative values).
/// On failure, returns 0 (C leaves the variable unchanged; we initialize to 0).
fn parse_u(tok: &str) -> u32 {
    let bytes = tok.as_bytes();
    let mut i = 0;
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            negative = true;
        }
        i += 1;
    }
    let start = i;
    let mut val: u32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as u32);
        i += 1;
    }
    if i == start {
        // No digits parsed; C would leave value unchanged (initialized to 0).
        return 0;
    }
    if negative {
        val = 0u32.wrapping_sub(val);
    }
    val
}

/// Mimic scanf("%d", ...): parse leading optional sign, then digits.
fn parse_i(tok: &str) -> i32 {
    let bytes = tok.as_bytes();
    let mut i = 0;
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            negative = true;
        }
        i += 1;
    }
    let start = i;
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return 0;
    }
    if negative {
        val = -val;
    }
    val as i32
}

fn main() {
    let input = read_all_stdin();
    let mut pos = 0usize;

    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    if let Some(tok) = next_token(&input, &mut pos) {
        x = parse_u(tok);
    }
    if let Some(tok) = next_token(&input, &mut pos) {
        y = parse_u(tok);
    }
    if let Some(tok) = next_token(&input, &mut pos) {
        b = parse_i(tok);
    }
    if let Some(tok) = next_token(&input, &mut pos) {
        z = parse_i(tok);
    }

    // !!b: nonzero -> true, zero -> false.
    let b_bool = b != 0;
    driver(x, y, b_bool, z);
}
