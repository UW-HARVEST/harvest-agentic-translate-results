// Copyright 2025 MIT Lincoln Laboratory
// Translated from c_src/src/driver.c

/// Mirrors C's `div_t div(int, int)`.
/// In C, `div(x, y)` returns the quotient and remainder where the
/// quotient is truncated toward zero (matching i32 division in Rust).
fn c_div(x: i32, y: i32) -> (i32, i32) {
    // C's div() truncates toward zero; Rust's `/` and `%` on i32 do the same.
    let quot = x / y;
    let rem = x % y;
    (quot, rem)
}

pub fn driver(x: i32, y: i32) {
    let (quot, rem) = c_div(x, y);
    print!("quotient: {}, remainder: {}\n", quot, rem);
}
