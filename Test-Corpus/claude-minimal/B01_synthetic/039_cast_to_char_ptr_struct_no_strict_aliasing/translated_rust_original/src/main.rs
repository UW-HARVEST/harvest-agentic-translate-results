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

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct HouseT {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(floors: i32) {
    let mut house: HouseT = HouseT::default();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    let size = std::mem::size_of::<HouseT>();
    let mut raw = vec![0u8; size];
    // memcpy(raw, &house, sizeof(house))
    unsafe {
        std::ptr::copy_nonoverlapping(
            &house as *const HouseT as *const u8,
            raw.as_mut_ptr(),
            size,
        );
    }
    print_hex(&raw);
}

fn main() {
    // Mimic scanf("%d", &x): read an integer (possibly with surrounding whitespace) from stdin.
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    // scanf("%d", ...) skips leading whitespace and parses the longest valid integer prefix.
    // If parsing fails, x stays 0 (matching C's `int x = 0;` initialization).
    let mut x: i32 = 0;
    let trimmed = input.trim_start();
    // Find the integer prefix
    let mut end = 0;
    let bytes = trimmed.as_bytes();
    let mut start = 0;
    if !bytes.is_empty() && (bytes[0] == b'+' || bytes[0] == b'-') {
        start = 1;
        end = 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end > start {
        if let Ok(parsed) = trimmed[..end].parse::<i32>() {
            x = parsed;
        }
    }

    driver(x);
}
