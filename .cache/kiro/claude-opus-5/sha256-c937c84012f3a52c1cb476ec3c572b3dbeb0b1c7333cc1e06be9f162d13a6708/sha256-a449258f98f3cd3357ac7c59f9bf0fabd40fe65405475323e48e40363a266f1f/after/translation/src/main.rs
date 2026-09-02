// Translation of c_src/src/main.c
//
// Original notice from the C source:
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

mod cfmt;
mod cscan;

use std::io::Write;

/// Mirrors:
///
/// ```c
/// typedef union { uint64_t x; double f; } raw_double_t;
///
/// void driver(double f) {
///     raw_double_t u = {.f = f};
///     printf("%llx %a %.4f\n", u.x, f, f);
/// }
/// ```
fn driver(f: f64) {
    // The union member `x` aliases the object representation of the double.
    let x: u64 = f.to_bits();
    let out = format!(
        "{} {} {}\n",
        cfmt::format_llx(x),
        cfmt::format_a(f),
        cfmt::format_f(f, 4)
    );
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    // Ignore write errors, exactly like the C code ignores printf's return value.
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}

fn main() {
    // double f = 0.0f;
    let mut f: f64 = 0.0;

    // scanf("%lf", &f);  -- on matching/input failure `f` is left untouched.
    let stdin = std::io::stdin();
    let mut reader = cscan::Reader::new(stdin.lock());
    if let Some(v) = cscan::scan_lf(&mut reader) {
        f = v;
    }

    driver(f);

    // return 0;
}
