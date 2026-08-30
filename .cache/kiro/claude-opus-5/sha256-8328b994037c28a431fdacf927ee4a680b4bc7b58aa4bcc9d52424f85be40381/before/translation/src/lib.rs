// Rust translation of c_src/src/driver.c
//
// Original copyright notice from the C sources:
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

#![allow(non_snake_case)]

use std::ffi::c_char;
use std::ffi::c_int;

// Use the platform's C `printf` so that output formatting, encoding and stdio
// buffering behaviour are byte-for-byte identical to the original C library
// (including any interleaving with other C stdio output in the same process).
extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%s\n\0"` format string used by `printLine`.
const FMT_STR: [c_char; 4] = [b'%' as c_char, b's' as c_char, b'\n' as c_char, 0];
/// `"%d\n\0"` format string used by `printIntLine`.
const FMT_INT: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// Number of `int` slots in `buffer` in the original C code.
const BUFFER_LEN: usize = 10;

/// Extra slots kept behind `buffer` so that the deliberate out-of-bounds write
/// reproduced from the C code stays inside an allocation we own. The C program
/// scribbles over whatever happens to follow `buffer` on the stack; the visible
/// output (only `buffer[0..10]` is ever printed) is unaffected as long as the
/// write lands past the array, which is what this slack provides.
const BUFFER_SLACK: usize = 4096;

// -----------------------------------------------------------------------------
// void printLine(const char * line)
// -----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(FMT_STR.as_ptr(), line);
    }
}

// -----------------------------------------------------------------------------
// void printIntLine(int intNumber)
// -----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    printf(FMT_INT.as_ptr(), intNumber);
}

/// Helper for the `printLine("literal")` calls: the literals are NUL-terminated
/// byte strings, so their pointers can be handed straight to `printLine`.
unsafe fn print_line_lit(bytes: &[u8]) {
    debug_assert_eq!(bytes.last().copied(), Some(0));
    printLine(bytes.as_ptr() as *const c_char);
}

/// Emulates `buffer[index] = 1;` on a 10-element `int` array without bounds
/// checking, exactly as the C code does.
unsafe fn store_one(backing: &mut [c_int], index: usize) {
    // Within the array (or within the slack that stands in for the adjacent
    // stack slots the C code would clobber) the write is performed in place.
    if index < backing.len() {
        *backing.get_unchecked_mut(index) = 1;
    } else {
        // Beyond the slack the C program would be writing far outside its stack
        // frame; reproduce the raw, unchecked store the C code performs.
        std::ptr::write(backing.as_mut_ptr().add(index), 1);
    }
}

/// Prints `buffer[0..10]`, matching the C loop.
unsafe fn print_buffer(backing: &[c_int]) {
    for i in 0..BUFFER_LEN {
        printIntLine(backing[i]);
    }
}

// -----------------------------------------------------------------------------
// void bad(int data)
// -----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad(data: c_int) {
    let mut backing = [0 as c_int; BUFFER_LEN + BUFFER_SLACK];
    // NOTE: the original C code only checks `data >= 0`; the missing upper
    // bound check (the injected defect) is preserved verbatim.
    if data >= 0 {
        store_one(&mut backing, data as usize);
        /* Print the array values */
        print_buffer(&backing);
    } else {
        print_line_lit(b"ERROR: Array index is negative.\0");
    }
}

// -----------------------------------------------------------------------------
// static void goodG2B()
// -----------------------------------------------------------------------------
unsafe fn good_g2b() {
    let data: c_int = 7;
    let mut backing = [0 as c_int; BUFFER_LEN + BUFFER_SLACK];
    if data >= 0 {
        store_one(&mut backing, data as usize);
        /* Print the array values */
        print_buffer(&backing);
    } else {
        print_line_lit(b"ERROR: Array index is negative.\0");
    }
}

// -----------------------------------------------------------------------------
// static void goodB2G(int data)
// -----------------------------------------------------------------------------
unsafe fn good_b2g(data: c_int) {
    let mut backing = [0 as c_int; BUFFER_LEN + BUFFER_SLACK];
    if data >= 0 && data < (BUFFER_LEN as c_int) {
        store_one(&mut backing, data as usize);
        /* Print the array values */
        print_buffer(&backing);
    } else {
        print_line_lit(b"ERROR: Array index is out-of-bounds\0");
    }
}

// -----------------------------------------------------------------------------
// void good(int data)
// -----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

// -----------------------------------------------------------------------------
// void driver(int goodData, int badData)
// -----------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(goodData: c_int, badData: c_int) {
    print_line_lit(b"Calling good()...\0");
    good(goodData);
    print_line_lit(b"Finished good()\0");
    print_line_lit(b"Calling bad()...\0");
    bad(badData);
    print_line_lit(b"Finished bad()\0");
}
