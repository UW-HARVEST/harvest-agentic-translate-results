//! Rust translation of the C library located in `c_src/`.
//!
//! The C library is a single-file PNG loader (cute_png style).  This
//! translation reproduces the *exact* observable behaviour of the C code,
//! including its quirks and bugs, and exports the very same public ABI:
//!
//!   * functions: `cp_inflate`, `load_png_mem`
//!   * data:      `cp_error_reason`, `cp_fixed_table`, `cp_permutation_order`,
//!                `cp_len_extra_bits`, `cp_len_base`, `cp_dist_extra_bits`,
//!                `cp_dist_base`
//!
//! The C sources are compiled with `NDEBUG` semantics in mind (the `assert()`
//! calls of the original are pure debugging aids and have no effect on the
//! produced output), therefore they are not reproduced here.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_assignments)]
#![allow(unused_variables)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// public types (see include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

#[inline]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

#[inline]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

// ---------------------------------------------------------------------------
// exported data
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 320] = [
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 8, 8, 8, 8, 8, 8, 8, 8, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
];
#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];
#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];
#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];
#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];
#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

// Raw accessors for the exported tables (the C code indexes them without any
// bounds checking, so raw pointers are used here as well).
#[inline]
fn fixed_table_ptr() -> *mut u8 {
    (&raw mut cp_fixed_table).cast::<u8>()
}
#[inline]
fn permutation_order_ptr() -> *const u8 {
    (&raw const cp_permutation_order).cast::<u8>()
}
#[inline]
fn len_extra_bits_ptr() -> *const u8 {
    (&raw const cp_len_extra_bits).cast::<u8>()
}
#[inline]
fn len_base_ptr() -> *const u32 {
    (&raw const cp_len_base).cast::<u32>()
}
#[inline]
fn dist_extra_bits_ptr() -> *const u8 {
    (&raw const cp_dist_extra_bits).cast::<u8>()
}
#[inline]
fn dist_base_ptr() -> *const u32 {
    (&raw const cp_dist_base).cast::<u32>()
}

/// Sets `cp_error_reason` to a static, NUL terminated C string.
macro_rules! cp_set_error {
    ($msg:expr) => {{
        cp_error_reason = concat!($msg, "\0").as_ptr() as *const c_char;
    }};
}

// ---------------------------------------------------------------------------
// DEFLATE decompressor
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

/// Only ever used by the `assert()` calls of the original C code.
#[allow(dead_code)]
#[inline]
unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

#[inline]
unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    ((*s).words.wrapping_offset((*s).word_index as isize) as *mut c_char)
        .wrapping_offset(-(((*s).count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.wrapping_offset((*s).word_index as isize);
            (*s).word_index = (*s).word_index.wrapping_add(1);
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add(32);
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add((*s).bits_left);
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    let mask = (1u64.wrapping_shl(num_bits_to_read as u32)).wrapping_sub(1);
    let bits = ((*s).bits & mask) as u32;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count = (*s).count.wrapping_sub(num_bits_to_read);
    (*s).bits_left = (*s).bits_left.wrapping_sub(num_bits_to_read);
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
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

unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    // The C code uses `int codes[16], first[16], counts[16]`; larger arrays are
    // used here so that (undefined in C) out of range code lengths cannot
    // corrupt unrelated memory.
    let mut codes = [0i32; 256];
    let mut first = [0i32; 256];
    let mut counts = [0i32; 256];

    let mut n: c_int = 0;
    while n < sym_count {
        let idx = *lens.wrapping_offset(n as isize) as usize;
        counts[idx] = counts[idx].wrapping_add(1);
        n = n.wrapping_add(1);
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1usize..=15usize {
        codes[n] = codes[n - 1].wrapping_add(counts[n - 1]).wrapping_mul(2);
        first[n] = first[n - 1].wrapping_add(counts[n - 1]);
    }
    if !s.is_null() {
        memset(
            (&raw mut (*s).lookup).cast::<c_void>(),
            0,
            core::mem::size_of::<[u16; 1 << 9]>(),
        );
    }
    let mut i: c_int = 0;
    while i < sym_count {
        let len = *lens.wrapping_offset(i as isize) as c_int;
        if len != 0 {
            let code = codes[len as usize] as u32;
            codes[len as usize] = codes[len as usize].wrapping_add(1);
            let slot = first[len as usize] as u32;
            first[len as usize] = first[len as usize].wrapping_add(1);
            *tree.wrapping_offset(slot as isize) = code
                .wrapping_shl((32 - len) as u32)
                | ((i as u32) << 4)
                | (len as u32);
            if !s.is_null() && len <= 9 {
                let lookup = (&raw mut (*s).lookup).cast::<u16>();
                let mut j: c_int = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    *lookup.wrapping_offset(j as isize) =
                        (((len as u32) << 9) | (i as u32)) as u16;
                    j = j.wrapping_add(1 << len);
                }
            }
        }
        i = i.wrapping_add(1);
    }
    first[15]
}

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        cp_set_error!(
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."
        );
        return 0;
    }
    if !((*s).bits_left / 8 <= LEN as c_int) {
        cp_set_error!("Stored block extends beyond end of input stream.");
        return 0;
    }
    let p = cp_ptr(s);
    memcpy(
        (*s).out as *mut c_void,
        p as *const c_void,
        LEN as usize,
    );
    (*s).out = (*s).out.wrapping_offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    (*s).nlit = cp_build(s, (&raw mut (*s).lit).cast::<u32>(), fixed_table_ptr(), 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (&raw mut (*s).dst).cast::<u32>(),
        fixed_table_ptr().wrapping_add(288),
        32,
    ) as u32;
    1
}

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
    let code = cp_consume_bits(s, (key & 0xF) as c_int);
    let _ = code;
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    // `uint8_t lenlens[19] = {0};` in C.  Slack is reserved because
    // `cp_permutation_order` is a public (writable) symbol and may contain
    // values >= 19, in which case the C code writes past the array.
    let mut lenlens = [0u8; 19 + 256];
    let nlit: c_int = 257i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let ndst: c_int = 1i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let nlen: c_int = 4i32.wrapping_add(cp_read_bits(s, 4) as c_int);
    {
        let mut i: c_int = 0;
        while i < nlen {
            let slot = *permutation_order_ptr().wrapping_offset(i as isize) as usize;
            let v = cp_read_bits(s, 3) as u8;
            *(lenlens.as_mut_ptr()).wrapping_add(slot) = v;
            i = i.wrapping_add(1);
        }
    }
    (*s).nlen = cp_build(
        ptr::null_mut(),
        (&raw mut (*s).len).cast::<u32>(),
        lenlens.as_ptr(),
        19,
    ) as u32;

    // `uint8_t lens[288 + 32];` in C.  The run-length cases below may write a
    // little past `nlit + ndst`, so extra slack is reserved here (the C code
    // simply smashes its own stack in that case).
    let mut lens_storage = [0u8; 288 + 32 + 160];
    let lens = lens_storage.as_mut_ptr();

    let mut n: c_int = 0;
    while n < nlit.wrapping_add(ndst) {
        let sym = cp_decode(s, (&raw mut (*s).len).cast::<u32>(), (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i: c_int = 3i32.wrapping_add(cp_read_bits(s, 2) as c_int);
                while i != 0 {
                    *lens.wrapping_offset(n as isize) =
                        *lens.wrapping_offset((n.wrapping_sub(1)) as isize);
                    i = i.wrapping_sub(1);
                    n = n.wrapping_add(1);
                }
            }
            17 => {
                let mut i: c_int = 3i32.wrapping_add(cp_read_bits(s, 3) as c_int);
                while i != 0 {
                    *lens.wrapping_offset(n as isize) = 0;
                    i = i.wrapping_sub(1);
                    n = n.wrapping_add(1);
                }
            }
            18 => {
                let mut i: c_int = 11i32.wrapping_add(cp_read_bits(s, 7) as c_int);
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
    (*s).nlit = cp_build(s, (&raw mut (*s).lit).cast::<u32>(), lens, nlit) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (&raw mut (*s).dst).cast::<u32>(),
        lens.wrapping_offset(nlit as isize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    loop {
        let mut symbol = cp_decode(s, (&raw mut (*s).lit).cast::<u32>(), (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.wrapping_offset(1) <= (*s).out_end) {
                cp_set_error!("Attempted to overwrite out buffer while outputting a symbol.");
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.wrapping_offset(1);
        } else if symbol > 256 {
            symbol = symbol.wrapping_sub(257);
            let length: c_int = cp_read_bits(
                s,
                *len_extra_bits_ptr().wrapping_offset(symbol as isize) as c_int,
            )
            .wrapping_add(*len_base_ptr().wrapping_offset(symbol as isize)) as c_int;
            let distance_symbol = cp_decode(s, (&raw mut (*s).dst).cast::<u32>(), (*s).ndst as c_int);
            let backwards_distance: c_int = cp_read_bits(
                s,
                *dist_extra_bits_ptr().wrapping_offset(distance_symbol as isize) as c_int,
            )
            .wrapping_add(*dist_base_ptr().wrapping_offset(distance_symbol as isize))
                as c_int;
            if !((*s)
                .out
                .wrapping_offset(-(backwards_distance as isize))
                >= (*s).begin)
            {
                cp_set_error!("Attempted to write before out buffer (invalid backwards distance).");
                return 0;
            }
            if !((*s).out.wrapping_offset(length as isize) <= (*s).out_end) {
                cp_set_error!("Attempted to overwrite out buffer while outputting a string.");
                return 0;
            }
            let mut src = (*s).out.wrapping_offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.wrapping_offset(length as isize);
            match backwards_distance {
                1 => {
                    memset(
                        dst as *mut c_void,
                        *src as c_int,
                        length as usize,
                    );
                }
                _ => {
                    let mut length = length;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let s = calloc(1, core::mem::size_of::<cp_state_t>()) as *mut cp_state_t;
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let in_addr = input as usize;
    let first_bytes: c_int = (((in_addr.wrapping_add(3)) & !3usize).wrapping_sub(in_addr)) as c_int;
    (*s).words = (input as *mut c_char).wrapping_offset(first_bytes as isize) as *mut u32;
    (*s).word_count = in_bytes.wrapping_sub(first_bytes) / 4;
    let last_bytes: c_int = in_bytes.wrapping_sub(first_bytes) & 3;
    {
        let mut i: c_int = 0;
        while i < first_bytes {
            let byte = *(input as *const u8).wrapping_offset(i as isize);
            (*s).bits |= (byte as u64).wrapping_shl((i.wrapping_mul(8)) as u32);
            i = i.wrapping_add(1);
        }
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    {
        let mut i: c_int = 0;
        while i < last_bytes {
            let byte = *(input as *const u8)
                .wrapping_offset((in_bytes.wrapping_sub(last_bytes).wrapping_add(i)) as isize);
            (*s).final_word |= (byte as u32).wrapping_shl((i.wrapping_mul(8)) as u32);
            i = i.wrapping_add(1);
        }
    }
    (*s).count = first_bytes.wrapping_mul(8);
    (*s).out = out as *mut c_char;
    (*s).out_end = (*s).out.wrapping_offset(out_bytes as isize);
    (*s).begin = out as *mut c_char;

    let ok = cp_inflate_blocks(s);
    free(s as *mut c_void);
    ok
}

unsafe fn cp_inflate_blocks(s: *mut cp_state_t) -> c_int {
    let mut count: c_int = 0;
    let mut bfinal: c_int;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
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
                cp_set_error!("Detected unknown block type within input stream.");
                return 0;
            }
            _ => {}
        }
        count = count.wrapping_add(1);
        if bfinal != 0 {
            break;
        }
    }
    let _ = count;
    1
}

// ---------------------------------------------------------------------------
// PNG loading
// ---------------------------------------------------------------------------

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

unsafe fn cp_make32(s: *const u8) -> u32 {
    ((*s.wrapping_offset(0) as u32) << 24)
        | ((*s.wrapping_offset(1) as u32) << 16)
        | ((*s.wrapping_offset(2) as u32) << 8)
        | (*s.wrapping_offset(3) as u32)
}

/// `memcmp(a, b, n) == 0`.  All `n` bytes are always read (just like the block
/// wise `memcmp()` of libc does) before the comparison result is formed.
unsafe fn cp_mem_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    let mut eq = true;
    let mut i = 0usize;
    while i < n {
        let x = ptr::read_volatile(a.wrapping_add(i));
        let y = *b.wrapping_add(i);
        eq &= x == y;
        i += 1;
    }
    eq
}

unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: *const u8, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if cp_mem_eq(start.wrapping_offset(4), chunk, 4) && len >= minlen {
        let offset: c_int = len.wrapping_add(12) as c_int;
        if (*png).p.wrapping_offset(offset as isize) <= (*png).end {
            (*png).p = (*png).p.wrapping_offset(offset as isize);
            return start.wrapping_offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const u8, minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        // `png->p += len + 12;` -- unsigned (zero extended) pointer arithmetic
        (*png).p = (*png).p.wrapping_add(len.wrapping_add(12) as usize);
        if cp_mem_eq(start.wrapping_offset(4), chunk, 4) && len >= minlen && (*png).p <= (*png).end {
            return start.wrapping_offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len: c_int = w.wrapping_mul(bpp);
    let mut raw = raw;
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
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    let v = *raw.wrapping_offset((x.wrapping_sub(bpp)) as isize) / 2;
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let v = cp_paeth(*raw.wrapping_offset((x.wrapping_sub(bpp)) as isize), 0, 0);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            _ => return 0,
        }
    }
    let mut prev = raw;
    raw = raw.wrapping_offset(len as isize);
    let mut y: c_int = 1;
    while y < h {
        let filter = *raw;
        raw = raw.wrapping_offset(1);
        match filter {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(0);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = *raw.wrapping_offset((x.wrapping_sub(bpp)) as isize);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    let v = *prev.wrapping_offset(x as isize);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = *prev.wrapping_offset(x as isize);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    let v = *prev.wrapping_offset(x as isize) / 2;
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let a = *raw.wrapping_offset((x.wrapping_sub(bpp)) as isize) as c_int;
                    let b = *prev.wrapping_offset(x as isize) as c_int;
                    let v = (a.wrapping_add(b) / 2) as u8;
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    let v = *prev.wrapping_offset(x as isize);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = cp_paeth(
                        *raw.wrapping_offset((x.wrapping_sub(bpp)) as isize),
                        *prev.wrapping_offset(x as isize),
                        *prev.wrapping_offset((x.wrapping_sub(bpp)) as isize),
                    );
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
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

unsafe fn cp_convert(bpp: c_int, w: c_int, h: c_int, src: *mut u8, dst: *mut cp_pixel_t) {
    let mut src = src;
    let mut dst = dst;
    let mut y: c_int = 0;
    while y < h {
        src = src.wrapping_offset(1);
        let mut x: c_int = 0;
        while x < w {
            match bpp {
                1 => {
                    *dst = cp_make_pixel(*src, *src, *src);
                    dst = dst.wrapping_offset(1);
                }
                2 => {
                    *dst = cp_make_pixel_a(*src, *src, *src, *src.wrapping_offset(1));
                    dst = dst.wrapping_offset(1);
                }
                3 => {
                    *dst = cp_make_pixel(
                        *src,
                        *src.wrapping_offset(1),
                        *src.wrapping_offset(2),
                    );
                    dst = dst.wrapping_offset(1);
                }
                4 => {
                    *dst = cp_make_pixel_a(
                        *src,
                        *src.wrapping_offset(1),
                        *src.wrapping_offset(2),
                        *src.wrapping_offset(3),
                    );
                    dst = dst.wrapping_offset(1);
                }
                _ => {}
            }
            x = x.wrapping_add(1);
            src = src.wrapping_offset(bpp as isize);
        }
        y = y.wrapping_add(1);
    }
}

unsafe fn cp_get_alpha_for_indexed_image(index: c_int, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() {
        255
    } else if (index as u32) >= trns_len {
        255
    } else {
        *trns.wrapping_offset(index as isize)
    }
}

unsafe fn cp_depalette(
    w: c_int,
    h: c_int,
    src: *mut u8,
    dst: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    let mut src = src;
    let mut dst = dst;
    let mut y: c_int = 0;
    while y < h {
        src = src.wrapping_offset(1);
        let mut x: c_int = 0;
        while x < w {
            let c = *src as c_int;
            let r = *plte.wrapping_offset((c.wrapping_mul(3)) as isize);
            let g = *plte.wrapping_offset((c.wrapping_mul(3).wrapping_add(1)) as isize);
            let b = *plte.wrapping_offset((c.wrapping_mul(3).wrapping_add(2)) as isize);
            let a = cp_get_alpha_for_indexed_image(c, trns, trns_len);
            *dst = cp_make_pixel_a(r, g, b, a);
            dst = dst.wrapping_offset(1);
            x = x.wrapping_add(1);
            src = src.wrapping_offset(1);
        }
        y = y.wrapping_add(1);
    }
}

unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    cp_make32(chunk.wrapping_offset(-8))
}

unsafe fn cp_out_size(img: *const cp_image_t, bpp: c_int) -> c_int {
    ((*img).w.wrapping_add(1))
        .wrapping_mul((*img).h)
        .wrapping_mul(bpp)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let mut img = cp_image_t {
        w: 0,
        h: 0,
        pix: ptr::null_mut(),
    };
    let mut data: *mut u8 = ptr::null_mut();
    let ok = load_png_mem_body(png_data, png_length, &mut img, &mut data);
    if ok {
        free(data as *mut c_void);
        return img;
    }
    free(data as *mut c_void);
    free(img.pix as *mut c_void);
    img.pix = ptr::null_mut();
    img
}

unsafe fn load_png_mem_body(
    png_data: *const u8,
    png_length: c_int,
    img: &mut cp_image_t,
    data_out: &mut *mut u8,
) -> bool {
    const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

    let ihdr: *const u8;
    let mut first: *const u8;
    let plte: *const u8;
    let trns: *const u8;
    let bit_depth: c_int;
    let color_type: c_int;
    let bpp: c_int;
    let w: c_int;
    let h: c_int;
    let pix_bytes: c_int;
    let compression: c_int;
    let filter: c_int;
    let interlace: c_int;
    let mut datalen: c_int;
    let mut offset: c_int;
    let out: *mut u8;

    let mut png = cp_raw_png_t {
        p: png_data,
        end: png_data.wrapping_offset(png_length as isize),
    };

    if !cp_mem_eq(png.p, SIG.as_ptr(), 8) {
        cp_set_error!("incorrect file signature (is this a png file?)");
        return false;
    }
    png.p = png.p.wrapping_offset(8);
    ihdr = cp_chunk(&mut png, b"IHDR".as_ptr(), 13);
    if ihdr.is_null() {
        cp_set_error!("unable to find IHDR chunk");
        return false;
    }
    bit_depth = *ihdr.wrapping_offset(8) as c_int;
    color_type = *ihdr.wrapping_offset(9) as c_int;
    if bit_depth != 8 {
        cp_set_error!("only bit-depth of 8 is supported");
        return false;
    }
    match color_type {
        0 => bpp = 1,
        2 => bpp = 3,
        3 => bpp = 1,
        4 => bpp = 2,
        6 => bpp = 4,
        _ => {
            cp_set_error!("unknown color type");
            return false;
        }
    }
    w = cp_make32(ihdr).wrapping_add(1) as c_int;
    h = cp_make32(ihdr.wrapping_offset(4)) as c_int;
    if !(w >= 1) {
        cp_set_error!("invalid IHDR chunk found, image width was less than 1");
        return false;
    }
    if !(h >= 1) {
        cp_set_error!("invalid IHDR chunk found, image height was less than 1");
        return false;
    }
    // `(int64_t)w * h * sizeof(cp_pixel_t) < INT_MAX` -- the multiplication with
    // `sizeof` makes the whole expression unsigned.
    if !((((w as i64).wrapping_mul(h as i64)) as u64)
        .wrapping_mul(core::mem::size_of::<cp_pixel_t>() as u64)
        < c_int::MAX as u64)
    {
        cp_set_error!("image too large");
        return false;
    }
    pix_bytes = ((w as i64).wrapping_mul(h as i64) as u64)
        .wrapping_mul(core::mem::size_of::<cp_pixel_t>() as u64) as c_int;
    img.w = w.wrapping_sub(1);
    img.h = h;
    img.pix = malloc(pix_bytes as usize) as *mut cp_pixel_t;
    if img.pix.is_null() {
        cp_set_error!("unable to allocate raw image space");
        return false;
    }
    compression = *ihdr.wrapping_offset(10) as c_int;
    filter = *ihdr.wrapping_offset(11) as c_int;
    interlace = *ihdr.wrapping_offset(12) as c_int;
    if compression != 0 {
        cp_set_error!("only standard compression DEFLATE is supported");
        return false;
    }
    if filter != 0 {
        cp_set_error!("only standard adaptive filtering is supported");
        return false;
    }
    if interlace != 0 {
        cp_set_error!("interlacing is not supported");
        return false;
    }
    first = png.p;
    plte = cp_find(&mut png, b"PLTE".as_ptr(), 0);
    if plte.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    trns = cp_find(&mut png, b"tRNS".as_ptr(), 0);
    if trns.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    datalen = 0;
    {
        let mut idat = cp_find(&mut png, b"IDAT".as_ptr(), 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat);
            datalen = datalen.wrapping_add(len as c_int);
            idat = cp_chunk(&mut png, b"IDAT".as_ptr(), 0);
        }
    }
    png.p = first;
    let data = malloc(datalen as usize) as *mut u8;
    *data_out = data;
    offset = 0;
    {
        let mut idat = cp_find(&mut png, b"IDAT".as_ptr(), 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat);
            memcpy(
                data.wrapping_offset(offset as isize) as *mut c_void,
                idat as *const c_void,
                len as usize,
            );
            offset = offset.wrapping_add(len as c_int);
            idat = cp_chunk(&mut png, b"IDAT".as_ptr(), 0);
        }
    }
    if !(!data.is_null() && datalen >= 6) {
        cp_set_error!("corrupt zlib structure in DEFLATE stream");
        return false;
    }
    if !((*data & 0x0f) == 0x08) {
        cp_set_error!("only zlib compression method (RFC 1950) is supported");
        return false;
    }
    if !((*data & 0xf0) <= 0x70) {
        cp_set_error!("innapropriate window size detected");
        return false;
    }
    if !((*data.wrapping_offset(1) & 0x20) == 0) {
        cp_set_error!("preset dictionary is present and not supported");
        return false;
    }
    if !(cp_out_size(img, 4) >= 1) {
        cp_set_error!("invalid image size found");
        return false;
    }
    if !(cp_out_size(img, bpp) >= 1) {
        cp_set_error!("invalid image size found");
        return false;
    }
    out = (img.pix as *mut u8)
        .wrapping_offset(cp_out_size(img, 4) as isize)
        .wrapping_offset(-(cp_out_size(img, bpp) as isize));
    if cp_inflate(
        data.wrapping_offset(2) as *mut c_void,
        datalen.wrapping_sub(6),
        out as *mut c_void,
        pix_bytes,
    ) == 0
    {
        cp_set_error!("DEFLATE algorithm failed");
        return false;
    }
    if cp_unfilter(img.w, img.h, bpp, out) == 0 {
        cp_set_error!("invalid filter byte found");
        return false;
    }
    if color_type == 3 {
        if plte.is_null() {
            cp_set_error!("color type of indexed requires a PLTE chunk");
            return false;
        }
        let trns_len: u32 = if !trns.is_null() {
            cp_get_chunk_byte_length(trns)
        } else {
            0
        };
        cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
    } else {
        cp_convert(bpp, img.w, img.h, out, img.pix);
    }
    true
}
