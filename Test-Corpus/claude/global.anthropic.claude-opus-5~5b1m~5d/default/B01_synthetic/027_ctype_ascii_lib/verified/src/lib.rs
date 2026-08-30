// Rust translation of the C library found in `c_src/`.
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
//
// The complete public ABI of the C library consists of a single exported
// symbol, `driver` (see `c_src/include/driver.h`).  There are no
// namespace-renaming preprocessor macros in the public header, so the linker
// symbol is plain `driver`.

mod ctype;

use core::ffi::{c_char, c_int};

// `LC_ALL` as defined by glibc's <locale.h> on Linux.
const LC_ALL: c_int = 6;

unsafe extern "C" {
    // The C source prints through the C runtime's stdio.  Re-using `printf`
    // keeps the buffering behaviour (and therefore the exact byte stream and
    // its interleaving with any other C output) identical to the original.
    fn printf(format: *const c_char, ...) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

/// `"C"` — the locale name passed to `setlocale`, NUL terminated.
static LOCALE_C: [c_char; 2] = [b'C' as c_char, 0];

/// Format strings, NUL terminated, matching `c_src/src/driver.c` verbatim.
static FMT_ALNUM: &[u8] = b"alphanumeric: %d\n\0";
static FMT_ALPHA: &[u8] = b"alphabetic: %d\n\0";
static FMT_LOWER: &[u8] = b"lowercase: %d\n\0";
static FMT_UPPER: &[u8] = b"uppercase: %d\n\0";
static FMT_DIGIT: &[u8] = b"digit: %d\n\0";
static FMT_XDIGIT: &[u8] = b"hexadecimal: %d\n\0";
static FMT_CNTRL: &[u8] = b"control: %d\n\0";
static FMT_GRAPH: &[u8] = b"graphical: %d\n\0";
static FMT_SPACE: &[u8] = b"space: %d\n\0";
static FMT_BLANK: &[u8] = b"blank: %d\n\0";
static FMT_PRINT: &[u8] = b"printing: %d\n\0";
static FMT_PUNCT: &[u8] = b"punctuation: %d\n\0";
static FMT_TO_LOWER: &[u8] = b"to lower: %c\n\0";
static FMT_TO_UPPER: &[u8] = b"to upper: %c\n\0";

#[inline]
fn print_int(fmt: &[u8], value: c_int) {
    unsafe {
        printf(fmt.as_ptr() as *const c_char, value);
    }
}

/// `void driver(char c)`
///
/// Classifies `c` with every `<ctype.h>` predicate and prints the results, then
/// prints the lower- and upper-cased forms of `c`.
///
/// Note that the C code passes a (potentially negative, since `char` is signed
/// on this platform) `char` straight into the `is*()` macros; glibc's tables
/// cover the `-128 ..= 255` index range so this is well defined and the
/// classification bits for negative indices are all zero.  That behaviour is
/// reproduced exactly rather than "fixed".
#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    unsafe {
        setlocale(LC_ALL, LOCALE_C.as_ptr());
    }

    // The classification macros index glibc's tables with `(int) c`, keeping the
    // sign of the `char`.  Since the reachable index range is exactly the 256
    // bit patterns of one byte, the tables in `ctype` are keyed by that byte;
    // see the module comment there for why this truncation is both faithful to
    // the C and necessary to keep an optimised build in bounds.
    let cb: u8 = c as u8;

    print_int(FMT_ALNUM, ctype::isalnum(cb));
    print_int(FMT_ALPHA, ctype::isalpha(cb));
    print_int(FMT_LOWER, ctype::islower(cb));
    print_int(FMT_UPPER, ctype::isupper(cb));
    print_int(FMT_DIGIT, ctype::isdigit(cb));
    print_int(FMT_XDIGIT, ctype::isxdigit(cb));
    print_int(FMT_CNTRL, ctype::iscntrl(cb));
    print_int(FMT_GRAPH, ctype::isgraph(cb));
    print_int(FMT_SPACE, ctype::isspace(cb));
    print_int(FMT_BLANK, ctype::isblank(cb));
    print_int(FMT_PRINT, ctype::isprint(cb));
    print_int(FMT_PUNCT, ctype::ispunct(cb));
    // `%c` narrows the `int` argument to `unsigned char`, so a negative value
    // reproduces the original byte.
    print_int(FMT_TO_LOWER, ctype::tolower(cb));
    print_int(FMT_TO_UPPER, ctype::toupper(cb));
}
