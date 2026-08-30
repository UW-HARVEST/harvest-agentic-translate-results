// Rust translation of c_src/src/driver.c
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

use std::ffi::{CStr, c_char, c_int};
use std::sync::atomic::{AtomicI32, Ordering};

// The original C uses the platform `stdio` stream for output. Going through
// `printf` (rather than Rust's `std::io::stdout`) keeps the buffering and
// flushing behaviour byte-for-byte identical to the C library, including when
// the output is interleaved with writes performed by a C caller.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `printf("%s", s)` for a fixed, NUL-terminated string.
fn print_str(s: &CStr) {
    unsafe {
        printf(c"%s".as_ptr(), s.as_ptr());
    }
}

/// `printf("<prefix>%d\n", value)`.
fn print_int(fmt: &CStr, value: c_int) {
    unsafe {
        printf(fmt.as_ptr(), value);
    }
}

// `static int y = 123;` — file-scope mutable state in the C translation unit.
static Y: AtomicI32 = AtomicI32::new(123);

// `static int multi_stage(int x, int z)` — internal linkage, so no `no_mangle`.
fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;

    // The C body uses `goto fail` for the three failure paths; the closure below
    // reproduces the identical check order and the shared failure epilogue.
    let failed = loop {
        if x != 1 {
            print_str(c"Error: x != 1\n");
            result = 1;
            break true;
        }

        if Y.load(Ordering::Relaxed) != 2 {
            print_str(c"Error: x == 1 but y != 2\n");
            result = 2;
            break true;
        }

        if z != 3 {
            print_str(c"Error: x == 1 and y == 2, but z != 3\n");
            result = 3;
            break true;
        }

        print_str(c"Ok!\n");
        break false;
    };

    if failed {
        // fail:
        print_str(c"Operation failed\n");
    }

    result
}

// `void driver(int x, int local_y, int z)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    Y.store(local_y, Ordering::Relaxed);
    let result = multi_stage(x, z);
    print_int(c"Result: %d\n", result);
}
