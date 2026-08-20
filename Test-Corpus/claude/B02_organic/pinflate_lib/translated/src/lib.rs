//! Rust translation of the C library in `c_src/` (a `pinflate` DEFLATE
//! decompressor derived from cute_png / cute_headers by Randy Gaul).
//!
//! The translation is intentionally literal: it mirrors the original control
//! flow, the exact order of validation checks, the exact error strings, and the
//! (buggy) semantics of the original code. `assert()` calls from the C source
//! are reproduced as comments only, matching an `NDEBUG` (release) build of the
//! C library, which is what the shared library ships as.
//!
//! Public ABI reproduced (as exported by the C `.so`):
//!   * `pinflate`               (function)
//!   * `cp_error_reason`        (`const char *`)
//!   * `cp_fixed_table`         (`uint8_t [320]`)
//!   * `cp_permutation_order`   (`uint8_t [19]`)
//!   * `cp_len_extra_bits`      (`uint8_t [31]`)
//!   * `cp_len_base`            (`uint32_t[31]`)
//!   * `cp_dist_extra_bits`     (`uint8_t [32]`)
//!   * `cp_dist_base`           (`uint32_t[32]`)

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr::{self, addr_of, addr_of_mut};
use std::alloc::{alloc_zeroed, dealloc, Layout};

// ---------------------------------------------------------------------------
// Exported globals
// ---------------------------------------------------------------------------

/// `const char *cp_error_reason;`
#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

/// Build the fixed Huffman code-length table exactly as spelled out in the C
/// source: 144 * 8, 112 * 9, 24 * 7, 8 * 8 (literal/length tree) followed by
/// 32 * 5 (distance tree).
const fn cp_make_fixed_table() -> [u8; 288 + 32] {
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

/// `uint8_t cp_fixed_table[288 + 32];`
#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = cp_make_fixed_table();

/// `uint8_t cp_permutation_order[19];`
#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// `uint8_t cp_len_extra_bits[29 + 2];`
#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

/// `uint32_t cp_len_base[29 + 2];`
#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

/// `uint8_t cp_dist_extra_bits[30 + 2];`
#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

/// `uint32_t cp_dist_base[30 + 2];`
#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

// ---------------------------------------------------------------------------
// Error strings (byte-for-byte identical to the C string literals)
// ---------------------------------------------------------------------------

const ERR_LEN_NLEN: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const ERR_STORED_BEYOND: &[u8] = b"Stored block extends beyond end of input stream.\0";
const ERR_OUT_SYMBOL: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.\0";
const ERR_BACK_DIST: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const ERR_OUT_STRING: &[u8] = b"Attempted to overwrite out buffer while outputting a string.\0";
const ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";

#[inline]
unsafe fn cp_set_error(msg: &'static [u8]) {
    cp_error_reason = msg.as_ptr() as *const c_char;
}

// ---------------------------------------------------------------------------
// Types from the C source
// ---------------------------------------------------------------------------

/// `struct cp_pixel_t` (declared in the C source; only used by the unused
/// static helpers, kept for completeness).
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
struct cp_pixel_t {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

/// `struct cp_image_t`
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
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
/// `#[repr(C)]` keeps the field layout identical to the C struct, which matters
/// because `cp_decode()` can read `tree[-1]` (i.e. one element *before* the
/// beginning of one of the tree arrays) when a tree is empty.
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

#[allow(dead_code)]
unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    // assert(!(s->bits_left & 7));
    ((*s).words.wrapping_offset((*s).word_index as isize) as *mut c_char)
        .wrapping_offset(-(((*s).count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = ptr::read_unaligned((*s).words.wrapping_offset((*s).word_index as isize));
            (*s).word_index = (*s).word_index.wrapping_add(1);
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add(32);
            // assert(s->word_index <= s->word_count);
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
    // assert(s->count >= num_bits_to_read);
    let mask = 1u64
        .wrapping_shl(num_bits_to_read as u32)
        .wrapping_sub(1);
    let bits = ((*s).bits & mask) as u32;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count = (*s).count.wrapping_sub(num_bits_to_read);
    (*s).bits_left = (*s).bits_left.wrapping_sub(num_bits_to_read);
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    // assert(num_bits_to_read <= 32);
    // assert(num_bits_to_read >= 0);
    // assert(s->bits_left > 0);
    // assert(s->count <= 64);
    // assert(!cp_would_overflow(s, num_bits_to_read));
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
// Huffman tables
// ---------------------------------------------------------------------------

/// `static int cp_build(cp_state_t *s, uint32_t *tree, uint8_t *lens, int sym_count)`
///
/// The `counts` / `codes` / `first` scratch arrays are oversized (256 entries
/// instead of 16) purely so that a corrupt (>15) code length -- which is
/// undefined behaviour in the C original -- cannot trip a Rust bounds panic.
/// For every well defined input the observable behaviour is identical.
unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    let mut counts = [0i32; 256];
    let mut codes = [0i32; 256];
    let mut first = [0i32; 256];

    let mut n: c_int = 0;
    while n < sym_count {
        let l = *lens.wrapping_offset(n as isize) as usize;
        counts[l] = counts[l].wrapping_add(1);
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
        // memset(s->lookup, 0, sizeof(s->lookup));
        ptr::write_bytes(addr_of_mut!((*s).lookup) as *mut u8, 0, 1 << 10);
    }

    let mut i: c_int = 0;
    while i < sym_count {
        let len = *lens.wrapping_offset(i as isize) as usize;
        if len != 0 {
            // assert(len < 16);
            let code = codes[len] as u32;
            codes[len] = codes[len].wrapping_add(1);
            let slot = first[len];
            first[len] = first[len].wrapping_add(1);
            let value = code
                .wrapping_shl(32u32.wrapping_sub(len as u32))
                | ((i as u32) << 4)
                | (len as u32);
            if slot >= 0 && slot < sym_count {
                *tree.wrapping_offset(slot as isize) = value;
            }
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    (*s).lookup[j as usize] = ((len << 9) | (i as usize)) as u16;
                    j += 1 << len;
                }
            }
        }
        i += 1;
    }

    first[15]
}

// ---------------------------------------------------------------------------
// Block decoders
// ---------------------------------------------------------------------------

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let len: u16 = cp_read_bits(s, 16) as u16;
    let nlen: u16 = cp_read_bits(s, 16) as u16;
    if !(len == !nlen) {
        cp_set_error(ERR_LEN_NLEN);
        return 0;
    }
    if !((*s).bits_left / 8 <= len as c_int) {
        cp_set_error(ERR_STORED_BEYOND);
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, (*s).out, len as usize);
    (*s).out = (*s).out.wrapping_offset(len as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    let table = addr_of_mut!(cp_fixed_table) as *const u8;
    (*s).nlit = cp_build(s, addr_of_mut!((*s).lit) as *mut u32, table, 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).dst) as *mut u32,
        table.wrapping_offset(288),
        32,
    ) as u32;
    1
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    let mut hi: c_int = hi;
    while lo < hi {
        let guess = (lo.wrapping_add(hi)) >> 1;
        if search < *tree.wrapping_offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess.wrapping_add(1);
        }
    }
    let key = *tree.wrapping_offset((lo.wrapping_sub(1)) as isize);
    let _len = 32u32.wrapping_sub(key & 0xF);
    // assert((search >> len) == (key >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    // uint8_t lenlens[19] = {0}; (oversized so that a corrupted
    // cp_permutation_order entry cannot panic; only [0, 19) is ever read).
    let mut lenlens = [0u8; 256];
    let nlit: c_int = 257i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let ndst: c_int = 1i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let nlen: c_int = 4i32.wrapping_add(cp_read_bits(s, 4) as c_int);
    let perm = addr_of_mut!(cp_permutation_order) as *const u8;
    let mut i: c_int = 0;
    while i < nlen {
        let idx = *perm.wrapping_offset(i as isize) as usize;
        lenlens[idx] = cp_read_bits(s, 3) as u8;
        i += 1;
    }
    (*s).nlen = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).len) as *mut u32,
        lenlens.as_ptr(),
        19,
    ) as u32;

    // uint8_t lens[288 + 32]; -- the C code can legitimately run past the end
    // of this array (the run-length cases do not clamp), so the buffer here is
    // sized to absorb the largest possible overrun instead of smashing memory.
    let mut lens = [0u8; 512];
    let cap = lens.len() as c_int;
    let mut n: c_int = 0;
    while n < nlit.wrapping_add(ndst) {
        let sym = cp_decode(s, addr_of_mut!((*s).len) as *mut u32, (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i: c_int = 3i32.wrapping_add(cp_read_bits(s, 2) as c_int);
                while i != 0 {
                    let prev = if n >= 1 && n.wrapping_sub(1) < cap {
                        lens[(n - 1) as usize]
                    } else {
                        0
                    };
                    if n >= 0 && n < cap {
                        lens[n as usize] = prev;
                    }
                    i = i.wrapping_sub(1);
                    n = n.wrapping_add(1);
                }
            }
            17 => {
                let mut i: c_int = 3i32.wrapping_add(cp_read_bits(s, 3) as c_int);
                while i != 0 {
                    if n >= 0 && n < cap {
                        lens[n as usize] = 0;
                    }
                    i = i.wrapping_sub(1);
                    n = n.wrapping_add(1);
                }
            }
            18 => {
                let mut i: c_int = 11i32.wrapping_add(cp_read_bits(s, 7) as c_int);
                while i != 0 {
                    if n >= 0 && n < cap {
                        lens[n as usize] = 0;
                    }
                    i = i.wrapping_sub(1);
                    n = n.wrapping_add(1);
                }
            }
            _ => {
                if n >= 0 && n < cap {
                    lens[n as usize] = sym as u8;
                }
                n = n.wrapping_add(1);
            }
        }
    }

    (*s).nlit = cp_build(
        s,
        addr_of_mut!((*s).lit) as *mut u32,
        lens.as_ptr(),
        nlit,
    ) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).dst) as *mut u32,
        lens.as_ptr().wrapping_offset(nlit as isize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    loop {
        let mut symbol = cp_decode(s, addr_of_mut!((*s).lit) as *mut u32, (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.wrapping_offset(1) <= (*s).out_end) {
                cp_set_error(ERR_OUT_SYMBOL);
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.wrapping_offset(1);
        } else if symbol > 256 {
            symbol = symbol.wrapping_sub(257);
            let len_extra =
                *(addr_of!(cp_len_extra_bits) as *const u8).wrapping_offset(symbol as isize)
                    as c_int;
            let len_base = *(addr_of!(cp_len_base) as *const u32).wrapping_offset(symbol as isize);
            let mut length: c_int = cp_read_bits(s, len_extra).wrapping_add(len_base) as c_int;
            let distance_symbol = cp_decode(s, addr_of_mut!((*s).dst) as *mut u32, (*s).ndst as c_int);
            let dist_extra = *(addr_of!(cp_dist_extra_bits) as *const u8)
                .wrapping_offset(distance_symbol as isize) as c_int;
            let dist_base =
                *(addr_of!(cp_dist_base) as *const u32).wrapping_offset(distance_symbol as isize);
            let backwards_distance: c_int =
                cp_read_bits(s, dist_extra).wrapping_add(dist_base) as c_int;
            if !((*s)
                .out
                .wrapping_offset(-(backwards_distance as isize))
                >= (*s).begin)
            {
                cp_set_error(ERR_BACK_DIST);
                return 0;
            }
            if !((*s).out.wrapping_offset(length as isize) <= (*s).out_end) {
                cp_set_error(ERR_OUT_STRING);
                return 0;
            }
            let mut src = (*s).out.wrapping_offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.wrapping_offset(length as isize);
            if backwards_distance == 1 {
                ptr::write_bytes(dst as *mut u8, *src as u8, length as isize as usize);
            } else {
                while length != 0 {
                    length = length.wrapping_sub(1);
                    *dst = *src;
                    dst = dst.wrapping_offset(1);
                    src = src.wrapping_offset(1);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pinflate(
    r#in: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let layout = Layout::new::<cp_state_t>();
    let s = alloc_zeroed(layout) as *mut cp_state_t;

    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let in_addr = r#in as usize;
    let first_bytes: c_int = ((in_addr.wrapping_add(3) & !3usize).wrapping_sub(in_addr)) as c_int;
    (*s).words = (r#in as *mut c_char).wrapping_offset(first_bytes as isize) as *mut u32;
    (*s).word_count = in_bytes.wrapping_sub(first_bytes) / 4;
    let last_bytes: c_int = in_bytes.wrapping_sub(first_bytes) & 3;
    let in_u8 = r#in as *const u8;
    let mut i: c_int = 0;
    while i < first_bytes {
        (*s).bits |= (*in_u8.wrapping_offset(i as isize) as u64)
            .wrapping_shl((i.wrapping_mul(8)) as u32);
        i += 1;
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    let mut i: c_int = 0;
    while i < last_bytes {
        let idx = in_bytes.wrapping_sub(last_bytes).wrapping_add(i);
        (*s).final_word |= (*in_u8.wrapping_offset(idx as isize) as u32)
            .wrapping_shl((i.wrapping_mul(8)) as u32);
        i += 1;
    }
    (*s).count = first_bytes.wrapping_mul(8);
    (*s).out = out as *mut c_char;
    (*s).out_end = ((*s).out).wrapping_offset(out_bytes as isize);
    (*s).begin = out as *mut c_char;

    let mut _count: c_int = 0;
    let mut bfinal: c_int;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            3 => {
                cp_set_error(ERR_UNKNOWN_BLOCK);
                dealloc(s as *mut u8, layout);
                return 0;
            }
            _ => {}
        }
        _count = _count.wrapping_add(1);
        if bfinal != 0 {
            break;
        }
    }
    dealloc(s as *mut u8, layout);
    1
}
