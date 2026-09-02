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
// values 0x80..0xFF arrive as negative indices. glibc's <ctype.h> classifiers
// are macros that index a table which is legally addressable from -128, and
// they return the *raw masked bit*, not a normalised 0/1. For example
// `iscntrl('\0')` yields 2 (`_IScntrl`), not 1. Both behaviours are reproduced
// here verbatim via the embedded glibc "C" locale tables; the classification
// results are therefore bit-identical, including for negative `char` values.
//
// Output is emitted through libc's `printf` rather than Rust's own `std::io`
// stack so that the bytes land in the very same `stdout` FILE buffer a C
// caller uses, preserving exact interleaving and flush ordering.
//
// No bugs are corrected: the ctype calls are passed the raw `char` exactly as
// the C does, and the order/format of every `printf` is preserved.

use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// libc bindings (same entry points the C translation unit called)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

/// `LC_ALL` from glibc's <locale.h>.
const LC_ALL: c_int = 6;

// ---------------------------------------------------------------------------
// glibc <ctype.h> `_ISbit` masks
//   _ISbit(bit) = (bit) < 8 ? ((1 << (bit)) << 8) : ((1 << (bit)) >> 8)
// ---------------------------------------------------------------------------

const IS_UPPER: u16 = 256; // _ISupper
const IS_LOWER: u16 = 512; // _ISlower
const IS_ALPHA: u16 = 1024; // _ISalpha
const IS_DIGIT: u16 = 2048; // _ISdigit
const IS_XDIGIT: u16 = 4096; // _ISxdigit
const IS_SPACE: u16 = 8192; // _ISspace
const IS_PRINT: u16 = 16384; // _ISprint
const IS_GRAPH: u16 = 32768; // _ISgraph
const IS_BLANK: u16 = 1; // _ISblank
const IS_CNTRL: u16 = 2; // _IScntrl
const IS_PUNCT: u16 = 4; // _ISpunct
const IS_ALNUM: u16 = 8; // _ISalnum

/// glibc's ctype tables are addressable from index -128; the embedded arrays
/// below are shifted by this amount so index 0 of the array is C's index -128.
const CTYPE_BIAS: i32 = 128;

/// Maps a `char` argument to an index into the biased tables. Mirrors glibc's
/// `(*__ctype_b_loc ())[(int) (c)]` for a signed `char` in -128..=127.
#[inline]
fn ctype_index(c: c_char) -> usize {
    (c as i32 + CTYPE_BIAS) as usize
}

// ---------------------------------------------------------------------------
// The twelve classifiers, each returning the raw masked bits as glibc does.
// ---------------------------------------------------------------------------

#[inline]
fn class_of(c: c_char, mask: u16) -> c_int {
    (CTYPE_B[ctype_index(c)] & mask) as c_int
}

#[inline]
fn isalnum(c: c_char) -> c_int {
    class_of(c, IS_ALNUM)
}
#[inline]
fn isalpha(c: c_char) -> c_int {
    class_of(c, IS_ALPHA)
}
#[inline]
fn islower(c: c_char) -> c_int {
    class_of(c, IS_LOWER)
}
#[inline]
fn isupper(c: c_char) -> c_int {
    class_of(c, IS_UPPER)
}
#[inline]
fn isdigit(c: c_char) -> c_int {
    class_of(c, IS_DIGIT)
}
#[inline]
fn isxdigit(c: c_char) -> c_int {
    class_of(c, IS_XDIGIT)
}
#[inline]
fn iscntrl(c: c_char) -> c_int {
    class_of(c, IS_CNTRL)
}
#[inline]
fn isgraph(c: c_char) -> c_int {
    class_of(c, IS_GRAPH)
}
#[inline]
fn isspace(c: c_char) -> c_int {
    class_of(c, IS_SPACE)
}
#[inline]
fn isblank(c: c_char) -> c_int {
    class_of(c, IS_BLANK)
}
#[inline]
fn isprint(c: c_char) -> c_int {
    class_of(c, IS_PRINT)
}
#[inline]
fn ispunct(c: c_char) -> c_int {
    class_of(c, IS_PUNCT)
}

#[inline]
fn tolower(c: c_char) -> c_int {
    CTYPE_TOLOWER[ctype_index(c)]
}
#[inline]
fn toupper(c: c_char) -> c_int {
    CTYPE_TOUPPER[ctype_index(c)]
}

// ---------------------------------------------------------------------------
// Public ABI: `void driver(char c);` from include/driver.h
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
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
        printf(c"to lower: %c\n".as_ptr(), tolower(c));
        printf(c"to upper: %c\n".as_ptr(), toupper(c));
    }
}

// ---------------------------------------------------------------------------
// glibc "C" locale ctype tables, indices -128..=255 (biased by CTYPE_BIAS).
// Dumped verbatim from __ctype_b_loc / __ctype_tolower_loc / __ctype_toupper_loc
// after setlocale(LC_ALL, "C").
// ---------------------------------------------------------------------------

static CTYPE_B: [u16; 384] = [
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002,
    0x0002, 0x2003, 0x2002, 0x2002, 0x2002, 0x2002, 0x0002, 0x0002,
    0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002,
    0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002,
    0x6001, 0xc004, 0xc004, 0xc004, 0xc004, 0xc004, 0xc004, 0xc004,
    0xc004, 0xc004, 0xc004, 0xc004, 0xc004, 0xc004, 0xc004, 0xc004,
    0xd808, 0xd808, 0xd808, 0xd808, 0xd808, 0xd808, 0xd808, 0xd808,
    0xd808, 0xd808, 0xc004, 0xc004, 0xc004, 0xc004, 0xc004, 0xc004,
    0xc004, 0xd508, 0xd508, 0xd508, 0xd508, 0xd508, 0xd508, 0xc508,
    0xc508, 0xc508, 0xc508, 0xc508, 0xc508, 0xc508, 0xc508, 0xc508,
    0xc508, 0xc508, 0xc508, 0xc508, 0xc508, 0xc508, 0xc508, 0xc508,
    0xc508, 0xc508, 0xc508, 0xc004, 0xc004, 0xc004, 0xc004, 0xc004,
    0xc004, 0xd608, 0xd608, 0xd608, 0xd608, 0xd608, 0xd608, 0xc608,
    0xc608, 0xc608, 0xc608, 0xc608, 0xc608, 0xc608, 0xc608, 0xc608,
    0xc608, 0xc608, 0xc608, 0xc608, 0xc608, 0xc608, 0xc608, 0xc608,
    0xc608, 0xc608, 0xc608, 0xc004, 0xc004, 0xc004, 0xc004, 0x0002,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
];

static CTYPE_TOLOWER: [i32; 384] = [
    128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139,
    140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151,
    152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163,
    164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175,
    176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187,
    188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199,
    200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211,
    212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223,
    224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235,
    236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247,
    248, 249, 250, 251, 252, 253, 254, -1, 0, 1, 2, 3,
    4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
    40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51,
    52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    64, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107,
    108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119,
    120, 121, 122, 91, 92, 93, 94, 95, 96, 97, 98, 99,
    100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
    112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123,
    124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147,
    148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159,
    160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171,
    172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
    184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195,
    196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207,
    208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
    220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231,
    232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243,
    244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
];

static CTYPE_TOUPPER: [i32; 384] = [
    128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139,
    140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151,
    152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163,
    164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175,
    176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187,
    188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199,
    200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211,
    212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223,
    224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235,
    236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247,
    248, 249, 250, 251, 252, 253, 254, -1, 0, 1, 2, 3,
    4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
    40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51,
    52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75,
    76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87,
    88, 89, 90, 91, 92, 93, 94, 95, 96, 65, 66, 67,
    68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
    80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 123,
    124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147,
    148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159,
    160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171,
    172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183,
    184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195,
    196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207,
    208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
    220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231,
    232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243,
    244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255,
];
