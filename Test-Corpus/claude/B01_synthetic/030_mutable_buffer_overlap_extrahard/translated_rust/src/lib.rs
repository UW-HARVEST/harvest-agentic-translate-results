// Library crate exposing C-ABI wrappers around the translated functions, so
// that integration tests can dlopen the Rust .so and call them through the
// FFI boundary, exactly as an external (C) caller would.

use std::os::raw::{c_int, c_void};

/// Pure-Rust translation of `fma_array` from c_src/src/main.c.
///
/// Mirrors the C semantics, including signed-integer wrap (the C uses int).
/// Note: the C code intentionally calls this with the SAME pointer for all
/// of `out`, `mul1`, `mul2`, and `add`. We replicate that aliasing behavior
/// here by reading and writing element-by-element.
///
/// # Safety
/// `out`, `mul1`, `mul2`, and `add` must each point to at least `len` valid
/// `i32` elements. They may all alias the same buffer (as the C does).
#[no_mangle]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let n = len as isize;
    let mut i: isize = 0;
    while i < n {
        let a = *mul1.offset(i);
        let b = *mul2.offset(i);
        let c = *add.offset(i);
        *out.offset(i) = a.wrapping_mul(b).wrapping_add(c);
        i += 1;
    }
}

extern "C" {
    fn printf(fmt: *const std::os::raw::c_char, ...) -> c_int;
}

/// Pure-Rust translation of `driver` from c_src/src/main.c.
///
/// Calls `fma_array(out, out, out, out, len)` and then prints each element
/// via `printf("%d\n", out[i])` — using libc's printf so output goes through
/// the same C stdio stream as the original C, allowing fd-1 redirection to
/// capture identical bytes from either library.
///
/// # Safety
/// `out` must point to at least `len` valid `i32` elements.
#[no_mangle]
pub unsafe extern "C" fn driver(out: *mut c_int, len: c_int) {
    fma_array(out, out, out, out, len);
    let fmt = b"%d\n\0".as_ptr() as *const std::os::raw::c_char;
    let n = len as isize;
    let mut i: isize = 0;
    while i < n {
        printf(fmt, *out.offset(i));
        i += 1;
    }
    // Flush so the captured fd-1 output matches what the C caller sees.
    extern "C" {
        fn fflush(stream: *mut c_void) -> c_int;
    }
    fflush(std::ptr::null_mut());
}

/// Mirror of the C `main`. Returns `0` on success. Reads up to 100 ints from
/// stdin via `scanf("%d", ...)`, then calls `driver`.
///
/// Exported so the Rust .so exposes the same symbol set as the C .so.
/// Suppressed under `cfg(test)` to avoid clashing with the test harness's
/// own entry point.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    extern "C" {
        fn scanf(fmt: *const std::os::raw::c_char, ...) -> c_int;
    }
    let mut data: [c_int; 100] = [0; 100];
    let fmt = b"%d\0".as_ptr() as *const std::os::raw::c_char;
    let mut i: usize = 0;
    while i < 100 {
        if scanf(fmt, &mut data[i] as *mut c_int) != 1 {
            break;
        }
        i += 1;
    }
    driver(data.as_mut_ptr(), i as c_int);
    0
}

// Note: the C .so built by `gcc -shared` exports `_init` and `_fini` ELF
// init/fini-section stubs from crti.o/crtn.o. They're not user code — they
// are toolchain-emitted dynamic-linker hooks. The Rust cdylib build uses a
// different runtime layout (linker-generated DT_INIT/DT_FINI may be absent
// or under different names) and does not emit `_init`/`_fini` as text
// symbols. Defining them in Rust collides with crti.o's definitions, so we
// leave them to whatever the cdylib's link recipe produces. Every
// user-code symbol from the C .so (`fma_array`, `driver`, `main`) is
// exported by the Rust .so.


