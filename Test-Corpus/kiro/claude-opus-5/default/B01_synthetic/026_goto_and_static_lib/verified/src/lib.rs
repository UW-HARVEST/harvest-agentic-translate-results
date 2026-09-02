// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// Original C license header reproduced for provenance:
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

use core::ffi::{c_char, c_int};
use core::sync::atomic::{AtomicI32, Ordering};

// The original uses C `printf`, so route all output through libc's `printf`.
// This keeps stdio buffering, ordering and interleaving with any other C code
// in the process byte-for-byte identical to the C library.
unsafe extern "C" {
    #[link_name = "printf"]
    fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// `printf` with a literal format string that contains no conversions.
fn print_lit(s: &core::ffi::CStr) {
    unsafe {
        c_printf(s.as_ptr());
    }
}

/// File-scope `static int y = 123;` from driver.c.
///
/// Modelled as an atomic so the translation needs no `static mut`; the C code is
/// single-threaded and always assigns `y` before reading it, so `Relaxed`
/// accesses reproduce the original semantics exactly.
static Y: AtomicI32 = AtomicI32::new(123);

/// `static int multi_stage(int x, int z)` — internal, not part of the ABI.
///
/// The C control flow is a chain of guards that `goto fail`; the fail path
/// prints "Operation failed" and returns the code, while the success path
/// returns without printing it. Check order is preserved exactly.
fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;

    'fail: {
        if x != 1 {
            print_lit(c"Error: x != 1\n");
            result = 1;
            break 'fail;
        }

        if Y.load(Ordering::Relaxed) != 2 {
            print_lit(c"Error: x == 1 but y != 2\n");
            result = 2;
            break 'fail;
        }

        if z != 3 {
            print_lit(c"Error: x == 1 and y == 2, but z != 3\n");
            result = 3;
            break 'fail;
        }

        print_lit(c"Ok!\n");
        return result;
    }

    // fail:
    print_lit(c"Operation failed\n");
    result
}

/// `void driver(int x, int local_y, int z)` — the library's only public symbol.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    Y.store(local_y, Ordering::Relaxed);
    let result = multi_stage(x, z);
    unsafe {
        c_printf(c"Result: %d\n".as_ptr(), result);
    }
}
