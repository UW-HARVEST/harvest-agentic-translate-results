//! Faithful Rust translation of `c_src/src/lib.c` (a cut-down `cute_png` /
//! DEFLATE + PNG unfilter library by Randy Gaul).
//!
//! The goal of this crate is bit-exact behavioural parity with the C shared
//! library that CMake builds out of `c_src`, including its exported data
//! symbols (`cp_fixed_table`, `cp_len_base`, ...), its mutable global
//! `cp_error_reason` (and the exact error strings assigned to it), and the
//! order in which every validation check happens.
//!
//! Notes on fidelity:
//!   * `c_src`'s `assert()`s are translated behind the `c-asserts` cargo
//!     feature, which is **on by default** because the reference `.so` is built
//!     without `NDEBUG` (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON` sets no
//!     `CMAKE_BUILD_TYPE`, so `__assert_fail` is linked in).  A failing
//!     translated assert writes a glibc-shaped diagnostic to stderr and calls
//!     `abort()`, i.e. it dies with `SIGABRT` exactly where the C library does.
//!     `--no-default-features` reproduces a `-DNDEBUG` build instead.
//!   * All arithmetic mirrors C's wrap-around behaviour on x86-64 (wrapping
//!     add/sub/mul, shifts masked like the hardware `shl`/`shr` instructions).
//!   * Pointer arithmetic and (possibly out-of-range) loads/stores are done
//!     through raw pointers so that quirks such as `tree[lo - 1]` in
//!     `cp_decode` read exactly the same bytes of the decoder state as the C
//!     code does.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::offset_of;
use std::ptr;

// ---------------------------------------------------------------------------
// `assert()` (see the `c-asserts` feature)
// ---------------------------------------------------------------------------

/// The file name glibc's `assert()` would report.
#[cfg(feature = "c-asserts")]
const C_FILE: &str = "src/lib.c";

/// Mirrors glibc's `__assert_fail`: a diagnostic on stderr, then `abort()`.
#[cfg(feature = "c-asserts")]
#[cold]
#[inline(never)]
fn cp_assert_fail(expr: &str, line: u32, func: &str) -> ! {
    use std::io::Write;
    let _ = writeln!(
        std::io::stderr(),
        "unfilter_lib: {C_FILE}:{line}: {func}: Assertion `{expr}' failed."
    );
    std::process::abort()
}

#[cfg(feature = "c-asserts")]
macro_rules! c_assert {
    ($cond:expr, $expr:expr, $line:expr, $func:expr) => {
        if !($cond) {
            crate::cp_assert_fail($expr, $line, $func)
        }
    };
}

#[cfg(not(feature = "c-asserts"))]
macro_rules! c_assert {
    ($cond:expr, $expr:expr, $line:expr, $func:expr) => {};
}

// ---------------------------------------------------------------------------
// Types that exist in the C translation unit (unused by the public ABI, but
// kept for completeness / documentation of the original source).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
struct cp_pixel_t {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct cp_image_t {
    w: c_int,
    h: c_int,
    pix: *mut cp_pixel_t,
}

#[allow(dead_code)]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

#[allow(dead_code)]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

// ---------------------------------------------------------------------------
// Exported global data (matches the C symbols exactly, including sizes)
// ---------------------------------------------------------------------------

/// `const char *cp_error_reason;`
#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

const fn make_fixed_table() -> [u8; 288 + 32] {
    let mut t = [0u8; 288 + 32];
    let mut i = 0usize;
    while i < 144 {
        t[i] = 8;
        i += 1;
    }
    while i < 256 {
        t[i] = 9;
        i += 1;
    }
    while i < 280 {
        t[i] = 7;
        i += 1;
    }
    while i < 288 {
        t[i] = 8;
        i += 1;
    }
    while i < 288 + 32 {
        t[i] = 5;
        i += 1;
    }
    t
}

/// `uint8_t cp_fixed_table[288 + 32]`
#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = make_fixed_table();

/// `uint8_t cp_permutation_order[19]`
#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// `uint8_t cp_len_extra_bits[29 + 2]`
#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

/// `uint32_t cp_len_base[29 + 2]`
#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

/// `uint8_t cp_dist_extra_bits[30 + 2]`
#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

/// `uint32_t cp_dist_base[30 + 2]`
#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

// Error strings, byte-for-byte identical to the C string literals.
macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

const ERR_STORED_LEN_NLEN: *const c_char =
    cstr!("Failed to find LEN and NLEN as complements within stored (uncompressed) stream.");
const ERR_STORED_BEYOND: *const c_char = cstr!("Stored block extends beyond end of input stream.");
const ERR_OUT_SYMBOL: *const c_char =
    cstr!("Attempted to overwrite out buffer while outputting a symbol.");
const ERR_BACKWARDS: *const c_char =
    cstr!("Attempted to write before out buffer (invalid backwards distance).");
const ERR_OUT_STRING: *const c_char =
    cstr!("Attempted to overwrite out buffer while outputting a string.");
const ERR_UNKNOWN_BLOCK: *const c_char =
    cstr!("Detected unknown block type within input stream.");

#[inline]
unsafe fn set_error(msg: *const c_char) {
    cp_error_reason = msg;
}

// Helpers that read the (publicly mutable) tables through raw pointers so that
// external writes to the exported symbols are observed exactly as in C.
#[inline]
unsafe fn fixed_table_ptr() -> *mut u8 {
    (&raw mut cp_fixed_table) as *mut u8
}
#[inline]
unsafe fn permutation_order(i: c_int) -> u8 {
    *((&raw const cp_permutation_order) as *const u8).wrapping_offset(i as isize)
}
#[inline]
unsafe fn len_extra_bits(i: c_int) -> u8 {
    *((&raw const cp_len_extra_bits) as *const u8).wrapping_offset(i as isize)
}
#[inline]
unsafe fn len_base(i: c_int) -> u32 {
    *((&raw const cp_len_base) as *const u32).wrapping_offset(i as isize)
}
#[inline]
unsafe fn dist_extra_bits(i: c_int) -> u8 {
    *((&raw const cp_dist_extra_bits) as *const u8).wrapping_offset(i as isize)
}
#[inline]
unsafe fn dist_base(i: c_int) -> u32 {
    *((&raw const cp_dist_base) as *const u32).wrapping_offset(i as isize)
}

// ---------------------------------------------------------------------------
// Inflate state
// ---------------------------------------------------------------------------

#[repr(C)]
struct cp_state_t {
    bits: u64,
    count: c_int,
    words: *mut u32,
    word_count: c_int,
    word_index: c_int,
    bits_left: c_int,
    final_word_available: c_int,
    final_word: u32,
    out: *mut c_char,
    out_end: *mut c_char,
    begin: *mut c_char,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

// All of the sub-array pointers are derived from the base of the whole state
// allocation, so that (in)famous out-of-range accesses such as `tree[lo - 1]`
// in `cp_decode()` touch the same memory the C code touches.
#[inline]
unsafe fn lookup_ptr(s: *mut cp_state_t) -> *mut u16 {
    (s as *mut u8).wrapping_add(offset_of!(cp_state_t, lookup)) as *mut u16
}
#[inline]
unsafe fn lit_ptr(s: *mut cp_state_t) -> *mut u32 {
    (s as *mut u8).wrapping_add(offset_of!(cp_state_t, lit)) as *mut u32
}
#[inline]
unsafe fn dst_ptr(s: *mut cp_state_t) -> *mut u32 {
    (s as *mut u8).wrapping_add(offset_of!(cp_state_t, dst)) as *mut u32
}
#[inline]
unsafe fn len_ptr(s: *mut cp_state_t) -> *mut u32 {
    (s as *mut u8).wrapping_add(offset_of!(cp_state_t, len)) as *mut u32
}

/// `static int cp_would_overflow(cp_state_t *s, int num_bits)`
/// (only referenced from asserts in the C source)
#[allow(dead_code)]
unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

/// `static char *cp_ptr(cp_state_t *s)`
unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    c_assert!((*s).bits_left & 7 == 0, "!(s->bits_left & 7)", 95, "cp_ptr");
    ((*s).words.wrapping_offset((*s).word_index as isize) as *mut c_char)
        .wrapping_offset(-(((*s).count / 8) as isize))
}

/// `static uint64_t cp_peak_bits(cp_state_t *s, int num_bits_to_read)`
unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = ptr::read_unaligned((*s).words.wrapping_offset((*s).word_index as isize));
            (*s).word_index = (*s).word_index.wrapping_add(1);
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add(32);
            c_assert!(
                (*s).word_index <= (*s).word_count,
                "s->word_index <= s->word_count",
                104,
                "cp_peak_bits"
            );
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add((*s).bits_left);
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

/// `static uint32_t cp_consume_bits(cp_state_t *s, int num_bits_to_read)`
unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    c_assert!(
        (*s).count >= num_bits_to_read,
        "s->count >= num_bits_to_read",
        115,
        "cp_consume_bits"
    );
    let mask = 1u64
        .wrapping_shl(num_bits_to_read as u32)
        .wrapping_sub(1);
    let bits = ((*s).bits & mask) as u32;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count = (*s).count.wrapping_sub(num_bits_to_read);
    (*s).bits_left = (*s).bits_left.wrapping_sub(num_bits_to_read);
    bits
}

/// `static uint32_t cp_read_bits(cp_state_t *s, int num_bits_to_read)`
unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    c_assert!(num_bits_to_read <= 32, "num_bits_to_read <= 32", 123, "cp_read_bits");
    c_assert!(num_bits_to_read >= 0, "num_bits_to_read >= 0", 124, "cp_read_bits");
    c_assert!((*s).bits_left > 0, "s->bits_left > 0", 125, "cp_read_bits");
    c_assert!((*s).count <= 64, "s->count <= 64", 126, "cp_read_bits");
    c_assert!(
        cp_would_overflow(s, num_bits_to_read) == 0,
        "!cp_would_overflow(s, num_bits_to_read)",
        127,
        "cp_read_bits"
    );
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

/// `static uint32_t cp_rev16(uint32_t a)`
fn cp_rev16(a: u32) -> u32 {
    let mut a = a;
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

/// `static int cp_build(cp_state_t *s, uint32_t *tree, uint8_t *lens, int sym_count)`
///
/// The C code declares `codes[16]`, `first[16]`, `counts[16]` and indexes them
/// with the code lengths.  Well-formed input keeps every length below 16; the
/// tables here are over-sized (and zero filled) purely so that malformed input
/// cannot make the translation trap where C would silently scribble on its
/// stack frame.
unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    let mut codes = [0i32; 256];
    let mut first = [0i32; 256];
    let mut counts = [0i32; 256];

    let mut n: c_int = 0;
    while n < sym_count {
        let idx = *lens.wrapping_offset(n as isize) as usize;
        counts[idx] = counts[idx].wrapping_add(1);
        n += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    n = 1;
    while n <= 15 {
        let i = n as usize;
        codes[i] = (codes[i - 1].wrapping_add(counts[i - 1])) << 1;
        first[i] = first[i - 1].wrapping_add(counts[i - 1]);
        n += 1;
    }
    if !s.is_null() {
        ptr::write_bytes(lookup_ptr(s), 0, 1 << 9);
    }
    let mut i: c_int = 0;
    while i < sym_count {
        let len = *lens.wrapping_offset(i as isize) as c_int;
        if len != 0 {
            c_assert!(len < 16, "len < 16", 154, "cp_build");
            let li = len as usize;
            let code = codes[li] as u32;
            codes[li] = codes[li].wrapping_add(1);
            let slot = first[li] as u32;
            first[li] = first[li].wrapping_add(1);
            *tree.wrapping_offset(slot as i32 as isize) = code
                .wrapping_shl((32i32.wrapping_sub(len)) as u32)
                | ((i as u32) << 4)
                | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> ((16i32.wrapping_sub(len)) as u32 & 31)) as c_int;
                let lut = lookup_ptr(s);
                while j < (1 << 9) {
                    *lut.wrapping_offset(j as isize) = (((len << 9) | i) as u32) as u16;
                    j = j.wrapping_add(1 << (len & 31));
                }
            }
        }
        i += 1;
    }
    first[15]
}

/// `static int cp_stored(cp_state_t *s)`
unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        set_error(ERR_STORED_LEN_NLEN);
        return 0;
    }
    if !((*s).bits_left / 8 <= LEN as c_int) {
        set_error(ERR_STORED_BEYOND);
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy(p as *const u8, (*s).out as *mut u8, LEN as usize);
    (*s).out = (*s).out.wrapping_offset(LEN as isize);
    1
}

/// `static int cp_fixed(cp_state_t *s)`
unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    (*s).nlit = cp_build(s, lit_ptr(s), fixed_table_ptr(), 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        dst_ptr(s),
        fixed_table_ptr().wrapping_add(288),
        32,
    ) as u32;
    1
}

/// `static int cp_decode(cp_state_t *s, uint32_t *tree, int hi)`
unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo.wrapping_add(hi)) >> 1;
        if search < *tree.wrapping_offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess.wrapping_add(1);
        }
    }
    let key = *tree.wrapping_offset((lo.wrapping_sub(1)) as isize);
    let len = 32u32.wrapping_sub(key & 0xF);
    // `search >> len` / `key >> len` are 32-bit variable shifts in the C build
    // (`shr %cl, %esi`), so the count is taken modulo 32 - `wrapping_shr`
    // reproduces that for the `key & 0xF == 0` case where `len == 32`.
    c_assert!(
        search.wrapping_shr(len) == key.wrapping_shr(len),
        "(search >> len) == (key >> len)",
        217,
        "cp_decode"
    );
    let _ = len;
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    (((key >> 4) & 0xFFF) as i32) as c_int
}

/// `static int cp_dynamic(cp_state_t *s)`
unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    // `uint8_t lenlens[19] = {0};` - padded so that a corrupted
    // `cp_permutation_order` cannot trap the translation.
    let mut lenlens_buf = [0u8; 19 + 256];
    let lenlens = lenlens_buf.as_mut_ptr();

    let nlit: c_int = 257i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let ndst: c_int = 1i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let nlen: c_int = 4i32.wrapping_add(cp_read_bits(s, 4) as c_int);
    let mut i: c_int = 0;
    while i < nlen {
        *lenlens.wrapping_offset(permutation_order(i) as isize) = cp_read_bits(s, 3) as u8;
        i += 1;
    }
    (*s).nlen = cp_build(ptr::null_mut(), len_ptr(s), lenlens, 19) as u32;

    // `uint8_t lens[288 + 32];` - uninitialised in C.  A leading slack byte
    // stands in for the `lens[-1]` read that the `case 16` path can perform
    // when `n == 0`, and trailing slack absorbs run-length overshoot.
    let mut lens_buf = [0u8; 8 + 288 + 32 + 512];
    let lens = lens_buf.as_mut_ptr().wrapping_add(8);

    let mut n: c_int = 0;
    while n < nlit.wrapping_add(ndst) {
        let sym = cp_decode(s, len_ptr(s), (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i = 3i32.wrapping_add(cp_read_bits(s, 2) as c_int);
                while i != 0 {
                    *lens.wrapping_offset(n as isize) =
                        *lens.wrapping_offset((n.wrapping_sub(1)) as isize);
                    i = i.wrapping_sub(1);
                    n = n.wrapping_add(1);
                }
            }
            17 => {
                let mut i = 3i32.wrapping_add(cp_read_bits(s, 3) as c_int);
                while i != 0 {
                    *lens.wrapping_offset(n as isize) = 0;
                    i = i.wrapping_sub(1);
                    n = n.wrapping_add(1);
                }
            }
            18 => {
                let mut i = 11i32.wrapping_add(cp_read_bits(s, 7) as c_int);
                while i != 0 {
                    *lens.wrapping_offset(n as isize) = 0;
                    i = i.wrapping_sub(1);
                    n = n.wrapping_add(1);
                }
            }
            _ => {
                *lens.wrapping_offset(n as isize) = sym as u8;
                n = n.wrapping_add(1);
            }
        }
    }
    (*s).nlit = cp_build(s, lit_ptr(s), lens, nlit) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        dst_ptr(s),
        lens.wrapping_offset(nlit as isize),
        ndst,
    ) as u32;
    1
}

/// `static int cp_block(cp_state_t *s)`
unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    loop {
        let mut symbol = cp_decode(s, lit_ptr(s), (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.wrapping_offset(1) as usize <= (*s).out_end as usize) {
                set_error(ERR_OUT_SYMBOL);
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.wrapping_offset(1);
        } else if symbol > 256 {
            symbol = symbol.wrapping_sub(257);
            let mut length: c_int = cp_read_bits(s, len_extra_bits(symbol) as c_int)
                .wrapping_add(len_base(symbol)) as c_int;
            let distance_symbol = cp_decode(s, dst_ptr(s), (*s).ndst as c_int);
            let backwards_distance: c_int = cp_read_bits(s, dist_extra_bits(distance_symbol) as c_int)
                .wrapping_add(dist_base(distance_symbol)) as c_int;
            if !((*s).out.wrapping_offset(-(backwards_distance as isize)) as usize
                >= (*s).begin as usize)
            {
                set_error(ERR_BACKWARDS);
                return 0;
            }
            if !((*s).out.wrapping_offset(length as isize) as usize <= (*s).out_end as usize) {
                set_error(ERR_OUT_STRING);
                return 0;
            }
            let mut src = (*s).out.wrapping_offset(-(backwards_distance as isize)) as *const u8;
            let mut dst = (*s).out as *mut u8;
            (*s).out = (*s).out.wrapping_offset(length as isize);
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst, *src, length as usize);
                }
                _ => {
                    // `while (length--) *dst++ = *src++;`
                    while length != 0 {
                        length = length.wrapping_sub(1);
                        *dst = *src;
                        dst = dst.wrapping_offset(1);
                        src = src.wrapping_offset(1);
                    }
                }
            }
        } else {
            break;
        }
    }
    1
}

/// `int cp_inflate(void *in, int in_bytes, void *out, int out_bytes)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    r#in: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let layout = std::alloc::Layout::new::<cp_state_t>();
    let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;

    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let in_addr = r#in as usize;
    let first_bytes: c_int = (((in_addr.wrapping_add(3)) & !3usize).wrapping_sub(in_addr)) as c_int;
    (*s).words = (r#in as *mut u8).wrapping_offset(first_bytes as isize) as *mut u32;
    (*s).word_count = in_bytes.wrapping_sub(first_bytes) / 4;
    let last_bytes: c_int = in_bytes.wrapping_sub(first_bytes) & 3;
    let mut i: c_int = 0;
    while i < first_bytes {
        (*s).bits |= (*(r#in as *const u8).wrapping_offset(i as isize) as u64)
            .wrapping_shl((i.wrapping_mul(8)) as u32);
        i += 1;
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    i = 0;
    while i < last_bytes {
        let byte = *(r#in as *const u8)
            .wrapping_offset(in_bytes.wrapping_sub(last_bytes).wrapping_add(i) as isize);
        (*s).final_word |=
            ((byte as c_int).wrapping_shl((i.wrapping_mul(8)) as u32 & 31)) as u32;
        i += 1;
    }
    (*s).count = first_bytes.wrapping_mul(8);
    (*s).out = out as *mut c_char;
    (*s).out_end = (*s).out.wrapping_offset(out_bytes as isize);
    (*s).begin = out as *mut c_char;

    let mut count: c_int = 0;
    let mut bfinal: c_int;
    let mut ok = true;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    ok = false;
                    break;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    ok = false;
                    break;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    ok = false;
                    break;
                }
            }
            3 => {
                set_error(ERR_UNKNOWN_BLOCK);
                ok = false;
                break;
            }
            _ => {}
        }
        count = count.wrapping_add(1);
        let _ = count;
        if bfinal != 0 {
            break;
        }
    }
    std::alloc::dealloc(s as *mut u8, layout);
    if ok {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// PNG unfiltering
// ---------------------------------------------------------------------------

/// `static uint8_t cp_paeth(uint8_t a, uint8_t b, uint8_t c)`
fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: c_int = (a as c_int).wrapping_add(b as c_int).wrapping_sub(c as c_int);
    let pa = (p.wrapping_sub(a as c_int)).wrapping_abs();
    let pb = (p.wrapping_sub(b as c_int)).wrapping_abs();
    let pc = (p.wrapping_sub(c as c_int)).wrapping_abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[repr(C)]
struct cp_raw_png_t {
    p: *const u8,
    end: *const u8,
}

/// `static uint32_t cp_make32(const uint8_t *s)`
#[allow(dead_code)]
unsafe fn cp_make32(s: *const u8) -> u32 {
    (((*s.wrapping_offset(0) as c_int) << 24)
        | ((*s.wrapping_offset(1) as c_int) << 16)
        | ((*s.wrapping_offset(2) as c_int) << 8)
        | (*s.wrapping_offset(3) as c_int)) as u32
}

/// `static const uint8_t *cp_chunk(cp_raw_png_t *png, const char *chunk, uint32_t minlen)`
#[allow(dead_code)]
unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if memcmp_c(start.wrapping_offset(4), chunk as *const u8, 4) == 0 && len >= minlen {
        let offset = len.wrapping_add(12) as c_int;
        if (*png).p.wrapping_offset(offset as isize) as usize <= (*png).end as usize {
            (*png).p = (*png).p.wrapping_offset(offset as isize);
            return start.wrapping_offset(8);
        }
    }
    ptr::null()
}

/// `static const uint8_t *cp_find(cp_raw_png_t *png, const char *chunk, uint32_t minlen)`
#[allow(dead_code)]
unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    while ((*png).p as usize) < ((*png).end as usize) {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.wrapping_offset(len.wrapping_add(12) as isize);
        if memcmp_c(start.wrapping_offset(4), chunk as *const u8, 4) == 0
            && len >= minlen
            && (*png).p as usize <= (*png).end as usize
        {
            return start.wrapping_offset(8);
        }
    }
    ptr::null()
}

unsafe fn memcmp_c(a: *const u8, b: *const u8, n: usize) -> c_int {
    let mut i = 0usize;
    while i < n {
        let x = *a.wrapping_add(i);
        let y = *b.wrapping_add(i);
        if x != y {
            return x as c_int - y as c_int;
        }
        i += 1;
    }
    0
}

/// `int unfilter(int w, int h, int bpp, uint8_t *raw)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len: c_int = w.wrapping_mul(bpp);
    let mut raw = raw;
    let prev: *mut u8;
    let mut x: c_int;

    if h > 0 {
        let filter = *raw;
        raw = raw.wrapping_offset(1);
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    let v = *raw.wrapping_offset((x.wrapping_sub(bpp)) as isize);
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    let v = *raw.wrapping_offset((x.wrapping_sub(bpp)) as isize) / 2;
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let v = cp_paeth(*raw.wrapping_offset((x.wrapping_sub(bpp)) as isize), 0, 0);
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            _ => return 0,
        }
    }

    prev = raw;
    let mut prev = prev;
    raw = raw.wrapping_offset(len as isize);

    let mut y: c_int = 1;
    while y < h {
        let filter = *raw;
        raw = raw.wrapping_offset(1);
        match filter {
            0 => {}
            1 => {
                // `for (x = 0; x < bpp; x++) raw[x] += 0;`
                // The value is unchanged, but the read-modify-write itself is
                // observable (it faults when `raw + x` is not mapped), so it is
                // translated literally rather than folded away.
                x = 0;
                while x < bpp {
                    let d = raw.wrapping_offset(x as isize);
                    // volatile: `+= 0` is a no-op that LLVM would otherwise
                    // delete, and the access itself is what matters here.
                    let v = ptr::read_volatile(d);
                    ptr::write_volatile(d, v.wrapping_add(0));
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = *raw.wrapping_offset((x.wrapping_sub(bpp)) as isize);
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    let v = *prev.wrapping_offset(x as isize);
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = *prev.wrapping_offset(x as isize);
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    let v = *prev.wrapping_offset(x as isize) / 2;
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let a = *raw.wrapping_offset((x.wrapping_sub(bpp)) as isize) as c_int;
                    let b = *prev.wrapping_offset(x as isize) as c_int;
                    let v = (a.wrapping_add(b) / 2) as u8;
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    let v = *prev.wrapping_offset(x as isize);
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = cp_paeth(
                        *raw.wrapping_offset((x.wrapping_sub(bpp)) as isize),
                        *prev.wrapping_offset(x as isize),
                        *prev.wrapping_offset((x.wrapping_sub(bpp)) as isize),
                    );
                    let d = raw.wrapping_offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            _ => return 0,
        }
        y = y.wrapping_add(1);
        prev = raw;
        raw = raw.wrapping_offset(len as isize);
    }
    1
}

// Keep the unused C-side types referenced so the translation documents the
// whole translation unit without warnings.
#[allow(dead_code)]
const _ASSERT_TYPES: usize = std::mem::size_of::<cp_image_t>() + std::mem::size_of::<cp_pixel_t>();
