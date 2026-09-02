//! Rust translation of c_src/src/lib.c (cute_png style DEFLATE + PNG unfilter).
//!
//! The translation is intentionally literal: it reproduces the exact control
//! flow, arithmetic (including wrapping / truncation), error-check ordering and
//! out-of-bounds behaviour of the original C. `assert()` is a no-op here, which
//! matches an `NDEBUG` (release) build of the C library.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_assignments)]
#![allow(unused_variables)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// Public (exported) globals
// ---------------------------------------------------------------------------

/// `const char *cp_error_reason;`
#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

/// `uint8_t cp_fixed_table[288 + 32]`
#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = [
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5,
];

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

#[inline]
unsafe fn set_error(msg: &'static [u8]) {
    // The C code assigns string literals to cp_error_reason.
    *ptr::addr_of_mut!(cp_error_reason) = msg.as_ptr() as *const c_char;
}

// ---------------------------------------------------------------------------
// Internal types (mirroring the C layout exactly)
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
#[derive(Copy, Clone)]
#[allow(dead_code)]
struct cp_image_t {
    w: c_int,
    h: c_int,
    pix: *mut cp_pixel_t,
}

// `static` helpers in the C file; unused there as well (kept for fidelity).
#[allow(dead_code)]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

#[allow(dead_code)]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

/// Mirror of `cp_state_t`. `#[repr(C)]` guarantees the same field offsets as
/// the C struct, which matters because `cp_decode` reads `tree[-1]`.
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

// ---------------------------------------------------------------------------
// Bit reader
// ---------------------------------------------------------------------------

unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    // assert(!(s->bits_left & 7));
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
            // assert(s->word_index <= s->word_count);
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
    // assert(s->count >= num_bits_to_read);
    let mask = (1u64.wrapping_shl(num_bits_to_read as u32)).wrapping_sub(1);
    let bits = (*s).bits & mask;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits as u32
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
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
// Huffman table construction / decoding
// ---------------------------------------------------------------------------

unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    // The C declares `int codes[16], first[16], counts[16] = {0}` and then does
    // `counts[lens[n]]++`. `lens[n]` is a `uint8_t`, so a malformed stream (or a
    // caller that corrupts the exported `cp_fixed_table`) can drive the index to
    // 255 and write past the end of a 16-int stack array -- undefined behaviour
    // in C, and something the C's own `assert(len < 16)` only catches *after*
    // the write. Backing these with 256 entries keeps every in-range (len < 16)
    // result bit-identical while making the out-of-range case total instead of
    // a bounds-check panic: aborting would be a far larger divergence, since the
    // C does not abort here under NDEBUG.
    let mut codes: [c_int; 256] = [0; 256];
    let mut first: [c_int; 256] = [0; 256];
    let mut counts: [c_int; 256] = [0; 256];

    let mut n: c_int = 0;
    while n < sym_count {
        counts[*lens.offset(n as isize) as usize] += 1;
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
        ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, 1 << 9);
    }

    let mut i: c_int = 0;
    while i < sym_count {
        let len = *lens.offset(i as isize) as c_int;
        if len != 0 {
            // assert(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] = codes[len as usize].wrapping_add(1);
            let slot = first[len as usize] as u32;
            first[len as usize] = first[len as usize].wrapping_add(1);
            // `uint32_t slot` indexes `tree` in C, so the offset is
            // zero-extended, not sign-extended.
            *tree.add(slot as usize) =
                (code.wrapping_shl((32 - len) as u32)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j: c_int = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    *(*s).lookup.as_mut_ptr().offset(j as isize) =
                        (((len as u32) << 9) | (i as u32)) as u16;
                    j += 1 << len;
                }
            }
        }
        i += 1;
    }

    first[15]
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo.wrapping_add(hi)) >> 1;
        if search < *tree.offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.offset((lo - 1) as isize);
    // assert((search >> len) == (key >> len));
    let code = cp_consume_bits(s, (key & 0xF) as c_int);
    let _ = code;
    ((key >> 4) & 0xFFF) as c_int
}

// ---------------------------------------------------------------------------
// Block decoders
// ---------------------------------------------------------------------------

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        set_error(
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0",
        );
        return 0;
    }
    if !((*s).bits_left / 8 <= LEN as c_int) {
        set_error(b"Stored block extends beyond end of input stream.\0");
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, LEN as usize);
    (*s).out = (*s).out.offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    let table = ptr::addr_of_mut!(cp_fixed_table) as *mut u8;
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), table, 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        table.offset(288),
        32,
    ) as u32;
    1
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    // `uint8_t lenlens[19] = {0};` indexed by `cp_permutation_order[i]`. That
    // table is exported and mutable, so a caller can make the index exceed 18
    // and the C would write past the array. Padding keeps that total in Rust.
    let mut lenlens_store: [u8; 19 + 256] = [0; 19 + 256];
    let lenlens: *mut u8 = lenlens_store.as_mut_ptr();
    let nlit: c_int = 257 + cp_read_bits(s, 5) as c_int;
    let ndst: c_int = 1 + cp_read_bits(s, 5) as c_int;
    let nlen: c_int = 4 + cp_read_bits(s, 4) as c_int;

    let perm = ptr::addr_of_mut!(cp_permutation_order) as *mut u8;
    let mut i: c_int = 0;
    while i < nlen {
        let idx = *perm.offset(i as isize) as usize;
        *lenlens.add(idx) = cp_read_bits(s, 3) as u8;
        i += 1;
    }
    (*s).nlen = cp_build(ptr::null_mut(), (*s).len.as_mut_ptr(), lenlens, 19) as u32;

    // `uint8_t lens[288 + 32];` -- the C array is uninitialised, and malformed
    // streams can index it out of range (including `lens[-1]`).  A padded,
    // zeroed backing store keeps that behaviour observable without UB.
    let mut lens_store: [u8; 8 + 288 + 32 + 512] = [0; 8 + 288 + 32 + 512];
    let lens: *mut u8 = lens_store.as_mut_ptr().add(8);

    let mut n: c_int = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, (*s).len.as_mut_ptr(), (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as c_int;
                while i != 0 {
                    *lens.offset(n as isize) = *lens.offset((n - 1) as isize);
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as c_int;
                while i != 0 {
                    *lens.offset(n as isize) = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as c_int;
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
    let len_extra = ptr::addr_of_mut!(cp_len_extra_bits) as *mut u8;
    let len_base = ptr::addr_of_mut!(cp_len_base) as *mut u32;
    let dist_extra = ptr::addr_of_mut!(cp_dist_extra_bits) as *mut u8;
    let dist_base = ptr::addr_of_mut!(cp_dist_base) as *mut u32;

    loop {
        let mut symbol = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.offset(1) <= (*s).out_end) {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.offset(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length: c_int = cp_read_bits(s, *len_extra.offset(symbol as isize) as c_int)
                .wrapping_add(*len_base.offset(symbol as isize)) as c_int;
            let distance_symbol = cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as c_int);
            let backwards_distance: c_int =
                cp_read_bits(s, *dist_extra.offset(distance_symbol as isize) as c_int)
                    .wrapping_add(*dist_base.offset(distance_symbol as isize)) as c_int;
            if !((*s).out.offset(-(backwards_distance as isize)) >= (*s).begin) {
                set_error(
                    b"Attempted to write before out buffer (invalid backwards distance).\0",
                );
                return 0;
            }
            if !((*s).out.offset(length as isize) <= (*s).out_end) {
                set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                return 0;
            }
            let mut src = (*s).out.offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.offset(length as isize);
            let mut length = length;
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
                }
                _ => loop {
                    let cond = length != 0;
                    length -= 1;
                    if !cond {
                        break;
                    }
                    *dst = *src;
                    dst = dst.offset(1);
                    src = src.offset(1);
                },
            }
        } else {
            break;
        }
    }
    1
}

// ---------------------------------------------------------------------------
// Public: cp_inflate
// ---------------------------------------------------------------------------

/// `int cp_inflate(void *in, int in_bytes, void *out, int out_bytes);`
#[unsafe(no_mangle)]
pub extern "C" fn cp_inflate(
    in_: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        let layout = core::alloc::Layout::new::<cp_state_t>();
        let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;
        if s.is_null() {
            // calloc failure in C would dereference NULL; nothing sensible to do.
            return 0;
        }

        (*s).bits = 0;
        (*s).count = 0;
        (*s).word_index = 0;
        (*s).bits_left = in_bytes.wrapping_mul(8);
        let addr = in_ as usize;
        let first_bytes = (((addr + 3) & !3usize).wrapping_sub(addr)) as c_int;
        (*s).words = (in_ as *mut c_char).offset(first_bytes as isize) as *mut u32;
        (*s).word_count = (in_bytes.wrapping_sub(first_bytes)) / 4;
        let last_bytes = (in_bytes.wrapping_sub(first_bytes)) & 3;
        let in_u8 = in_ as *const u8;
        let mut i: c_int = 0;
        while i < first_bytes {
            (*s).bits |= (*in_u8.offset(i as isize) as u64).wrapping_shl((i * 8) as u32);
            i += 1;
        }
        (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
        (*s).final_word = 0;
        i = 0;
        while i < last_bytes {
            (*s).final_word |= ((*in_u8.offset((in_bytes - last_bytes + i) as isize) as c_int)
                << (i * 8)) as u32;
            i += 1;
        }
        (*s).count = first_bytes.wrapping_mul(8);
        (*s).out = out as *mut c_char;
        (*s).out_end = (*s).out.offset(out_bytes as isize);
        (*s).begin = out as *mut c_char;

        let mut count: c_int = 0;
        let mut bfinal: c_int;
        let ok = loop {
            bfinal = cp_read_bits(s, 1) as c_int;
            let btype = cp_read_bits(s, 2) as c_int;
            match btype {
                0 => {
                    if cp_stored(s) == 0 {
                        break false;
                    }
                }
                1 => {
                    cp_fixed(s);
                    if cp_block(s) == 0 {
                        break false;
                    }
                }
                2 => {
                    cp_dynamic(s);
                    if cp_block(s) == 0 {
                        break false;
                    }
                }
                3 => {
                    set_error(b"Detected unknown block type within input stream.\0");
                    break false;
                }
                _ => {}
            }
            count += 1;
            if bfinal != 0 {
                break true;
            }
        };

        std::alloc::dealloc(s as *mut u8, layout);
        if ok {
            1
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// PNG helpers
// ---------------------------------------------------------------------------

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: c_int = (a as c_int) + (b as c_int) - (c as c_int);
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

#[repr(C)]
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

#[allow(dead_code)]
unsafe fn cp_chunk(
    png: *mut cp_raw_png_t,
    chunk: *const c_char,
    minlen: u32,
) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if memcmp4(start.offset(4), chunk) == 0 && len >= minlen {
        let offset = len.wrapping_add(12) as c_int;
        if (*png).p.offset(offset as isize) <= (*png).end {
            (*png).p = (*png).p.offset(offset as isize);
            return start.offset(8);
        }
    }
    ptr::null()
}

#[allow(dead_code)]
unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.offset(len.wrapping_add(12) as isize);
        if memcmp4(start.offset(4), chunk) == 0 && len >= minlen && (*png).p <= (*png).end {
            return start.offset(8);
        }
    }
    ptr::null()
}

/// `memcmp(a, b, 4)` (only the zero / non-zero result is used).
unsafe fn memcmp4(a: *const u8, b: *const c_char) -> c_int {
    let mut i = 0isize;
    while i < 4 {
        let x = *a.offset(i);
        let y = *(b.offset(i) as *const u8);
        if x != y {
            return x as c_int - y as c_int;
        }
        i += 1;
    }
    0
}

// ---------------------------------------------------------------------------
// Public: unfilter
// ---------------------------------------------------------------------------

/// `int unfilter(int w, int h, int bpp, uint8_t *raw);`
#[unsafe(no_mangle)]
pub extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    unsafe {
        let len: c_int = w.wrapping_mul(bpp);
        let mut raw = raw;
        let mut prev: *mut u8;
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
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                2 => {}
                3 => {
                    x = bpp;
                    while x < len {
                        let v = *raw.offset((x - bpp) as isize) / 2;
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                4 => {
                    x = bpp;
                    while x < len {
                        let v = cp_paeth(*raw.offset((x - bpp) as isize), 0, 0);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                _ => return 0,
            }
        }

        prev = raw;
        raw = raw.offset(len as isize);

        let mut y: c_int = 1;
        while y < h {
            let filter = *raw;
            raw = raw.offset(1);
            match filter {
                0 => {}
                1 => {
                    x = 0;
                    while x < bpp {
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(0);
                        x += 1;
                    }
                    while x < len {
                        let v = *raw.offset((x - bpp) as isize);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                2 => {
                    x = 0;
                    while x < bpp {
                        let v = *prev.offset(x as isize);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                    while x < len {
                        let v = *prev.offset(x as isize);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                3 => {
                    x = 0;
                    while x < bpp {
                        let v = *prev.offset(x as isize) / 2;
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                    while x < len {
                        let v = ((*raw.offset((x - bpp) as isize) as c_int
                            + *prev.offset(x as isize) as c_int)
                            / 2) as u8;
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                4 => {
                    x = 0;
                    while x < bpp {
                        let v = *prev.offset(x as isize);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                    while x < len {
                        let v = cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            *prev.offset(x as isize),
                            *prev.offset((x - bpp) as isize),
                        );
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                _ => return 0,
            }
            y += 1;
            prev = raw;
            raw = raw.offset(len as isize);
        }
        1
    }
}
