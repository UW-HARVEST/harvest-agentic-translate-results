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

static mut INNER: i32 = 1;

unsafe fn static_alias(outer: *mut i32) -> *mut i32 {
    if *outer >= INNER {
        INNER += *outer;
        &raw mut INNER
    } else {
        *outer += INNER;
        outer
    }
}

/// Parse a string as an integer using strtol-like semantics:
/// returns Some(value) if any leading characters parsed as a base-10 integer,
/// None otherwise (i.e. nothing was parsed).
fn strtol_like(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip leading whitespace (matches strtol behavior)
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let start = i;
    // optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if digits_start == i {
        // no digits parsed; strtol sets endptr to original start
        return None;
    }
    // parse the consumed substring
    let parsed = &s[start..i];
    parsed.parse::<i32>().ok()
}

/*
  Maintain a sum leveraging multiple references to a static variable
 */
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 3 {
        println!("Error: should only be two (integer) arguments!");
        return ExitCode::from(1);
    }

    let initial_parsed = strtol_like(&args[1]);
    let mut initial_value: i32 = match initial_parsed {
        Some(v) => v,
        None => {
            println!("Error: first argument must be an integer!");
            return ExitCode::from(1);
        }
    };

    let iterations: i32 = match strtol_like(&args[2]) {
        Some(v) => v,
        None => {
            println!("Error: second argument must be an integer!");
            return ExitCode::from(1);
        }
    };

    unsafe {
        let mut running_sum: *mut i32 = &mut initial_value as *mut i32;
        for _i in 0..iterations {
            running_sum = static_alias(running_sum);
            println!("{}", *running_sum);
        }
    }

    ExitCode::from(0)
}
