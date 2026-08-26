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

use std::io::{self, Read, Write};

#[allow(dead_code)]
fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

// Translation of the original C `bad()` function.
//
// The original C code calls `alloca(10)`, which allocates only 10 bytes on the
// stack and then treats the pointer as `int *`, writing 10 ints (40 bytes) into
// it. That is undefined behavior in C (a buffer overflow). We faithfully model
// the visible observable behavior here by mimicking an under-sized allocation
// and only writing into the bytes that legitimately belong to `data`.
fn bad() {
    // Mimic the under-sized allocation: 10 bytes interpreted as i32 storage.
    // 10 bytes / 4 bytes per i32 = 2 full i32 slots.
    let mut data: [i32; 10 / std::mem::size_of::<i32>()] =
        [0; 10 / std::mem::size_of::<i32>()];
    let source: [i32; 10] = [0; 10];

    // Avoid the actual out-of-bounds writes (which would be UB in Rust) by
    // bounding the copy to the size of `data`. The observable output (data[0])
    // matches the C program's expected output of 0.
    let len = data.len();
    for i in 0..len {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    // Properly sized allocation: 10 * sizeof(int) bytes.
    let mut data: [i32; 10] = [0; 10];
    let source: [i32; 10] = [0; 10];

    for i in 0..10usize {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        // If stdin can't be read, treat as 0 (matches uninitialized scanf default of 0 above).
    }
    let _ = io::stdout().flush();

    // Replicate scanf("%d", &x): parse the first whitespace-delimited integer.
    let x: i32 = input
        .split_whitespace()
        .next()
        .and_then(|tok| tok.parse::<i32>().ok())
        .unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
