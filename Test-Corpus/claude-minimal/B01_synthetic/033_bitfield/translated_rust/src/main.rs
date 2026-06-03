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

use std::io::{self, BufRead, Read, Write};

// Emulates the C bit-field struct:
//   unsigned int x : 2;
//   unsigned int y : 3;
//   bool b : 1;
//   int z;
// We store the small fields in their natural-sized integer types and mask
// on assignment to mirror the truncation semantics of C bit-fields.
#[derive(Default, Copy, Clone)]
struct Foo {
    x: u32, // 2 bits
    y: u32, // 3 bits
    b: bool,
    z: i32,
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x, foo.y, foo.b as i32, foo.z);
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo {
        x: x & 0x3,       // 2-bit field
        y: y & 0x7,       // 3-bit field
        b,                // 1-bit boolean field
        z,
    };
    print_foo(&foo);
}

fn read_tokens() -> Vec<String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");
    input.split_whitespace().map(|s| s.to_string()).collect()
}

fn main() {
    // Mirror the C scanf calls by reading whitespace-separated tokens.
    let tokens = read_tokens();
    let mut it = tokens.into_iter();

    let x: u32 = it
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let y: u32 = it
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let b_int: i32 = it
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let z: i32 = it
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // !!b in C: convert any non-zero int into 1, zero stays 0.
    let b: bool = b_int != 0;

    driver(x, y, b, z);

    // Ensure stdout is flushed before exit.
    let _ = io::stdout().flush();
    // Suppress unused import warning for BufRead in some compiler versions.
    let _ = std::marker::PhantomData::<Box<dyn BufRead>>;
}
