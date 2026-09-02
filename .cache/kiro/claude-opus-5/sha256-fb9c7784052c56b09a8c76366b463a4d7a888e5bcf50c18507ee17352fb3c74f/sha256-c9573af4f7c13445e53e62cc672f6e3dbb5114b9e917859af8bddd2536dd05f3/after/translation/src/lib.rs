// Rust translation of c_src/src/driver.c (public header: c_src/include/driver.h)
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
//
// ---------------------------------------------------------------------------
// Fidelity notes
// ---------------------------------------------------------------------------
// `driver` takes a plain `char`, which is *signed* on the target platform, so
// values 0x80..0xFF arrive as negative indices.
//
// The C compiles the twelve `isXXX(c)` calls from glibc's <ctype.h>, where each
// is the macro
//
//     #define __isctype(c, type) \
//         ((*__ctype_b_loc ())[(int) (c)] & (unsigned short int) type)
//
// Two consequences are reproduced here verbatim:
//
//   1. The result is the *raw masked bit*, not a normalised 0/1. For example
//      `iscntrl('\0')` yields 2 (`_IScntrl`) and `isdigit('0')` yields 2048
//      (`_ISdigit`), and those are the integers `printf("%d")` renders.
//   2. The table is read *live*, through `__ctype_b_loc()`, on every call. It is
//      therefore whatever locale is in effect at that instant. Freezing a copy
//      of the "C"-locale table into this crate would be wrong: `driver` calls
//      `setlocale(LC_ALL, "C")`, which sets the *global* locale, and that has no
//      effect on a thread that installed a locale with `uselocale()`. In that
//      state the C classifies according to the thread locale (e.g. byte 0x80 is
//      `_IScntrl` under `de_DE.ISO-8859-1`), so this translation calls
//      `__ctype_b_loc()` exactly where the C macro does.
//
// `tolower`/`toupper` are real function calls in the C (they appear as undefined
// imports in the C `.so`), and glibc resolves them against the same live locale.
// They are likewise called through libc rather than reimplemented, which keeps
// locale-specific mappings identical — including the Turkish case, where
// `tolower('I')` is `'I'` under `tr_TR` because dotless `ı` is not a single byte.
//
// The `char` argument is passed to each of them as C does: promoted to `int`
// with sign extension, so `(char) 0xFF` arrives as `-1`, which is also `EOF` and
// has its own slot in glibc's tables.
//
// Output is emitted through libc's `printf` rather than Rust's own `std::io`
// stack so that the bytes land in the very same `stdout` FILE buffer a C caller
// uses, preserving exact interleaving and flush ordering.
//
// No bugs are corrected: the ctype calls receive the raw `char` exactly as the C
// passes it, the return value of `setlocale` is discarded exactly as the C
// discards it, and the order and format strings of every `printf` are preserved.

use core::ffi::{c_char, c_int, c_ushort};

// ---------------------------------------------------------------------------
// libc bindings (the same entry points the C translation unit resolves against)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;

    /// Backing accessor for glibc's `isXXX` macros. Returns a pointer to a
    /// pointer into the middle of the current locale's classification table;
    /// the table is legally addressable over `-128..=255`.
    fn __ctype_b_loc() -> *mut *const c_ushort;

    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
}

/// `LC_ALL` from glibc's <locale.h>.
const LC_ALL: c_int = 6;

// ---------------------------------------------------------------------------
// glibc <ctype.h> `_ISbit` masks
//   _ISbit(bit) = (bit) < 8 ? ((1 << (bit)) << 8) : ((1 << (bit)) >> 8)
// ---------------------------------------------------------------------------

const IS_UPPER: c_ushort = 256; // _ISupper  = _ISbit(0)
const IS_LOWER: c_ushort = 512; // _ISlower  = _ISbit(1)
const IS_ALPHA: c_ushort = 1024; // _ISalpha  = _ISbit(2)
const IS_DIGIT: c_ushort = 2048; // _ISdigit  = _ISbit(3)
const IS_XDIGIT: c_ushort = 4096; // _ISxdigit = _ISbit(4)
const IS_SPACE: c_ushort = 8192; // _ISspace  = _ISbit(5)
const IS_PRINT: c_ushort = 16384; // _ISprint  = _ISbit(6)
const IS_GRAPH: c_ushort = 32768; // _ISgraph  = _ISbit(7)
const IS_BLANK: c_ushort = 1; // _ISblank  = _ISbit(8)
const IS_CNTRL: c_ushort = 2; // _IScntrl  = _ISbit(9)
const IS_PUNCT: c_ushort = 4; // _ISpunct  = _ISbit(10)
const IS_ALNUM: c_ushort = 8; // _ISalnum  = _ISbit(11)

/// Verbatim expansion of glibc's `__isctype(c, mask)`.
///
/// The `char` is promoted to `int` with sign extension first, exactly as C does
/// for the macro argument, so `(char) 0x80` indexes the table at `-128`.
///
/// # Safety
/// `__ctype_b_loc()` always yields a valid pointer whose addressable range
/// covers `-128..=255`, and `c as i32` is confined to `-128..=127`.
#[inline]
fn isctype(c: c_char, mask: c_ushort) -> c_int {
    unsafe {
        let table = *__ctype_b_loc();
        (*table.offset(c as c_int as isize) & mask) as c_int
    }
}

// ---------------------------------------------------------------------------
// The twelve classifiers, each yielding the raw masked bits as glibc does.
// ---------------------------------------------------------------------------

#[inline]
fn isalnum(c: c_char) -> c_int {
    isctype(c, IS_ALNUM)
}
#[inline]
fn isalpha(c: c_char) -> c_int {
    isctype(c, IS_ALPHA)
}
#[inline]
fn islower(c: c_char) -> c_int {
    isctype(c, IS_LOWER)
}
#[inline]
fn isupper(c: c_char) -> c_int {
    isctype(c, IS_UPPER)
}
#[inline]
fn isdigit(c: c_char) -> c_int {
    isctype(c, IS_DIGIT)
}
#[inline]
fn isxdigit(c: c_char) -> c_int {
    isctype(c, IS_XDIGIT)
}
#[inline]
fn iscntrl(c: c_char) -> c_int {
    isctype(c, IS_CNTRL)
}
#[inline]
fn isgraph(c: c_char) -> c_int {
    isctype(c, IS_GRAPH)
}
#[inline]
fn isspace(c: c_char) -> c_int {
    isctype(c, IS_SPACE)
}
#[inline]
fn isblank(c: c_char) -> c_int {
    isctype(c, IS_BLANK)
}
#[inline]
fn isprint(c: c_char) -> c_int {
    isctype(c, IS_PRINT)
}
#[inline]
fn ispunct(c: c_char) -> c_int {
    isctype(c, IS_PUNCT)
}

// ---------------------------------------------------------------------------
// Public ABI: `void driver(char c);` from include/driver.h
// ---------------------------------------------------------------------------

/// The argument is taken as a full-width `int` and narrowed here, rather than
/// declared `c_char`, because that is what the C actually does.
///
/// The x86-64 SysV ABI passes a sub-`int` argument in a 32-bit register slot and
/// leaves the upper bits unspecified, so a callee is free either to trust that
/// the caller extended them or to discard them. The two choices are
/// indistinguishable for a conforming caller but not for a caller that leaves
/// garbage in bits 8..31 — and the C, as GCC compiles it, discards them:
///
/// ```text
/// mov    %edi,%eax
/// mov    %al,-0x4(%rbp)      ; only the low byte is kept
/// ...
/// movsbq -0x4(%rbp),%rax     ; re-read, sign-extended from 8 bits
/// ```
///
/// Declaring the parameter as `c_char` would make rustc mark it `signext` and
/// emit `movslq %edi` instead, indexing the ctype table with the *whole* 32-bit
/// value — a wild read (segfault) where the C prints the result for the low
/// byte. Narrowing explicitly reproduces the C's `mov %al` + `movsb` pair.
#[unsafe(no_mangle)]
pub extern "C" fn driver(c_arg: c_int) {
    let c: c_char = c_arg as u8 as c_char;

    unsafe {
        setlocale(LC_ALL, c"C".as_ptr());

        printf(c"alphanumeric: %d\n".as_ptr(), isalnum(c));
        printf(c"alphabetic: %d\n".as_ptr(), isalpha(c));
        printf(c"lowercase: %d\n".as_ptr(), islower(c));
        printf(c"uppercase: %d\n".as_ptr(), isupper(c));
        printf(c"digit: %d\n".as_ptr(), isdigit(c));
        printf(c"hexadecimal: %d\n".as_ptr(), isxdigit(c));
        printf(c"control: %d\n".as_ptr(), iscntrl(c));
        printf(c"graphical: %d\n".as_ptr(), isgraph(c));
        printf(c"space: %d\n".as_ptr(), isspace(c));
        printf(c"blank: %d\n".as_ptr(), isblank(c));
        printf(c"printing: %d\n".as_ptr(), isprint(c));
        printf(c"punctuation: %d\n".as_ptr(), ispunct(c));
        printf(c"to lower: %c\n".as_ptr(), tolower(c as c_int));
        printf(c"to upper: %c\n".as_ptr(), toupper(c as c_int));
    }
}
