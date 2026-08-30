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

//! Rust translation of `c_src` (`include/driver.h`, `src/driver.c`).
//!
//! Public ABI exported by the C shared library (`nm -D libdriver.so`):
//!   * `printLine`
//!   * `bad`
//!   * `good`
//!   * `driver`
//!
//! `helperBad` and `helperGood1` are `static` in the C translation unit and are
//! therefore *not* part of the exported ABI; they are reproduced here as private
//! Rust functions.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

extern "C" {
    /// The C code writes its output with `printf("%s\n", ...)`. We call straight
    /// into the platform C library so that the bytes emitted, and the way they
    /// interleave with any stdio buffering performed by the caller, match the
    /// original library exactly (a Rust `println!` would use a separate,
    /// independently flushed buffer).
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%s\n"` format string, NUL terminated, matching the C source literal.
static FMT_S_NEWLINE: [c_char; 4] = [b'%' as c_char, b's' as c_char, b'\n' as c_char, 0];

// ---------------------------------------------------------------------------
// void printLine(const char *line)
// ---------------------------------------------------------------------------
//
//     void printLine(const char *line)
//     {
//         if (line != NULL)
//         {
//             printf("%s\n", line);
//         }
//     }

/// # Safety
///
/// `line` must either be NULL or point to a NUL-terminated byte string that
/// remains readable for the duration of the call. This is exactly the contract
/// the C function imposes: it null-checks the pointer and otherwise hands it
/// straight to `printf("%s\n", ...)`, which reads until the terminator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(FMT_S_NEWLINE.as_ptr(), line);
    }
}

// ---------------------------------------------------------------------------
// static char *helperBad()
// ---------------------------------------------------------------------------
//
//     static char *helperBad()
//     {
//         char charString[] = "helperBad string";
//         return charString;
//     }
//
// This is the CWE-562 ("Return of Stack Variable Address") defect the original
// sample exists to demonstrate: `charString` is an automatic array whose
// lifetime ends when `helperBad` returns, so returning it yields a dangling
// pointer and the behaviour is undefined.
//
// The bug is deliberately NOT fixed here. It is reproduced with the exact
// observable behaviour of the compiled C library: GCC diagnoses the return of a
// local address (`-Wreturn-local-addr`) and emits `mov $0x0, %eax` for the
// return value, i.e. `helperBad` hands back a NULL pointer. Verified on the
// reference build, and confirmed to be independent of the optimization level:
// `tests/optlevels.rs` recompiles this exact C source at -O0, -O1, -O2, -O3 and
// -Os and asserts each build's observable output matches this translation. The
// -O0 disassembly of the CMake reference build:
//
//     000000000000115b <helperBad>:
//       ...
//       117f:  mov    $0x0,%eax        # returns NULL
//       1184:  pop    %rbp
//       1185:  ret
//
// Consequently `bad()` calls `printLine(NULL)`, which prints nothing at all --
// the string "helperBad string" never reaches the output. Returning a real
// pointer to a local buffer here (or, worse, a pointer to a static/leaked copy)
// would print a line the C library does not print and would break byte-for-byte
// output equivalence.
fn helperBad() -> *mut c_char {
    // The local `charString` is materialised exactly as the C compiler does
    // (the bytes are built up in the frame) and then discarded, because the
    // dead store is not observable and the returned value is NULL.
    let _charString: [c_char; 17] = {
        let mut buf = [0 as c_char; 17];
        let src = b"helperBad string\0";
        let mut i = 0;
        while i < src.len() {
            buf[i] = src[i] as c_char;
            i += 1;
        }
        buf
    };

    core::ptr::null_mut()
}

// ---------------------------------------------------------------------------
// void bad()
// ---------------------------------------------------------------------------
//
//     void bad()
//     {
//         printLine(helperBad());
//     }

/// # Safety
///
/// Takes no arguments and dereferences nothing the caller supplies, so there is
/// no precondition; it is `unsafe` only to keep the `extern "C"` signature
/// identical to the C symbol's.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    printLine(helperBad());
}

// ---------------------------------------------------------------------------
// static char *helperGood1()
// ---------------------------------------------------------------------------
//
//     static char *helperGood1()
//     {
//         static char charString[] = "helperGood1 string";
//         return charString;
//     }
//
// `charString` has static storage duration, so returning it is well defined.
// In the reference build it lives in `.data` (it is a mutable `char` array, not
// a string literal) and `helperGood1` returns its address.

/// Mutable, statically allocated counterpart of the C function's
/// `static char charString[] = "helperGood1 string";` (19 bytes including the
/// terminating NUL), placed in writable memory just like the C `.data` object.
static mut HELPER_GOOD1_CHAR_STRING: [c_char; 19] = {
    let mut buf = [0 as c_char; 19];
    let src = b"helperGood1 string\0";
    let mut i = 0;
    while i < src.len() {
        buf[i] = src[i] as c_char;
        i += 1;
    }
    buf
};

fn helperGood1() -> *mut c_char {
    // Decay of the static array to a pointer to its first element.
    (&raw mut HELPER_GOOD1_CHAR_STRING) as *mut c_char
}

// ---------------------------------------------------------------------------
// void good()
// ---------------------------------------------------------------------------
//
//     void good()
//     {
//         printLine(helperGood1());
//     }

/// # Safety
///
/// Takes no arguments and dereferences nothing the caller supplies, so there is
/// no precondition; it is `unsafe` only to keep the `extern "C"` signature
/// identical to the C symbol's.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    printLine(helperGood1());
}

// ---------------------------------------------------------------------------
// void driver(int useGood)
// ---------------------------------------------------------------------------
//
//     void driver(int useGood)
//     {
//         if (useGood)
//         {
//             good();
//         }
//         else
//         {
//             bad();
//         }
//     }

/// # Safety
///
/// Every `c_int` bit pattern is a valid input -- the C only tests `useGood != 0`
/// -- so there is no precondition beyond the C ABI itself.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    // C truthiness: any non-zero value (including negative ones) selects
    // `good()`; only exactly 0 selects `bad()`.
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
