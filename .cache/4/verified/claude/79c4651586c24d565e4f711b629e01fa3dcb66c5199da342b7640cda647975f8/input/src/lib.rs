// Rust translation of c_src/ (MIT Lincoln Laboratory `driver` library).
//
// Public ABI exported by the C shared library (per `nm -D`):
//     driver  -- void driver(int x, int y, int z)
//
// Everything else in the C translation unit (`static int y`,
// `static int multi_stage(int, int)`) has internal linkage and therefore is
// *not* part of the exported ABI; it is reproduced here as private Rust items.
//
// All console output is emitted through the C runtime's `printf` so that the
// byte stream *and* the stdio buffering/interleaving behaviour are identical to
// the original C library.

use core::ffi::{c_char, c_int};
use core::sync::atomic::{AtomicI32, Ordering};

unsafe extern "C" {
    /// C standard library `printf`.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Emit a nul-terminated literal exactly as `printf("<literal>")` would.
#[inline]
fn c_print(s: &core::ffi::CStr) {
    // SAFETY: `s` is a valid nul-terminated C string and contains the only
    // conversion specifications the corresponding C call site had (none).
    unsafe {
        printf(s.as_ptr());
    }
}

/// Translation of `static int y = 123;` in c_src/src/driver.c.
///
/// An atomic is used so the mutable module-level state can be touched from safe
/// Rust; with `Ordering::Relaxed` accesses it behaves exactly like the plain C
/// `int` for the single-threaded use the C code exhibits.
static Y: AtomicI32 = AtomicI32::new(123);

/// Translation of `static int multi_stage(int x, int z)`.
///
/// The original uses `goto fail;` for the three error paths; the error ordering
/// and the exact return values / messages are preserved verbatim.
fn multi_stage(x: c_int, z: c_int) -> c_int {
    let mut result: c_int = 0;

    // if (x != 1) { ... result = 1; goto fail; }
    if x != 1 {
        c_print(c"Error: x != 1\n");
        result = 1;
        // fail:
        c_print(c"Operation failed\n");
        return result;
    }

    // if (y != 2) { ... result = 2; goto fail; }
    if Y.load(Ordering::Relaxed) != 2 {
        c_print(c"Error: x == 1 but y != 2\n");
        result = 2;
        // fail:
        c_print(c"Operation failed\n");
        return result;
    }

    // if (z != 3) { ... result = 3; goto fail; }
    if z != 3 {
        c_print(c"Error: x == 1 and y == 2, but z != 3\n");
        result = 3;
        // fail:
        c_print(c"Operation failed\n");
        return result;
    }

    c_print(c"Ok!\n");
    result
}

/// `void driver(int x, int local_y, int z)` -- the sole public entry point.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, local_y: c_int, z: c_int) {
    Y.store(local_y, Ordering::Relaxed);
    let result = multi_stage(x, z);
    // SAFETY: the format string is a valid nul-terminated C string whose single
    // `%d` conversion is matched by the `c_int` argument that follows.
    unsafe {
        printf(c"Result: %d\n".as_ptr(), result);
    }
}
