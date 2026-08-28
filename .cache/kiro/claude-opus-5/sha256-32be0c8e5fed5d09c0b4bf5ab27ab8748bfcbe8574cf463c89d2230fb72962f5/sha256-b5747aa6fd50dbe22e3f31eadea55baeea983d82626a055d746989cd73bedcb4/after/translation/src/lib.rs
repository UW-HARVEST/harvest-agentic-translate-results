//! Rust translation of the C PNG loader in `c_src/`.
//!
//! The translation is intentionally literal: pointer arithmetic, integer
//! truncation/wrapping and the (sometimes buggy) order of validation checks are
//! all reproduced exactly so that the observable behaviour is byte-identical to
//! the original C.
//!
//! `assert()` from the C sources is treated as compiled out (the reference
//! shared library is built as a release/NDEBUG artifact), so the asserts are
//! kept only as comments.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

/* ------------------------------------------------------------------------- */
/* public types                                                              */
/* ------------------------------------------------------------------------- */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

/* ------------------------------------------------------------------------- */
/* globals (exported with C linkage, exactly as in the C source)             */
/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

/// The literal in the C source is 144x8, 112x9, 24x7, 8x8, 32x5.
const fn cp_make_fixed_table() -> [u8; 288 + 32] {
    let mut t = [0u8; 288 + 32];
    let mut i = 0usize;
    while i < 288 + 32 {
        t[i] = if i < 144 {
            8
        } else if i < 256 {
            9
        } else if i < 280 {
            7
        } else if i < 288 {
            8
        } else {
            5
        };
        i += 1;
    }
    t
}

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = cp_make_fixed_table();

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

#[inline]
unsafe fn u8_at(base: *const u8, i: c_int) -> u8 {
    *base.offset(i as isize)
}

#[inline]
unsafe fn u32_at(base: *const u32, i: c_int) -> u32 {
    *base.offset(i as isize)
}

/* ------------------------------------------------------------------------- */
/* inflate                                                                   */
/* ------------------------------------------------------------------------- */

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

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    /* assert(!(s->bits_left & 7)); */
    ((*s).words.offset((*s).word_index as isize) as *mut c_char)
        .offset(-(((*s).count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.offset((*s).word_index as isize);
            (*s).word_index += 1;
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count += 32;
            /* assert(s->word_index <= s->word_count); */
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count += (*s).bits_left;
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    /* assert(s->count >= num_bits_to_read); */
    let mask = 1u64.wrapping_shl(num_bits_to_read as u32).wrapping_sub(1);
    let bits = ((*s).bits & mask) as u32;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count = (*s).count.wrapping_sub(num_bits_to_read);
    (*s).bits_left = (*s).bits_left.wrapping_sub(num_bits_to_read);
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    /* asserts elided (NDEBUG) */
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

/// `counts`/`codes`/`first` are oversized relative to the C originals so that a
/// corrupt code-length table cannot make the Rust build panic where the C would
/// merely scribble over adjacent stack slots.
unsafe fn cp_build(s: *mut cp_state_t, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0i32; 256];
    let mut first = [0i32; 256];
    let mut counts = [0i32; 256];

    let mut n = 0;
    while n < sym_count {
        let idx = u8_at(lens, n) as usize;
        counts[idx] = counts[idx].wrapping_add(1);
        n += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1].wrapping_add(counts[n - 1])) << 1;
        first[n] = first[n - 1].wrapping_add(counts[n - 1]);
    }

    if !s.is_null() {
        ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, 1 << 9);
    }

    let mut i = 0;
    while i < sym_count {
        let len = u8_at(lens, i) as c_int;
        if len != 0 {
            /* assert(len < 16); */
            let code = codes[len as usize] as u32;
            codes[len as usize] = codes[len as usize].wrapping_add(1);
            let slot = first[len as usize] as u32;
            first[len as usize] = first[len as usize].wrapping_add(1);
            *tree.offset(slot as isize) = code.wrapping_shl((32 - len) as u32)
                | ((i as u32) << 4)
                | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    (*s).lookup[j as usize] = ((len << 9) | i) as u16;
                    j += 1 << len;
                }
            }
        }
        i += 1;
    }

    first[15]
}

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        cp_error_reason = cstr!(
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."
        );
        return 0;
    }
    if !((*s).bits_left / 8 <= LEN as c_int) {
        cp_error_reason = cstr!("Stored block extends beyond end of input stream.");
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, (*s).out, LEN as usize);
    (*s).out = (*s).out.offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    let table = (&raw mut cp_fixed_table) as *mut u8;
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), table, 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        table.offset(288),
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
        let guess = (lo + hi) >> 1;
        if search < *tree.offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.offset((lo - 1) as isize);
    /* assert((search >> (32 - (key & 0xF))) == (key >> (32 - (key & 0xF)))); */
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit: c_int = 257i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let ndst: c_int = 1i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let nlen: c_int = 4i32.wrapping_add(cp_read_bits(s, 4) as c_int);
    let perm = (&raw const cp_permutation_order) as *const u8;
    for i in 0..nlen {
        lenlens[u8_at(perm, i) as usize] = cp_read_bits(s, 3) as u8;
    }
    (*s).nlen = cp_build(ptr::null_mut(), (*s).len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;

    /* `uint8_t lens[288 + 32]` in C; padded here so that the out-of-bounds
     * accesses a corrupt stream can produce stay inside an allocation. */
    let mut lens_backing = [0u8; 64 + 288 + 32 + 256];
    let lens = lens_backing.as_mut_ptr().add(64);

    let mut n: c_int = 0;
    while n < nlit.wrapping_add(ndst) {
        let sym = cp_decode(s, (*s).len.as_mut_ptr(), (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i = 3i32.wrapping_add(cp_read_bits(s, 2) as c_int);
                while i != 0 {
                    *lens.offset(n as isize) = *lens.offset((n - 1) as isize);
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3i32.wrapping_add(cp_read_bits(s, 3) as c_int);
                while i != 0 {
                    *lens.offset(n as isize) = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11i32.wrapping_add(cp_read_bits(s, 7) as c_int);
                while i != 0 {
                    *lens.offset(n as isize) = 0;
                    i -= 1;
                    n += 1;
                }
            }
            _ => {
                *lens.offset(n as isize) = sym as u8;
                n += 1;
            }
        }
    }

    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), lens, nlit) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        lens.offset(nlit as isize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    let len_extra = (&raw const cp_len_extra_bits) as *const u8;
    let len_base = (&raw const cp_len_base) as *const u32;
    let dist_extra = (&raw const cp_dist_extra_bits) as *const u8;
    let dist_base = (&raw const cp_dist_base) as *const u32;
    loop {
        let mut symbol = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.offset(1) <= (*s).out_end) {
                cp_error_reason =
                    cstr!("Attempted to overwrite out buffer while outputting a symbol.");
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.offset(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length: c_int = (cp_read_bits(s, u8_at(len_extra, symbol) as c_int)
                .wrapping_add(u32_at(len_base, symbol))) as c_int;
            let distance_symbol = cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as c_int);
            let backwards_distance: c_int =
                (cp_read_bits(s, u8_at(dist_extra, distance_symbol) as c_int)
                    .wrapping_add(u32_at(dist_base, distance_symbol))) as c_int;
            if !((*s).out.wrapping_offset(-(backwards_distance as isize)) >= (*s).begin) {
                cp_error_reason =
                    cstr!("Attempted to write before out buffer (invalid backwards distance).");
                return 0;
            }
            if !((*s).out.wrapping_offset(length as isize) <= (*s).out_end) {
                cp_error_reason =
                    cstr!("Attempted to overwrite out buffer while outputting a string.");
                return 0;
            }
            let mut src = (*s).out.wrapping_offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.wrapping_offset(length as isize);
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst, *src as u8, length as usize);
                }
                _ => {
                    let mut length = length;
                    while {
                        let t = length;
                        length = length.wrapping_sub(1);
                        t != 0
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    in_: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let s = calloc(1, core::mem::size_of::<cp_state_t>()) as *mut cp_state_t;
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let in_addr = in_ as usize;
    let first_bytes = ((in_addr.wrapping_add(3) & !3usize).wrapping_sub(in_addr)) as c_int;
    (*s).words = (in_ as *mut c_char).offset(first_bytes as isize) as *mut u32;
    (*s).word_count = (in_bytes.wrapping_sub(first_bytes)) / 4;
    let last_bytes = in_bytes.wrapping_sub(first_bytes) & 3;
    let in_u8 = in_ as *const u8;
    for i in 0..first_bytes {
        (*s).bits |= (u8_at(in_u8, i) as u64) << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        (*s).final_word |=
            (u8_at(in_u8, in_bytes.wrapping_sub(last_bytes).wrapping_add(i)) as u32) << (i * 8);
    }
    (*s).count = first_bytes.wrapping_mul(8);
    (*s).out = out as *mut c_char;
    (*s).out_end = (*s).out.wrapping_offset(out_bytes as isize);
    (*s).begin = out as *mut c_char;

    let mut count: c_int = 0;
    let mut bfinal: c_int;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    free(s as *mut c_void);
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    free(s as *mut c_void);
                    return 0;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    free(s as *mut c_void);
                    return 0;
                }
            }
            3 => {
                cp_error_reason = cstr!("Detected unknown block type within input stream.");
                free(s as *mut c_void);
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
    free(s as *mut c_void);
    1
}

/* ------------------------------------------------------------------------- */
/* png                                                                       */
/* ------------------------------------------------------------------------- */

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: c_int = a as c_int + b as c_int - c as c_int;
    let pa = (p - a as c_int).abs();
    let pb = (p - b as c_int).abs();
    let pc = (p - c as c_int).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

struct cp_raw_png_t {
    p: *const u8,
    end: *const u8,
}

unsafe fn cp_make32(s: *const u8) -> u32 {
    ((*s.offset(0) as u32) << 24)
        | ((*s.offset(1) as u32) << 16)
        | ((*s.offset(2) as u32) << 8)
        | (*s.offset(3) as u32)
}

unsafe fn cp_memcmp4(a: *const u8, b: *const u8) -> bool {
    for i in 0..4isize {
        if *a.offset(i) != *b.offset(i) {
            return false;
        }
    }
    true
}

unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: *const u8, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if cp_memcmp4(start.offset(4), chunk) && len >= minlen {
        let offset: c_int = len.wrapping_add(12) as c_int;
        if (*png).p.wrapping_offset(offset as isize) <= (*png).end {
            (*png).p = (*png).p.wrapping_offset(offset as isize);
            return start.offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const u8, minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.wrapping_add(len.wrapping_add(12) as usize);
        if cp_memcmp4(start.offset(4), chunk) && len >= minlen && (*png).p <= (*png).end {
            return start.offset(8);
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
        raw = raw.offset(1);
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    let v = *raw.offset((x - bpp) as isize);
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    let v = *raw.offset((x - bpp) as isize) / 2;
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let v = cp_paeth(*raw.offset((x - bpp) as isize), 0, 0);
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x += 1;
                }
            }
            _ => return 0,
        }
    }
    let mut prev: *mut u8 = raw;
    raw = raw.wrapping_offset(len as isize);
    let mut y: c_int = 1;
    while y < h {
        let filter = *raw;
        raw = raw.offset(1);
        match filter {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(0);
                    x += 1;
                }
                while x < len {
                    let v = *raw.offset((x - bpp) as isize);
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    let v = *prev.offset(x as isize);
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let v = *prev.offset(x as isize);
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    let v = *prev.offset(x as isize) / 2;
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let a = *raw.offset((x - bpp) as isize) as c_int;
                    let b = *prev.offset(x as isize) as c_int;
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(((a + b) / 2) as u8);
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    let v = *prev.offset(x as isize);
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let v = cp_paeth(
                        *raw.offset((x - bpp) as isize),
                        *prev.offset(x as isize),
                        *prev.offset((x - bpp) as isize),
                    );
                    let d = raw.offset(x as isize);
                    *d = (*d).wrapping_add(v);
                    x += 1;
                }
            }
            _ => return 0,
        }
        y += 1;
        prev = raw;
        raw = raw.wrapping_offset(len as isize);
    }
    1
}

unsafe fn cp_convert(bpp: c_int, w: c_int, h: c_int, src: *mut u8, dst: *mut cp_pixel_t) {
    let mut src = src;
    let mut dst = dst;
    for _y in 0..h {
        src = src.offset(1);
        let mut x: c_int = 0;
        while x < w {
            match bpp {
                1 => {
                    *dst = cp_make_pixel(*src.offset(0), *src.offset(0), *src.offset(0));
                    dst = dst.offset(1);
                }
                2 => {
                    *dst = cp_make_pixel_a(
                        *src.offset(0),
                        *src.offset(0),
                        *src.offset(0),
                        *src.offset(1),
                    );
                    dst = dst.offset(1);
                }
                3 => {
                    *dst = cp_make_pixel(*src.offset(0), *src.offset(1), *src.offset(2));
                    dst = dst.offset(1);
                }
                4 => {
                    *dst = cp_make_pixel_a(
                        *src.offset(0),
                        *src.offset(1),
                        *src.offset(2),
                        *src.offset(3),
                    );
                    dst = dst.offset(1);
                }
                _ => {}
            }
            x += 1;
            src = src.wrapping_offset(bpp as isize);
        }
    }
}

unsafe fn cp_get_alpha_for_indexed_image(index: c_int, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() {
        255
    } else if (index as u32) >= trns_len {
        255
    } else {
        *trns.offset(index as isize)
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
    for _y in 0..h {
        src = src.offset(1);
        let mut x: c_int = 0;
        while x < w {
            let c = *src as c_int;
            let r = *plte.offset((c * 3) as isize);
            let g = *plte.offset((c * 3 + 1) as isize);
            let b = *plte.offset((c * 3 + 2) as isize);
            let a = cp_get_alpha_for_indexed_image(c, trns, trns_len);
            *dst = cp_make_pixel_a(r, g, b, a);
            dst = dst.offset(1);
            x += 1;
            src = src.offset(1);
        }
    }
}

unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    cp_make32(chunk.offset(-8))
}

fn cp_out_size(img: &cp_image_t, bpp: c_int) -> c_int {
    (img.w.wrapping_add(1))
        .wrapping_mul(img.h)
        .wrapping_mul(bpp)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let sig: *const u8 = b"\x89PNG\r\n\x1a\n\0".as_ptr();
    let mut img = cp_image_t {
        w: 0,
        h: 0,
        pix: ptr::null_mut(),
    };
    let mut data: *mut u8 = ptr::null_mut();

    let mut png = cp_raw_png_t {
        p: png_data,
        end: png_data.wrapping_offset(png_length as isize),
    };

    'err: {
        // signature (8 bytes, unchecked read as in the C)
        let mut sig_ok = true;
        for i in 0..8isize {
            if *png.p.offset(i) != *sig.offset(i) {
                sig_ok = false;
                break;
            }
        }
        if !sig_ok {
            cp_error_reason = cstr!("incorrect file signature (is this a png file?)");
            break 'err;
        }
        png.p = png.p.offset(8);

        let ihdr = cp_chunk(&mut png, b"IHDR".as_ptr(), 13);
        if ihdr.is_null() {
            cp_error_reason = cstr!("unable to find IHDR chunk");
            break 'err;
        }

        let bit_depth = *ihdr.offset(8) as c_int;
        let color_type = *ihdr.offset(9) as c_int;
        if bit_depth != 8 {
            cp_error_reason = cstr!("only bit-depth of 8 is supported");
            break 'err;
        }

        let bpp: c_int = match color_type {
            0 => 1,
            2 => 3,
            3 => 1,
            4 => 2,
            6 => 4,
            _ => {
                cp_error_reason = cstr!("unknown color type");
                break 'err;
            }
        };

        let w: c_int = cp_make32(ihdr).wrapping_add(1) as c_int;
        let h: c_int = cp_make32(ihdr.offset(4)) as c_int;
        if !(w >= 1) {
            cp_error_reason = cstr!("invalid IHDR chunk found, image width was less than 1");
            break 'err;
        }
        if !(h >= 1) {
            cp_error_reason = cstr!("invalid IHDR chunk found, image height was less than 1");
            break 'err;
        }
        if !(((w as i64).wrapping_mul(h as i64) as u64).wrapping_mul(4) < c_int::MAX as u64) {
            cp_error_reason = cstr!("image too large");
            break 'err;
        }
        let pix_bytes: c_int =
            ((w.wrapping_mul(h) as i64 as u64).wrapping_mul(4) & 0xFFFF_FFFF) as u32 as c_int;
        img.w = w - 1;
        img.h = h;
        img.pix = malloc(pix_bytes as isize as usize) as *mut cp_pixel_t;
        if img.pix.is_null() {
            cp_error_reason = cstr!("unable to allocate raw image space");
            break 'err;
        }

        let compression = *ihdr.offset(10) as c_int;
        let filter = *ihdr.offset(11) as c_int;
        let interlace = *ihdr.offset(12) as c_int;
        if compression != 0 {
            cp_error_reason = cstr!("only standard compression DEFLATE is supported");
            break 'err;
        }
        if filter != 0 {
            cp_error_reason = cstr!("only standard adaptive filtering is supported");
            break 'err;
        }
        if interlace != 0 {
            cp_error_reason = cstr!("interlacing is not supported");
            break 'err;
        }

        let mut first = png.p;
        let plte = cp_find(&mut png, b"PLTE".as_ptr(), 0);
        if plte.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }
        let trns = cp_find(&mut png, b"tRNS".as_ptr(), 0);
        if trns.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }

        let mut datalen: c_int = 0;
        let mut idat = cp_find(&mut png, b"IDAT".as_ptr(), 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat);
            datalen = datalen.wrapping_add(len as c_int);
            idat = cp_chunk(&mut png, b"IDAT".as_ptr(), 0);
        }
        png.p = first;
        data = malloc(datalen as isize as usize) as *mut u8;
        let mut offset: c_int = 0;
        let mut idat = cp_find(&mut png, b"IDAT".as_ptr(), 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat);
            ptr::copy_nonoverlapping(idat, data.wrapping_offset(offset as isize), len as usize);
            offset = offset.wrapping_add(len as c_int);
            idat = cp_chunk(&mut png, b"IDAT".as_ptr(), 0);
        }

        if !(!data.is_null() && datalen >= 6) {
            cp_error_reason = cstr!("corrupt zlib structure in DEFLATE stream");
            break 'err;
        }
        if !((*data.offset(0) & 0x0f) == 0x08) {
            cp_error_reason = cstr!("only zlib compression method (RFC 1950) is supported");
            break 'err;
        }
        if !((*data.offset(0) & 0xf0) <= 0x70) {
            cp_error_reason = cstr!("innapropriate window size detected");
            break 'err;
        }
        if (*data.offset(1) & 0x20) != 0 {
            cp_error_reason = cstr!("preset dictionary is present and not supported");
            break 'err;
        }

        if !(cp_out_size(&img, 4) >= 1) {
            cp_error_reason = cstr!("invalid image size found");
            break 'err;
        }
        if !(cp_out_size(&img, bpp) >= 1) {
            cp_error_reason = cstr!("invalid image size found");
            break 'err;
        }

        let out = (img.pix as *mut u8).wrapping_offset(
            (cp_out_size(&img, 4).wrapping_sub(cp_out_size(&img, bpp))) as isize,
        );

        if cp_inflate(
            data.wrapping_offset(2) as *mut c_void,
            datalen.wrapping_sub(6),
            out as *mut c_void,
            pix_bytes,
        ) == 0
        {
            cp_error_reason = cstr!("DEFLATE algorithm failed");
            break 'err;
        }

        if cp_unfilter(img.w, img.h, bpp, out) == 0 {
            cp_error_reason = cstr!("invalid filter byte found");
            break 'err;
        }

        if color_type == 3 {
            if plte.is_null() {
                cp_error_reason = cstr!("color type of indexed requires a PLTE chunk");
                break 'err;
            }
            let trns_len = if !trns.is_null() {
                cp_get_chunk_byte_length(trns)
            } else {
                0
            };
            cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
        } else {
            cp_convert(bpp, img.w, img.h, out, img.pix);
        }

        free(data as *mut c_void);
        return img;
    }

    free(data as *mut c_void);
    free(img.pix as *mut c_void);
    img.pix = ptr::null_mut();
    img
}
