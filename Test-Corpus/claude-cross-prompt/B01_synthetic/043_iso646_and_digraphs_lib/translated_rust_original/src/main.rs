// Translated from c_src/src/driver.c
// The C source uses digraphs (%: -> #, <% -> {, %> -> }) and iso646
// alternative tokens (bitor -> |, compl -> ~).
//
// Original `driver` function (decoded):
//     void driver(int x, int y) {
//         int result = x | ~y;
//         printf("%d", result);
//         puts("");
//     }
//
// The C source has no main(); to make this an executable we read two
// integers from stdin (mirroring scanf("%d %d", &x, &y) behavior, which
// reads across whitespace/newlines) and invoke driver.

use std::io::{self, Read, Write};

fn driver(x: i32, y: i32) {
    // x bitor compl y  ==  x | !y  (in Rust, ! is bitwise not on integers)
    let result = x | !y;
    // printf("%d", result); puts(""); -> prints number followed by newline
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}", result);
    let _ = writeln!(out);
}

fn read_int(buf: &[u8], pos: &mut usize) -> Option<i32> {
    // Mimic scanf("%d", ...): skip whitespace (including newlines), then
    // read optional sign and decimal digits.
    while *pos < buf.len() {
        let c = buf[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r'
            || c == 0x0b || c == 0x0c
        {
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos >= buf.len() {
        return None;
    }
    let start = *pos;
    let mut sign: i64 = 1;
    if buf[*pos] == b'+' {
        *pos += 1;
    } else if buf[*pos] == b'-' {
        sign = -1;
        *pos += 1;
    }
    let digits_start = *pos;
    let mut value: i64 = 0;
    while *pos < buf.len() {
        let c = buf[*pos];
        if c.is_ascii_digit() {
            value = value.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos == digits_start {
        // No digits parsed; restore position and fail.
        *pos = start;
        return None;
    }
    Some((sign.wrapping_mul(value)) as i32)
}

fn main() {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        return;
    }
    let mut pos: usize = 0;
    let x = match read_int(&input, &mut pos) {
        Some(v) => v,
        None => return,
    };
    let y = match read_int(&input, &mut pos) {
        Some(v) => v,
        None => return,
    };
    driver(x, y);
}
