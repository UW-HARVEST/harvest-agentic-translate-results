//! Rust translation of c_src/src/lib.c (a DEFLATE / "pinflate" decompressor).
//!
//! The translation is deliberately literal: control flow, order of validation
//! checks, integer widths, wrap-around behaviour and even the original code's
//! quirks are reproduced exactly.  `assert()` calls from the C source are not
//! reproduced (the C library is built as a shared object with `NDEBUG` in
//! release configuration, where they compile away).

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(static_mut_refs)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr::{addr_of, addr_of_mut};

// ---------------------------------------------------------------------------
// Exported globals (all non-`static` objects of the C translation unit)
// ---------------------------------------------------------------------------

/// `const char *cp_error_reason;`
#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = std::ptr::null();

/// `uint8_t cp_fixed_table[288 + 32]`
///
/// 144 x 8, 112 x 9, 24 x 7, 8 x 8, 32 x 5 -- exactly as spelled out in the C
/// source literal.
#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = {
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
    while i < 320 {
        t[i] = 5;
        i += 1;
    }
    t
};

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

// ---------------------------------------------------------------------------
// Error strings, byte for byte identical to the C literals
// ---------------------------------------------------------------------------

const ERR_LEN_NLEN: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const ERR_STORED_BEYOND: &[u8] = b"Stored block extends beyond end of input stream.\0";
const ERR_OUT_SYMBOL: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.\0";
const ERR_BACK_DISTANCE: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const ERR_OUT_STRING: &[u8] = b"Attempted to overwrite out buffer while outputting a string.\0";
const ERR_BLOCK_TYPE: &[u8] = b"Detected unknown block type within input stream.\0";

#[inline]
unsafe fn set_error(msg: &'static [u8]) {
    cp_error_reason = msg.as_ptr() as *const c_char;
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

// `struct cp_pixel_t` / `struct cp_image_t` exist in the C translation unit but
// are only used by the two unused `static` helpers below; they contribute no
// exported symbols.  They are kept for fidelity of the translated surface.
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

/// `typedef struct cp_state_t { ... } cp_state_t;`
///
/// `#[repr(C)]` with the original field order so that the (out of bounds by
/// one) `tree[lo - 1]` read inside `cp_decode` hits the very same neighbouring
/// bytes it hits in C.
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

impl cp_state_t {
    /// Equivalent of `calloc(1, sizeof(cp_state_t))`.
    fn zeroed() -> Box<cp_state_t> {
        Box::new(cp_state_t {
            bits: 0,
            count: 0,
            words: std::ptr::null_mut(),
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: 0,
            final_word: 0,
            out: std::ptr::null_mut(),
            out_end: std::ptr::null_mut(),
            begin: std::ptr::null_mut(),
            lookup: [0; 1 << 9],
            lit: [0; 288],
            dst: [0; 32],
            len: [0; 19],
            nlit: 0,
            ndst: 0,
            nlen: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// Bit reader
// ---------------------------------------------------------------------------

#[inline]
fn cp_would_overflow(s: &cp_state_t, num_bits: c_int) -> c_int {
    ((s.bits_left.wrapping_add(s.count)).wrapping_sub(num_bits) < 0) as c_int
}

#[inline]
unsafe fn cp_ptr(s: &cp_state_t) -> *mut c_char {
    // assert(!(s->bits_left & 7));
    // (char *)(s->words + s->word_index) - (s->count / 8)
    (s.words.offset(s.word_index as isize) as *mut c_char)
        .offset(-((s.count / 8) as isize))
}

unsafe fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = *s.words.offset(s.word_index as isize);
            s.word_index += 1;
            s.bits |= (word as u64).wrapping_shl(s.count as u32);
            s.count += 32;
        } else if s.final_word_available != 0 {
            let word = s.final_word;
            s.bits |= (word as u64).wrapping_shl(s.count as u32);
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

#[inline]
fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    // assert(s->count >= num_bits_to_read);
    let mask = (1u64.wrapping_shl(num_bits_to_read as u32)).wrapping_sub(1);
    let bits = (s.bits & mask) as u32;
    s.bits = s.bits.wrapping_shr(num_bits_to_read as u32);
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    // asserts elided (NDEBUG)
    let _ = cp_would_overflow(s, num_bits_to_read);
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(a: u32) -> u32 {
    let mut a = a;
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

// ---------------------------------------------------------------------------
// Huffman table construction
// ---------------------------------------------------------------------------

/// `static int cp_build(cp_state_t *s, uint32_t *tree, uint8_t *lens, int sym_count)`
///
/// `counts` / `codes` / `first` are 16 entries wide in C; they are widened to
/// 256 here so that a corrupt code length (only reachable through input that
/// already makes the C code read/write out of bounds) cannot trap.  For every
/// well formed input the behaviour is identical.
unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    let mut codes = [0i32; 256];
    let mut first = [0i32; 256];
    let mut counts = [0i32; 256];

    let mut n = 0;
    while n < sym_count {
        counts[*lens.offset(n as isize) as usize] += 1;
        n += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        std::ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, 1 << 9);
    }
    for i in 0..sym_count {
        let len = *lens.offset(i as isize) as usize;
        if len != 0 {
            // assert(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as u32;
            first[len] += 1;
            *tree.offset(slot as isize) =
                code.wrapping_shl((32u32).wrapping_sub(len as u32)) | ((i as u32) << 4) | len as u32;
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    (*s).lookup[j] = ((len << 9) | i as usize) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

// ---------------------------------------------------------------------------
// Block decoders
// ---------------------------------------------------------------------------

unsafe fn cp_stored(s: &mut cp_state_t) -> c_int {
    cp_read_bits(s, s.count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        set_error(ERR_LEN_NLEN);
        return 0;
    }
    if !(s.bits_left / 8 <= LEN as c_int) {
        set_error(ERR_STORED_BEYOND);
        return 0;
    }
    let p = cp_ptr(s);
    std::ptr::copy_nonoverlapping(p as *const u8, s.out as *mut u8, LEN as usize);
    s.out = s.out.offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: &mut cp_state_t) -> c_int {
    let table = addr_of_mut!(cp_fixed_table) as *const u8;
    let sp: *mut cp_state_t = s;
    let lit = addr_of_mut!((*sp).lit) as *mut u32;
    let dst = addr_of_mut!((*sp).dst) as *mut u32;
    (*sp).nlit = cp_build(sp, lit, table, 288) as u32;
    (*sp).ndst = cp_build(std::ptr::null_mut(), dst, table.offset(288), 32) as u32;
    1
}

unsafe fn cp_decode(s: &mut cp_state_t, tree: *const u32, hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.offset((lo - 1) as isize);
    // assert((search >> (32 - (key & 0xF))) == (key >> (32 - (key & 0xF))));
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: &mut cp_state_t) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit: c_int = 257 + cp_read_bits(s, 5) as c_int;
    let ndst: c_int = 1 + cp_read_bits(s, 5) as c_int;
    let nlen: c_int = 4 + cp_read_bits(s, 4) as c_int;
    let order = addr_of!(cp_permutation_order) as *const u8;
    for i in 0..nlen {
        let bits = cp_read_bits(s, 3) as u8;
        lenlens[*order.offset(i as isize) as usize] = bits;
    }

    let sp: *mut cp_state_t = s;
    let len_tree = addr_of_mut!((*sp).len) as *mut u32;
    (*sp).nlen = cp_build(std::ptr::null_mut(), len_tree, lenlens.as_ptr(), 19) as u32;

    // `uint8_t lens[288 + 32];` in C.  The repeat opcodes can run past the
    // logical end of the array (a latent overflow in the original); the buffer
    // is padded here so that those stray writes stay harmless, while every
    // in-range value matches the C behaviour.  `lens[n - 1]` with `n == 0`
    // reads indeterminate stack memory in C; zero is used here.
    let mut lens = [0u8; 288 + 32 + 138];
    let mut n: c_int = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, len_tree as *const u32, (*sp).nlen as c_int);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as c_int;
                while i != 0 {
                    lens[n as usize] = if n == 0 { 0 } else { lens[(n - 1) as usize] };
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as c_int;
                while i != 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as c_int;
                while i != 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            _ => {
                lens[n as usize] = sym as u8;
                n += 1;
            }
        }
    }

    let lit = addr_of_mut!((*sp).lit) as *mut u32;
    let dst = addr_of_mut!((*sp).dst) as *mut u32;
    (*sp).nlit = cp_build(sp, lit, lens.as_ptr(), nlit) as u32;
    (*sp).ndst = cp_build(
        std::ptr::null_mut(),
        dst,
        lens.as_ptr().offset(nlit as isize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: &mut cp_state_t) -> c_int {
    let sp: *mut cp_state_t = s;
    let lit = addr_of_mut!((*sp).lit) as *const u32;
    let dst_tree = addr_of_mut!((*sp).dst) as *const u32;
    let len_extra = addr_of!(cp_len_extra_bits) as *const u8;
    let len_base = addr_of!(cp_len_base) as *const u32;
    let dist_extra = addr_of!(cp_dist_extra_bits) as *const u8;
    let dist_base = addr_of!(cp_dist_base) as *const u32;

    loop {
        let mut symbol = cp_decode(s, lit, (*sp).nlit as c_int);
        if symbol < 256 {
            if !(s.out.wrapping_offset(1) <= s.out_end) {
                set_error(ERR_OUT_SYMBOL);
                return 0;
            }
            *s.out = symbol as c_char;
            s.out = s.out.offset(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length: c_int = cp_read_bits(s, *len_extra.offset(symbol as isize) as c_int)
                as c_int
                + *len_base.offset(symbol as isize) as c_int;
            let distance_symbol = cp_decode(s, dst_tree, (*sp).ndst as c_int);
            let backwards_distance: c_int =
                cp_read_bits(s, *dist_extra.offset(distance_symbol as isize) as c_int) as c_int
                    + *dist_base.offset(distance_symbol as isize) as c_int;
            if !(s.out.wrapping_offset(-(backwards_distance as isize)) >= s.begin) {
                set_error(ERR_BACK_DISTANCE);
                return 0;
            }
            if !(s.out.wrapping_offset(length as isize) <= s.out_end) {
                set_error(ERR_OUT_STRING);
                return 0;
            }
            let mut src = s.out.offset(-(backwards_distance as isize));
            let mut dst = s.out;
            s.out = s.out.offset(length as isize);
            match backwards_distance {
                1 => {
                    std::ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
                }
                _ => {
                    let mut length = length;
                    while {
                        let old = length;
                        length -= 1;
                        old != 0
                    } {
                        *dst = *src;
                        dst = dst.offset(1);
                        src = src.offset(1);
                    }
                }
            }
        } else {
            break;
        }
    }
    1
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// `int pinflate(void *in, int in_bytes, void *out, int out_bytes)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pinflate(
    in_: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut boxed = cp_state_t::zeroed();
    let s: &mut cp_state_t = &mut boxed;

    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes.wrapping_mul(8);

    let in_addr = in_ as usize;
    let first_bytes: c_int = (((in_addr + 3) & !3usize) - in_addr) as c_int;
    s.words = (in_ as *mut c_char).offset(first_bytes as isize) as *mut u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes: c_int = (in_bytes - first_bytes) & 3;

    let in_u8 = in_ as *const u8;
    for i in 0..first_bytes {
        s.bits |= (*in_u8.offset(i as isize) as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        s.final_word |=
            (*in_u8.offset((in_bytes - last_bytes + i) as isize) as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out as *mut c_char;
    s.out_end = s.out.wrapping_offset(out_bytes as isize);
    s.begin = out as *mut c_char;

    let mut _count: c_int = 0;
    let mut bfinal: u32;
    loop {
        bfinal = cp_read_bits(s, 1);
        let btype = cp_read_bits(s, 2);
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    return 0;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    return 0;
                }
            }
            3 => {
                set_error(ERR_BLOCK_TYPE);
                return 0;
            }
            _ => {}
        }
        _count += 1;
        if bfinal != 0 {
            break;
        }
    }
    1
}
