// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::env;
use std::process::ExitCode;
use std::sync::Mutex;

// Mimic C's `static int sum = 0;` inside the function by using a
// function-scoped static guarded by a Mutex.
fn static_sum(update: i32) -> i32 {
    static SUM: Mutex<i32> = Mutex::new(0);
    let mut sum = SUM.lock().unwrap();
    *sum += update;
    *sum
}

/// Parse a leading integer from `s` the way C's `strtol(s, &end, 10)` does:
/// consume optional whitespace and sign, then as many decimal digits as
/// possible. Returns `Some((value, consumed_any_digits))`.
///
/// The second element of the tuple corresponds to whether any characters
/// were consumed beyond the initial position (i.e. `end != s` in C).
fn c_strtol(s: &str) -> (i64, bool) {
    let bytes = s.as_bytes();
    let mut i = 0;

    // Skip leading whitespace (C strtol behavior).
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }

    let start_after_ws = i;

    // Optional sign.
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut value: i64 = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        let d = (bytes[i] - b'0') as i64;
        value = value.saturating_mul(10).saturating_add(d);
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: in C, end == argv[1] (no progress at all).
        // Match C semantics: nothing parsed -> end points back to the
        // original start, so return false for "consumed_any_digits".
        // Note: pure whitespace/sign without digits is also a parse failure
        // as far as the original program is concerned.
        return (0, start_after_ws != 0 && i != start_after_ws);
    }

    if negative {
        value = -value;
    }
    (value, true)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    let (parsed, consumed_any) = c_strtol(&args[1]);
    if !consumed_any {
        // Mirror C's check: end == argv[1] means nothing was parsed.
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    let stride = parsed as i32;

    for i in 0..10i32 {
        println!("{}", static_sum(i * stride));
    }

    ExitCode::from(0)
}
