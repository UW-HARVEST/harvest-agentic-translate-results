//! Translation of `c_src/src/goto.c`.
//!
//! The original file demonstrates C `goto`-based control flow. Rust has no
//! `goto`, so each jump is re-expressed with the equivalent structured control
//! flow. The observable behaviour — return values, stream writes, and the order
//! in which they happen — is preserved exactly, including the original's quirks
//! (see the notes on `open_with_cleanup` below).

use std::ffi::{c_char, c_int};
use std::ptr;

use crate::cstdio::{FILE, fclose, ferror, fgets, fopen, fprintf, printf, stderr};

/// Size of the read buffer in `open_with_cleanup`, matching `char buffer[100]`.
const BUFFER_LEN: usize = 100;

/// `int forward_goto_example(int x)`
///
/// C source:
///
/// ```c
/// if (x < 0) {
///   goto error;
/// }
/// printf("Processing: %d\n", x);
/// return x * 2;
///
/// error:
///   fprintf(stderr, "Error: negative input\n");
///   return -1;
/// ```
///
/// The `error` label is only reachable via the explicit `goto` (the success path
/// returns before falling into it), so the forward jump collapses into a plain
/// early-return branch.
#[unsafe(no_mangle)]
pub extern "C" fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        // error:
        unsafe {
            fprintf(stderr, c"Error: negative input\n".as_ptr());
        }
        return -1;
    }

    unsafe {
        printf(c"Processing: %d\n".as_ptr(), x);
    }

    // `x * 2` overflows for large `x`, which is undefined behaviour in C but in
    // practice compiles to a wrapping multiply. Spell that out explicitly so the
    // Rust build does not panic in debug or miscompile in release.
    x.wrapping_mul(2)
}

/// `FILE* open_with_cleanup(const char *filename)`
///
/// C source:
///
/// ```c
/// FILE* fp = fopen(filename, "r");
/// if (!fp) {
///   goto cleanup;
/// }
/// char buffer[100];
/// while (fgets(buffer, sizeof(buffer), fp)) {
///     printf("%s", buffer);
/// }
/// if (ferror(fp)) {
///     goto cleanup;
/// }
/// return fp;
///
/// cleanup:
///   fprintf(stderr, "Error: opening or processing file %s\n", filename);
///   if(fp) fclose(fp);
///   return NULL;
/// ```
///
/// Behaviour preserved verbatim, including two oddities of the original that are
/// deliberately **not** fixed:
///
/// * On success the file has already been read to EOF but is returned still
///   open, leaving the caller to close it.
/// * The `cleanup` label is shared by both failure paths, so the `if (fp)` guard
///   is a no-op on the `fopen`-failed path and a real `fclose` on the
///   `ferror` path.
///
/// # Safety
///
/// `filename` is forwarded to `fopen` and to `fprintf`'s `%s` conversion exactly
/// as the C code does; it must be a valid NUL-terminated string (a null pointer
/// is passed straight through, matching the original).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut FILE {
    let fp: *mut FILE = unsafe { fopen(filename, c"r".as_ptr()) };

    if !fp.is_null() {
        // C leaves `buffer` uninitialized; `fgets` NUL-terminates whatever it
        // stores, so zeroing here is observationally equivalent.
        let mut buffer: [c_char; BUFFER_LEN] = [0; BUFFER_LEN];

        while !unsafe { fgets(buffer.as_mut_ptr(), BUFFER_LEN as c_int, fp) }.is_null() {
            unsafe {
                printf(c"%s".as_ptr(), buffer.as_ptr());
            }
        }

        if unsafe { ferror(fp) } == 0 {
            return fp;
        }
        // otherwise: fall through to cleanup with `fp` still non-null
    }

    // cleanup:
    unsafe {
        fprintf(
            stderr,
            c"Error: opening or processing file %s\n".as_ptr(),
            filename,
        );
    }
    if !fp.is_null() {
        unsafe {
            fclose(fp);
        }
    }
    ptr::null_mut()
}

/// `int driver(int num, const char* filename)`
///
/// The library's documented entry point (the only symbol declared in
/// `include/goto.h`). Returns `-1` if `forward_goto_example` reported an error,
/// `-2` if the file could not be opened or read, and `0` on success.
///
/// # Safety
///
/// `filename` is forwarded to [`open_with_cleanup`]; the same requirements apply.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    let res = forward_goto_example(num);
    if res == -1 {
        return -1;
    } else {
        unsafe {
            printf(c"Goto output: %d\n".as_ptr(), res);
        }
    }

    let out: *mut FILE = unsafe { open_with_cleanup(filename) };
    if out.is_null() {
        return -2;
    } else {
        unsafe {
            fclose(out);
        }
    }

    0
}
