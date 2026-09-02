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

//! Rust translation of `c_src/src/main.c` (a divide-by-zero test driver).
//!
//! The original divide-by-zero behaviour is preserved deliberately: `bad()`
//! divides by a float that may be `0.0`, and the resulting out-of-range
//! `(int)` conversion is reproduced rather than fixed.

mod cio;
mod cruntime;

use cio::{flush, printf_int_line, printf_line};
use cruntime::{atof, f64_to_int, fgets};

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        // C's `printf` reports write errors through its return value, which
        // `printLine` discards; it never aborts. `println!` would panic, so the
        // byte-faithful `cio` writers are used instead.
        printf_line(line);
    }
}

fn print_int_line(int_number: i32) {
    printf_int_line(int_number);
}

const CHAR_ARRAY_SIZE: usize = 20;

fn bad() {
    let mut data: f32;
    data = 0.0f32;
    {
        match fgets(CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = atof(&input_buffer) as f32;
            }
            None => {
                print_line(Some("fgets() failed."));
            }
        }
    }
    {
        let result = f64_to_int(100.0 / data as f64);
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32;
    data = 2.0f32;
    {
        let result = f64_to_int(100.0 / data as f64);
        print_int_line(result);
    }
}

fn good_b2g() {
    let mut data: f32;
    data = 0.0f32;
    {
        match fgets(CHAR_ARRAY_SIZE) {
            Some(input_buffer) => {
                data = atof(&input_buffer) as f32;
            }
            None => {
                print_line(Some("fgets() failed."));
            }
        }
    }
    if (data as f64).abs() > 0.000001 {
        let result = f64_to_int(100.0 / data as f64);
        print_int_line(result);
    } else {
        print_line(Some("This would result in a divide by zero"));
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));
    // `stdio` flushes at C `exit` and ignores any failure there too.
    flush();
    std::process::exit(0);
}
