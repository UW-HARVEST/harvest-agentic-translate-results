//! Rust translation of `c_src/src/lib.c` (a cute_png style DEFLATE/PNG helper).
//!
//! The translation is intentionally literal: the same order of operations,
//! the same integer widths/wrap-around behaviour, and the same (buggy) bounds
//! checks as the original C.  `assert()` calls from the C source are not
//! reproduced (they only fire on inputs that are already undefined behaviour
//! in C).

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// Public C types (include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
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
// Exported globals
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = [
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    8, 8, 8, 8, 8, 8, 8, 8, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
];

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

// The tables above are mutable C globals, so they are always read back through
// the exported symbol rather than from a private copy.
#[inline]
fn fixed_table() -> &'static [u8] {
    unsafe { std::slice::from_raw_parts((&raw const cp_fixed_table).cast::<u8>(), 288 + 32) }
}

#[inline]
fn permutation_order() -> &'static [u8] {
    unsafe { std::slice::from_raw_parts((&raw const cp_permutation_order).cast::<u8>(), 19) }
}

#[inline]
fn len_extra_bits() -> &'static [u8] {
    unsafe { std::slice::from_raw_parts((&raw const cp_len_extra_bits).cast::<u8>(), 29 + 2) }
}

#[inline]
fn len_base() -> &'static [u32] {
    unsafe { std::slice::from_raw_parts((&raw const cp_len_base).cast::<u32>(), 29 + 2) }
}

#[inline]
fn dist_extra_bits() -> &'static [u8] {
    unsafe { std::slice::from_raw_parts((&raw const cp_dist_extra_bits).cast::<u8>(), 30 + 2) }
}

#[inline]
fn dist_base() -> &'static [u32] {
    unsafe { std::slice::from_raw_parts((&raw const cp_dist_base).cast::<u32>(), 30 + 2) }
}

#[inline]
fn set_error(reason: &'static std::ffi::CStr) {
    unsafe {
        cp_error_reason = reason.as_ptr();
    }
}

// ---------------------------------------------------------------------------
// Inflate state
// ---------------------------------------------------------------------------

/// Bit-reader + output cursor half of `cp_state_t`.
struct Reader {
    bits: u64,
    count: c_int,
    words: *const u32,
    word_count: c_int,
    word_index: c_int,
    bits_left: c_int,
    final_word_available: c_int,
    final_word: u32,
    out: *mut u8,
    out_end: *mut u8,
    begin: *mut u8,
}

struct CpState {
    r: Reader,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

impl CpState {
    /// Mirrors `calloc(1, sizeof(cp_state_t))`.
    fn zeroed() -> Box<CpState> {
        Box::new(CpState {
            r: Reader {
                bits: 0,
                count: 0,
                words: ptr::null(),
                word_count: 0,
                word_index: 0,
                bits_left: 0,
                final_word_available: 0,
                final_word: 0,
                out: ptr::null_mut(),
                out_end: ptr::null_mut(),
                begin: ptr::null_mut(),
            },
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

fn cp_would_overflow(s: &Reader, num_bits: c_int) -> c_int {
    ((s.bits_left.wrapping_add(s.count)).wrapping_sub(num_bits) < 0) as c_int
}

/// `(char *)(s->words + s->word_index) - (s->count / 8)`
unsafe fn cp_ptr(s: &Reader) -> *const u8 {
    unsafe {
        s.words
            .add(s.word_index as usize)
            .cast::<u8>()
            // signed offset: C subtracts an `int`, so a negative `count`
            // moves the cursor forward rather than wrapping around.
            .offset(-((s.count / 8) as isize))
    }
}

unsafe fn cp_peak_bits(s: &mut Reader, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { ptr::read_unaligned(s.words.add(s.word_index as usize)) };
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

fn cp_consume_bits(s: &mut Reader, num_bits_to_read: c_int) -> u32 {
    let mask = 1u64.wrapping_shl(num_bits_to_read as u32).wrapping_sub(1);
    let bits = (s.bits & mask) as u32;
    s.bits = s.bits.wrapping_shr(num_bits_to_read as u32);
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: &mut Reader, num_bits_to_read: c_int) -> u32 {
    let _ = cp_would_overflow(s, num_bits_to_read);
    unsafe { cp_peak_bits(s, num_bits_to_read) };
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

/// `cp_build`: `lookup` is `Some(..)` exactly when the C code passed a non-null
/// `cp_state_t *`.
fn cp_build(
    mut lookup: Option<&mut [u16; 1 << 9]>,
    tree: &mut [u32],
    lens: &[u8],
    sym_count: usize,
) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];

    for n in 0..sym_count {
        counts[lens[n] as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }

    if let Some(l) = lookup.as_deref_mut() {
        l.fill(0);
    }

    for i in 0..sym_count {
        let len = lens[i] as usize;
        if len != 0 {
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as u32;
            first[len] += 1;
            tree[slot as usize] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(l) = lookup.as_deref_mut() {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        l[j] = ((len << 9) | i) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }

    first[15]
}

unsafe fn cp_stored(s: &mut Reader) -> c_int {
    unsafe {
        let n = s.count & 7;
        cp_read_bits(s, n);
        let len = cp_read_bits(s, 16) as u16;
        let nlen = cp_read_bits(s, 16) as u16;

        if !(len == !nlen) {
            set_error(c"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.");
            return 0;
        }
        if !(s.bits_left / 8 <= len as c_int) {
            set_error(c"Stored block extends beyond end of input stream.");
            return 0;
        }

        let p = cp_ptr(s);
        ptr::copy_nonoverlapping(p, s.out, len as usize);
        s.out = s.out.add(len as usize);
        1
    }
}

fn cp_fixed(s: &mut CpState) -> c_int {
    let table = fixed_table();
    let nlit = cp_build(Some(&mut s.lookup), &mut s.lit[..], &table[..288], 288);
    s.nlit = nlit as u32;
    let ndst = cp_build(None, &mut s.dst[..], &table[288..], 32);
    s.ndst = ndst as u32;
    1
}

unsafe fn cp_decode(s: &mut Reader, tree: &[u32], hi: c_int) -> c_int {
    let bits = unsafe { cp_peak_bits(s, 16) };
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;

    let mut lo: c_int = 0;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < tree[guess as usize] {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }

    let key = tree[(lo - 1) as usize];
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: &mut CpState) -> c_int {
    unsafe {
        let mut lenlens = [0u8; 19];
        let nlit = 257 + cp_read_bits(&mut s.r, 5) as c_int;
        let ndst = 1 + cp_read_bits(&mut s.r, 5) as c_int;
        let nlen = 4 + cp_read_bits(&mut s.r, 4) as c_int;

        let perm = permutation_order();
        for i in 0..nlen as usize {
            let v = cp_read_bits(&mut s.r, 3) as u8;
            lenlens[perm[i] as usize] = v;
        }
        let nlen_built = cp_build(None, &mut s.len[..], &lenlens, 19);
        s.nlen = nlen_built as u32;

        // `uint8_t lens[288 + 32];` -- the C array can be overrun by malformed
        // repeat codes; the extra slack here absorbs that without changing the
        // behaviour for well formed input.
        let mut lens = [0u8; 288 + 32 + 192];

        let total = (nlit + ndst) as usize;
        let mut n: usize = 0;
        while n < total {
            let sym = cp_decode(&mut s.r, &s.len[..], s.nlen as c_int);
            match sym {
                16 => {
                    let mut i = 3 + cp_read_bits(&mut s.r, 2) as c_int;
                    while i != 0 {
                        // C reads lens[-1] when n == 0 (undefined); use 0.
                        lens[n] = if n == 0 { 0 } else { lens[n - 1] };
                        i -= 1;
                        n += 1;
                    }
                }
                17 => {
                    let mut i = 3 + cp_read_bits(&mut s.r, 3) as c_int;
                    while i != 0 {
                        lens[n] = 0;
                        i -= 1;
                        n += 1;
                    }
                }
                18 => {
                    let mut i = 11 + cp_read_bits(&mut s.r, 7) as c_int;
                    while i != 0 {
                        lens[n] = 0;
                        i -= 1;
                        n += 1;
                    }
                }
                _ => {
                    lens[n] = sym as u8;
                    n += 1;
                }
            }
        }

        let built_lit = cp_build(Some(&mut s.lookup), &mut s.lit[..], &lens[..], nlit as usize);
        s.nlit = built_lit as u32;
        let built_dst = cp_build(
            None,
            &mut s.dst[..],
            &lens[nlit as usize..],
            ndst as usize,
        );
        s.ndst = built_dst as u32;
        1
    }
}

unsafe fn cp_block(s: &mut CpState) -> c_int {
    unsafe {
        loop {
            let mut symbol = cp_decode(&mut s.r, &s.lit[..], s.nlit as c_int);
            if symbol < 256 {
                if !(s.r.out.wrapping_add(1) <= s.r.out_end) {
                    set_error(c"Attempted to overwrite out buffer while outputting a symbol.");
                    return 0;
                }
                *s.r.out = symbol as u8;
                s.r.out = s.r.out.add(1);
            } else if symbol > 256 {
                symbol -= 257;
                let extra = len_extra_bits()[symbol as usize] as c_int;
                let length =
                    cp_read_bits(&mut s.r, extra).wrapping_add(len_base()[symbol as usize]) as c_int;

                let distance_symbol = cp_decode(&mut s.r, &s.dst[..], s.ndst as c_int);
                let dextra = dist_extra_bits()[distance_symbol as usize] as c_int;
                let backwards_distance = cp_read_bits(&mut s.r, dextra)
                    .wrapping_add(dist_base()[distance_symbol as usize])
                    as c_int;

                if !(s.r.out.wrapping_offset(-(backwards_distance as isize)) >= s.r.begin) {
                    set_error(
                        c"Attempted to write before out buffer (invalid backwards distance).",
                    );
                    return 0;
                }
                if !(s.r.out.wrapping_offset(length as isize) <= s.r.out_end) {
                    set_error(c"Attempted to overwrite out buffer while outputting a string.");
                    return 0;
                }

                let src = s.r.out.wrapping_offset(-(backwards_distance as isize));
                let dst = s.r.out;
                s.r.out = s.r.out.wrapping_offset(length as isize);

                if backwards_distance == 1 {
                    ptr::write_bytes(dst, *src, length as usize);
                } else {
                    let mut remaining = length;
                    let mut sp = src as *const u8;
                    let mut dp = dst;
                    while remaining != 0 {
                        *dp = *sp;
                        dp = dp.add(1);
                        sp = sp.add(1);
                        remaining -= 1;
                    }
                }
            } else {
                break;
            }
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        let mut state = CpState::zeroed();
        let s: &mut CpState = &mut state;

        s.r.bits = 0;
        s.r.count = 0;
        s.r.word_index = 0;
        s.r.bits_left = in_bytes.wrapping_mul(8);

        let in_addr = input as usize;
        let first_bytes = (((in_addr + 3) & !3usize) - in_addr) as c_int;
        s.r.words = (input as *const u8).wrapping_add(first_bytes as usize) as *const u32;
        s.r.word_count = (in_bytes.wrapping_sub(first_bytes)) / 4;
        let last_bytes = in_bytes.wrapping_sub(first_bytes) & 3;

        let in_u8 = input as *const u8;
        for i in 0..first_bytes {
            s.r.bits |= (*in_u8.offset(i as isize) as u64) << (i * 8);
        }
        s.r.final_word_available = if last_bytes != 0 { 1 } else { 0 };
        s.r.final_word = 0;
        for i in 0..last_bytes {
            // C indexes with an `int`; keep the arithmetic signed so a
            // negative index reads *before* `in`, exactly as C does.
            let idx = in_bytes.wrapping_sub(last_bytes).wrapping_add(i) as isize;
            s.r.final_word |= (*in_u8.offset(idx) as u32) << (i * 8);
        }
        s.r.count = first_bytes * 8;
        s.r.out = out as *mut u8;
        s.r.out_end = (out as *mut u8).wrapping_add(out_bytes as usize);
        s.r.begin = out as *mut u8;

        let mut count: c_int = 0;
        let mut bfinal: c_int;
        loop {
            bfinal = cp_read_bits(&mut s.r, 1) as c_int;
            let btype = cp_read_bits(&mut s.r, 2);
            match btype {
                0 => {
                    if cp_stored(&mut s.r) == 0 {
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
                _ => {
                    set_error(c"Detected unknown block type within input stream.");
                    return 0;
                }
            }
            count += 1;
            if bfinal != 0 {
                break;
            }
        }
        let _ = count;
        1
    }
}

// ---------------------------------------------------------------------------
// PNG helpers (all `static` in the C source; unreachable but kept for parity)
// ---------------------------------------------------------------------------

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as c_int + b as c_int - c as c_int;
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
    unsafe {
        ((*s.add(0) as u32) << 24)
            | ((*s.add(1) as u32) << 16)
            | ((*s.add(2) as u32) << 8)
            | (*s.add(3) as u32)
    }
}

unsafe fn cp_memcmp4(a: *const u8, b: *const c_char) -> bool {
    unsafe {
        for i in 0..4usize {
            if *a.add(i) != *b.add(i) as u8 {
                return false;
            }
        }
        true
    }
}

unsafe fn cp_chunk(png: &mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    unsafe {
        let len = cp_make32(png.p);
        let start = png.p;
        if cp_memcmp4(start.add(4), chunk) && len >= minlen {
            // C stores the offset in an `int`, so the value is truncated to 32
            // bits and then *sign extended* for the pointer arithmetic.
            let offset = len.wrapping_add(12) as i32 as isize;
            if png.p.wrapping_offset(offset) <= png.end {
                png.p = png.p.wrapping_offset(offset);
                return start.add(8);
            }
        }
        ptr::null()
    }
}

unsafe fn cp_find(png: &mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    unsafe {
        while png.p < png.end {
            let len = cp_make32(png.p);
            let start = png.p;
            png.p = png.p.wrapping_add(len.wrapping_add(12) as usize);
            if cp_memcmp4(start.add(4), chunk) && len >= minlen && png.p <= png.end {
                return start.add(8);
            }
        }
        ptr::null()
    }
}

unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    unsafe {
        let len = w.wrapping_mul(bpp);
        let mut raw = raw;
        let mut prev: *mut u8;
        let mut x: c_int;

        if h > 0 {
            let filter = *raw;
            raw = raw.add(1);
            match filter {
                0 => {}
                1 => {
                    x = bpp;
                    while x < len {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize));
                        x += 1;
                    }
                }
                2 => {}
                3 => {
                    x = bpp;
                    while x < len {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize) / 2);
                        x += 1;
                    }
                }
                4 => {
                    x = bpp;
                    while x < len {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(cp_paeth(*raw.offset((x - bpp) as isize), 0, 0));
                        x += 1;
                    }
                }
                _ => return 0,
            }
        }

        prev = raw;
        raw = raw.offset(len as isize);

        let mut y = 1;
        while y < h {
            let filter = *raw;
            raw = raw.add(1);
            match filter {
                0 => {}
                1 => {
                    x = 0;
                    while x < bpp {
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(0);
                        x += 1;
                    }
                    while x < len {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize));
                        x += 1;
                    }
                }
                2 => {
                    x = 0;
                    while x < bpp {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                        x += 1;
                    }
                    while x < len {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                        x += 1;
                    }
                }
                3 => {
                    x = 0;
                    while x < bpp {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize) / 2);
                        x += 1;
                    }
                    while x < len {
                        let v = (*raw.offset((x - bpp) as isize) as c_int
                            + *prev.offset(x as isize) as c_int)
                            / 2;
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(v as u8);
                        x += 1;
                    }
                }
                4 => {
                    x = 0;
                    while x < bpp {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                        x += 1;
                    }
                    while x < len {
                        let v = cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            *prev.offset(x as isize),
                            *prev.offset((x - bpp) as isize),
                        );
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
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

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn convert_pix(
    bpp: c_int,
    w: c_int,
    h: c_int,
    src: *mut u8,
    dst: *mut cp_pixel_t,
) {
    unsafe {
        let mut src = src;
        let mut dst = dst;
        let mut y = 0;
        while y < h {
            src = src.add(1);
            let mut x = 0;
            while x < w {
                match bpp {
                    1 => {
                        *dst = cp_make_pixel(*src.add(0), *src.add(0), *src.add(0));
                        dst = dst.add(1);
                    }
                    2 => {
                        *dst = cp_make_pixel_a(*src.add(0), *src.add(0), *src.add(0), *src.add(1));
                        dst = dst.add(1);
                    }
                    3 => {
                        *dst = cp_make_pixel(*src.add(0), *src.add(1), *src.add(2));
                        dst = dst.add(1);
                    }
                    4 => {
                        *dst =
                            cp_make_pixel_a(*src.add(0), *src.add(1), *src.add(2), *src.add(3));
                        dst = dst.add(1);
                    }
                    _ => {}
                }
                x += 1;
                src = src.wrapping_offset(bpp as isize);
            }
            y += 1;
        }
    }
}
