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

//! Rust translation of the `String_Slice` C library (`c_src/src/slicing.c`).
//!
//! The single public entry point is [`slice`], which mirrors
//! `int slice(char *mystr, int *start_ptr, int *stop_ptr)` exactly, including
//! its diagnostic messages, its ordering of validation checks, and its quirks
//! (notably the signed/unsigned comparison of `int` bounds against the
//! `size_t` string length).
//!
//! Output is emitted through the platform C library's `printf` so that the
//! bytes written to `stdout` — and the stream buffering behaviour that governs
//! their interleaving with any other C output in the host process — are
//! byte-for-byte identical to the original library.

// The crate/library name mirrors the C target name (`libString_Slice.so`).
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    /// `size_t strlen(const char *s)`
    fn strlen(s: *const c_char) -> usize;

    /// `int printf(const char *restrict format, ...)`
    fn printf(format: *const c_char, ...) -> c_int;
}

// NUL-terminated format strings, byte-identical to the C source literals.
const ERR_START_OFF_END: &[u8] = b"Error: start is off the end of the string!\n\0";
const ERR_STOP_OFF_END: &[u8] = b"Error: stop is off the end of the string!\n\0";
const ERR_STOP_BEFORE_START: &[u8] = b"Error: stop must come after start!\n\0";
const FMT_SLICE: &[u8] = b"%.*s\n\0";

/// Writes a NUL-terminated literal through the C library's `printf`.
///
/// The literals above contain no `%` conversions, so this reproduces the
/// original `printf("...")` calls exactly.
#[inline]
fn print_literal(literal: &[u8]) {
    // SAFETY: `literal` is a `'static`, NUL-terminated byte string with no
    // format conversions.
    unsafe {
        printf(literal.as_ptr() as *const c_char);
    }
}

/// Index into a passed string and print the substring indexed by
/// `[*start_ptr, *stop_ptr)`.
///
/// If there is no start, use 0. If there is no stop, use the end of the string.
///
/// Returns `0` on success and `1` if a bound was rejected.
///
/// # Safety
///
/// `mystr` must be a valid pointer to a NUL-terminated string, and
/// `start_ptr` / `stop_ptr` must each be either null or point to a readable
/// `int`. These are exactly the requirements imposed by the C original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    // size_t len = strlen(mystr);
    let len: usize = unsafe { strlen(mystr) };

    let start: c_int;
    let stop: c_int;

    if !start_ptr.is_null() {
        // start = *start_ptr;
        start = unsafe { *start_ptr };

        // C: `start > len` compares an `int` against a `size_t`. The usual
        // arithmetic conversions promote `start` to `size_t`, so a negative
        // `start` wraps to a huge unsigned value and is rejected here. Casting
        // `c_int` to `usize` in Rust sign-extends first, reproducing this.
        if (start as usize) > len {
            print_literal(ERR_START_OFF_END);
            return 1;
        }
    } else {
        start = 0;
    }

    if !stop_ptr.is_null() {
        // stop = *stop_ptr;
        let requested_stop: c_int = unsafe { *stop_ptr };

        // Same `int` vs. `size_t` comparison quirk as for `start`.
        if (requested_stop as usize) > len {
            print_literal(ERR_STOP_OFF_END);
            return 1;
        }

        // This comparison, unlike the one above, is a plain signed `int`
        // comparison in the C source.
        if requested_stop <= start {
            print_literal(ERR_STOP_BEFORE_START);
            return 1;
        }

        stop = requested_stop;
    } else {
        // single-line else statement just to make style checking sad
        // `stop = len;` truncates `size_t` to `int`.
        stop = len as c_int;
    }

    // char arithmetic: skip ahead `start` characters in the array
    // printf("%.*s\n", stop - start, mystr + start);
    //
    // SAFETY: `start` is within `[0, len]` on every path that reaches here, so
    // `mystr + start` stays inside the string (one-past-the-end at worst), and
    // the precision `stop - start` never exceeds the remaining length.
    unsafe {
        printf(
            FMT_SLICE.as_ptr() as *const c_char,
            stop.wrapping_sub(start),
            mystr.wrapping_offset(start as isize),
        );
    }

    0
}
