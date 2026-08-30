// Rust translation of c_src/src/main.c
//
// Original copyright notice from the C source:
//
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

use std::io::{self, Write};

mod scanf;

use scanf::Scanner;

/// Mirrors the C bit-field struct:
///
/// ```c
/// typedef struct {
///     unsigned int x : 2;
///     unsigned int y : 3;
///     bool b : 1;
///     int z;
/// } foo_t;
/// ```
///
/// The narrow bit-field widths are emulated by masking on assignment, which is
/// what the C compiler does when storing into the bit-field members.
struct Foo {
    x: u32,
    y: u32,
    b: bool,
    z: i32,
}

impl Foo {
    /// Equivalent of `foo_t foo = {.x = x, .y = y, .b = b, .z = z};`
    fn new(x: u32, y: u32, b: bool, z: i32) -> Self {
        Foo {
            x: x & 0b11,       // unsigned int x : 2
            y: y & 0b111,      // unsigned int y : 3
            b,                 // bool b : 1  (already 0/1)
            z,
        }
    }
}

/// `printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);`
fn print_foo(out: &mut impl Write, foo: &Foo) {
    let _ = write!(
        out,
        "{} {} {} {}\n",
        foo.x,
        foo.y,
        if foo.b { 1 } else { 0 },
        foo.z
    );
}

fn driver(out: &mut impl Write, x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo::new(x, y, b, z);
    print_foo(out, &foo);
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());

    // Same initial values as the C code; a failed/EOF conversion leaves the
    // variable untouched, exactly as scanf() does.
    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    // scanf("%u", &x);
    if let Some(v) = scanner.scan_u32() {
        x = v;
    }
    // scanf("%u", &y);
    if let Some(v) = scanner.scan_u32() {
        y = v;
    }
    // scanf("%d", &b);
    if let Some(v) = scanner.scan_i32() {
        b = v;
    }
    // scanf("%d", &z);
    if let Some(v) = scanner.scan_i32() {
        z = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, x, y, b != 0, z); // driver(x, y, !!b, z)
    let _ = out.flush();
}
