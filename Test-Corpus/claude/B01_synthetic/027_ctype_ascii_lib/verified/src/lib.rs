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

//! Rust translation of `c_src/` — the `driver` shared library.
//!
//! The C build globs the whole of `c_src/` into one shared object.  That is a
//! single translation unit, `src/driver.c`, whose only public header is
//! `include/driver.h`, declaring one function:
//!
//! ```c
//! void driver(char c);
//! ```
//!
//! `nm -D` on the reference `libdriver.so` confirms `driver` is the complete
//! exported public ABI.  There are no namespace/renaming macros in the public
//! header, so the linker symbol is plainly `driver`.

mod ctype;
mod ffi;

use core::ffi::{c_char, c_int};

/// Format strings, byte for byte as they appear in `driver.c`, each with the
/// implicit C string terminator appended.
mod fmt {
    pub const ALPHANUMERIC: &[u8] = b"alphanumeric: %d\n\0";
    pub const ALPHABETIC: &[u8] = b"alphabetic: %d\n\0";
    pub const LOWERCASE: &[u8] = b"lowercase: %d\n\0";
    pub const UPPERCASE: &[u8] = b"uppercase: %d\n\0";
    pub const DIGIT: &[u8] = b"digit: %d\n\0";
    pub const HEXADECIMAL: &[u8] = b"hexadecimal: %d\n\0";
    pub const CONTROL: &[u8] = b"control: %d\n\0";
    pub const GRAPHICAL: &[u8] = b"graphical: %d\n\0";
    pub const SPACE: &[u8] = b"space: %d\n\0";
    pub const BLANK: &[u8] = b"blank: %d\n\0";
    pub const PRINTING: &[u8] = b"printing: %d\n\0";
    pub const PUNCTUATION: &[u8] = b"punctuation: %d\n\0";
    pub const TO_LOWER: &[u8] = b"to lower: %c\n\0";
    pub const TO_UPPER: &[u8] = b"to upper: %c\n\0";
}

/// The `"C"` locale name passed to `setlocale`.
const LOCALE_C: &[u8] = b"C\0";

/// Emits one `printf("<label>: %d\n", value)` line.
fn print_int(format: &[u8], value: c_int) {
    // SAFETY: `format` is a `'static` NUL-terminated byte literal from `fmt`
    // containing exactly one `%d`, matched here by a single `c_int` argument.
    unsafe {
        ffi::printf(format.as_ptr() as *const c_char, value);
    }
}

/// Emits one `printf("<label>: %c\n", value)` line.
///
/// `%c` makes `printf` convert the `int` argument to `unsigned char`, so a
/// negative conversion-table result is printed as its low byte — matching the C
/// library for `char` values that sign-extend to a negative index.
fn print_char(format: &[u8], value: c_int) {
    // SAFETY: `format` is a `'static` NUL-terminated byte literal from `fmt`
    // containing exactly one `%c`, matched here by a single `c_int` argument,
    // which is the type `%c` consumes after default argument promotion.
    unsafe {
        ffi::printf(format.as_ptr() as *const c_char, value);
    }
}

/// Translation of `void driver(char c)` from `c_src/src/driver.c`.
///
/// # Why the parameter is `c_int` and not `c_char`
///
/// The C prototype is `void driver(char c)`, and on every C ABI a `char`
/// argument is *delivered* in an `int`-sized register or stack slot, of which
/// the callee reads only the low byte.  GCC compiles the C callee's parameter
/// to exactly that, at every optimisation level:
///
/// ```text
/// mov  %edi,%eax
/// mov  %al,-0x4(%rbp)     ; only the low 8 bits are ever stored
/// movsbq -0x4(%rbp),%rax  ; ...and sign-extended from that byte
/// ```
///
/// So `driver` provably observes nothing but the low 8 bits of whatever the
/// caller passed, and a caller that passes `256` through a `void driver(int)`
/// prototype gets `driver('\0')`'s behaviour.
///
/// Declaring the Rust parameter as `c_char` instead attaches LLVM's `signext`
/// attribute, which entitles an **optimised** build to assume the caller
/// already sign-extended the byte and to use the raw 32-bit register as the
/// `<ctype.h>` table index.  It does: a release build compiled
/// `mov %edi,%ebx; movslq %ebx,%rbx`, so `driver(256)` indexed `table[256]` —
/// outside the `-128 ..= 255` range glibc's tables are defined over.  That is
/// both an out-of-bounds read and a visible divergence from the C library
/// (`control: 0` where C prints `control: 2`).
///
/// Accepting the widened `c_int` and narrowing it here reproduces the C
/// callee's `mov %al` for *every* possible argument, and is ABI-identical for
/// callers that use the correct `char` prototype.
#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_int) {
    driver_impl(c as u8 as c_char)
}

/// The body of `driver`, in terms of the `char` the C source declares.
///
/// The statement order, the format strings and the `<ctype.h>` interface used
/// on each line are preserved exactly as written in the C source.
fn driver_impl(c: c_char) {
    // SAFETY: `LOCALE_C` is a `'static` NUL-terminated byte literal, and
    // `setlocale` only reads it.  The returned string is discarded, exactly as
    // the C code discards it.
    unsafe {
        ffi::setlocale(ffi::LC_ALL, LOCALE_C.as_ptr() as *const c_char);
    }

    print_int(fmt::ALPHANUMERIC, ctype::isalnum(c));
    print_int(fmt::ALPHABETIC, ctype::isalpha(c));
    print_int(fmt::LOWERCASE, ctype::islower(c));
    print_int(fmt::UPPERCASE, ctype::isupper(c));
    print_int(fmt::DIGIT, ctype::isdigit(c));
    print_int(fmt::HEXADECIMAL, ctype::isxdigit(c));
    print_int(fmt::CONTROL, ctype::iscntrl(c));
    print_int(fmt::GRAPHICAL, ctype::isgraph(c));
    print_int(fmt::SPACE, ctype::isspace(c));
    print_int(fmt::BLANK, ctype::isblank(c));
    print_int(fmt::PRINTING, ctype::isprint(c));
    print_int(fmt::PUNCTUATION, ctype::ispunct(c));
    print_char(fmt::TO_LOWER, ctype::tolower(c));
    print_char(fmt::TO_UPPER, ctype::toupper(c));
}
