//! Rust translation of `c_src/src/lib.c`.
//!
//! The translation is deliberately literal: the same order of operations, the
//! same integer widths / wrap-around behaviour and the same (buggy) validation
//! order as the original C. Exported symbols keep their C names and ABI.
//!
//! `assert()` from the original is not reproduced (it only guards internal
//! invariants of the DEFLATE reader and produces no program output).

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// cp_pixel_t / cp_image_t (unused by the public API, kept for parity)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
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

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

// ---------------------------------------------------------------------------
// Global (externally visible) data
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = std::ptr::null();

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
    while i < 288 + 32 {
        t[i] = 5;
        i += 1;
    }
    t
};

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
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049,
    3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

/// `cp_error_reason = "...";`
fn set_error_reason(msg: &'static [u8]) {
    unsafe {
        *(&raw mut cp_error_reason) = msg.as_ptr() as *const c_char;
    }
}

const ERR_STORED_LEN_NLEN: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const ERR_STORED_BEYOND: &[u8] = b"Stored block extends beyond end of input stream.\0";
const ERR_OUT_SYMBOL: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.\0";
const ERR_BACKWARDS: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const ERR_OUT_STRING: &[u8] = b"Attempted to overwrite out buffer while outputting a string.\0";
const ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";

// ---------------------------------------------------------------------------
// cp_state_t
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

unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    let s = unsafe { &mut *s };
    ((s.bits_left.wrapping_add(s.count)).wrapping_sub(num_bits) < 0) as c_int
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    let s = unsafe { &mut *s };
    unsafe {
        (s.words.offset(s.word_index as isize) as *mut c_char).offset(-((s.count / 8) as isize))
    }
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    let s = unsafe { &mut *s };
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { s.words.offset(s.word_index as isize).read_unaligned() };
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

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    let s = unsafe { &mut *s };
    let mask = 1u64.wrapping_shl(num_bits_to_read as u32).wrapping_sub(1);
    let bits = (s.bits & mask) as u32;
    s.bits = s.bits.wrapping_shr(num_bits_to_read as u32);
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    unsafe {
        cp_peak_bits(s, num_bits_to_read);
        cp_consume_bits(s, num_bits_to_read)
    }
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
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];

    let mut n = 0;
    while n < sym_count {
        let l = unsafe { *lens.offset(n as isize) } as usize;
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
        unsafe {
            (*s).lookup = [0u16; 1 << 9];
        }
    }

    for i in 0..sym_count {
        let len = unsafe { *lens.offset(i as isize) } as usize;
        if len != 0 {
            let code = codes[len] as u32;
            codes[len] = codes[len].wrapping_add(1);
            let slot = first[len] as u32;
            first[len] = first[len].wrapping_add(1);
            unsafe {
                *tree.offset(slot as i32 as isize) =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            }
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                while j < (1 << 9) {
                    unsafe {
                        (*s).lookup[j as usize] = (((len as u32) << 9) | (i as u32)) as u16;
                    }
                    j += 1 << len;
                }
            }
        }
    }

    first[15]
}

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    unsafe {
        let count_low = (*s).count & 7;
        cp_read_bits(s, count_low);
        let len_field = cp_read_bits(s, 16) as u16;
        let nlen_field = cp_read_bits(s, 16) as u16;

        if !(len_field == !nlen_field) {
            set_error_reason(ERR_STORED_LEN_NLEN);
            return 0;
        }
        if !((*s).bits_left / 8 <= len_field as c_int) {
            set_error_reason(ERR_STORED_BEYOND);
            return 0;
        }

        let p = cp_ptr(s);
        std::ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, len_field as usize);
        (*s).out = (*s).out.offset(len_field as isize);
        1
    }
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    unsafe {
        let table = (&raw mut cp_fixed_table) as *mut u8;
        (*s).nlit = cp_build(s, (&raw mut (*s).lit) as *mut u32, table, 288) as u32;
        (*s).ndst = cp_build(
            std::ptr::null_mut(),
            (&raw mut (*s).dst) as *mut u32,
            table.add(288),
            32,
        ) as u32;
        1
    }
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi: c_int) -> c_int {
    unsafe {
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
        let _code = cp_consume_bits(s, (key & 0xF) as c_int);
        ((key >> 4) & 0xFFF) as c_int
    }
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    unsafe {
        let mut lenlens = [0u8; 19];
        let nlit = 257 + cp_read_bits(s, 5) as c_int;
        let ndst = 1 + cp_read_bits(s, 5) as c_int;
        let nlen = 4 + cp_read_bits(s, 4) as c_int;
        let perm = (&raw const cp_permutation_order) as *const u8;
        for i in 0..nlen {
            let idx = *perm.offset(i as isize) as usize;
            lenlens[idx] = cp_read_bits(s, 3) as u8;
        }
        (*s).nlen = cp_build(
            std::ptr::null_mut(),
            (&raw mut (*s).len) as *mut u32,
            lenlens.as_ptr(),
            19,
        ) as u32;

        // `uint8_t lens[288 + 32];` -- the C code can both read `lens[-1]` and
        // write past the end of the array (a run-length symbol may overshoot
        // `nlit + ndst`).  One byte of head room plus 138 bytes of tail room
        // keeps those accesses inside our allocation.
        let mut lens_buf = [0u8; 1 + 288 + 32 + 138 + 8];
        let lens = lens_buf.as_mut_ptr().add(1);

        let mut n: c_int = 0;
        while n < nlit + ndst {
            let sym = cp_decode(s, (&raw mut (*s).len) as *mut u32, (*s).nlen as c_int);
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

        (*s).nlit = cp_build(s, (&raw mut (*s).lit) as *mut u32, lens, nlit) as u32;
        (*s).ndst = cp_build(
            std::ptr::null_mut(),
            (&raw mut (*s).dst) as *mut u32,
            lens.offset(nlit as isize),
            ndst,
        ) as u32;
        1
    }
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    unsafe {
        loop {
            let mut symbol = cp_decode(s, (&raw mut (*s).lit) as *mut u32, (*s).nlit as c_int);
            if symbol < 256 {
                if !((*s).out.offset(1) <= (*s).out_end) {
                    set_error_reason(ERR_OUT_SYMBOL);
                    return 0;
                }
                *(*s).out = symbol as c_char;
                (*s).out = (*s).out.offset(1);
            } else if symbol > 256 {
                symbol -= 257;
                let extra = *((&raw const cp_len_extra_bits) as *const u8).offset(symbol as isize);
                let base = *((&raw const cp_len_base) as *const u32).offset(symbol as isize);
                let mut length =
                    (cp_read_bits(s, extra as c_int) as c_int).wrapping_add(base as c_int);

                let distance_symbol =
                    cp_decode(s, (&raw mut (*s).dst) as *mut u32, (*s).ndst as c_int);
                let dextra = *((&raw const cp_dist_extra_bits) as *const u8)
                    .offset(distance_symbol as isize);
                let dbase =
                    *((&raw const cp_dist_base) as *const u32).offset(distance_symbol as isize);
                let backwards_distance =
                    (cp_read_bits(s, dextra as c_int) as c_int).wrapping_add(dbase as c_int);

                if !((*s).out.offset(-(backwards_distance as isize)) >= (*s).begin) {
                    set_error_reason(ERR_BACKWARDS);
                    return 0;
                }
                if !((*s).out.offset(length as isize) <= (*s).out_end) {
                    set_error_reason(ERR_OUT_STRING);
                    return 0;
                }

                let mut src = (*s).out.offset(-(backwards_distance as isize)) as *const u8;
                let mut dst = (*s).out as *mut u8;
                (*s).out = (*s).out.offset(length as isize);
                match backwards_distance {
                    1 => {
                        std::ptr::write_bytes(dst, *src, length as usize);
                    }
                    _ => {
                        // `while (length--) *dst++ = *src++;` -- forward byte
                        // copy, overlapping on purpose.
                        while length != 0 {
                            length -= 1;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        let layout = std::alloc::Layout::new::<cp_state_t>();
        let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;

        (*s).bits = 0;
        (*s).count = 0;
        (*s).word_index = 0;
        (*s).bits_left = in_bytes.wrapping_mul(8);
        let in_addr = input as usize;
        let first_bytes = (((in_addr + 3) & !3usize).wrapping_sub(in_addr)) as c_int;
        (*s).words = (input as *mut c_char).offset(first_bytes as isize) as *mut u32;
        (*s).word_count = (in_bytes - first_bytes) / 4;
        let last_bytes = (in_bytes - first_bytes) & 3;
        for i in 0..first_bytes {
            let byte = *(input as *const u8).offset(i as isize);
            (*s).bits |= (byte as u64).wrapping_shl((i * 8) as u32);
        }
        (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
        (*s).final_word = 0;
        for i in 0..last_bytes {
            let byte = *(input as *const u8).offset((in_bytes - last_bytes + i) as isize);
            (*s).final_word |= (byte as u32).wrapping_shl((i * 8) as u32);
        }
        (*s).count = first_bytes.wrapping_mul(8);
        (*s).out = out as *mut c_char;
        (*s).out_end = (*s).out.offset(out_bytes as isize);
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
                    set_error_reason(ERR_UNKNOWN_BLOCK);
                    ok = false;
                    break;
                }
                _ => {}
            }
            count += 1;
            if bfinal != 0 {
                break;
            }
        }
        let _ = count;

        std::alloc::dealloc(s as *mut u8, layout);
        if ok { 1 } else { 0 }
    }
}

// ---------------------------------------------------------------------------
// PNG helpers
// ---------------------------------------------------------------------------

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = (a as c_int) + (b as c_int) - (c as c_int);
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
    unsafe {
        ((*s.offset(0) as u32) << 24)
            | ((*s.offset(1) as u32) << 16)
            | ((*s.offset(2) as u32) << 8)
            | (*s.offset(3) as u32)
    }
}

unsafe fn cp_memcmp4(a: *const u8, b: *const c_char) -> bool {
    unsafe {
        for i in 0..4isize {
            if *a.offset(i) != *(b.offset(i) as *const u8) {
                return false;
            }
        }
        true
    }
}

unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    unsafe {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        if cp_memcmp4(start.offset(4), chunk) && len >= minlen {
            let offset = (len as c_int).wrapping_add(12);
            if (*png).p.offset(offset as isize) <= (*png).end {
                (*png).p = (*png).p.offset(offset as isize);
                return start.offset(8);
            }
        }
        std::ptr::null()
    }
}

unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    unsafe {
        while (*png).p < (*png).end {
            let len = cp_make32((*png).p);
            let start = (*png).p;
            // `png->p += len + 12;` -- `len` is `uint32_t`, so the increment is
            // computed with unsigned wrap-around and then zero-extended, unlike
            // `cp_chunk` which routes the same expression through an `int`.
            (*png).p = (*png).p.add(len.wrapping_add(12) as usize);
            if cp_memcmp4(start.offset(4), chunk) && len >= minlen && (*png).p <= (*png).end {
                return start.offset(8);
            }
        }
        std::ptr::null()
    }
}

// ---------------------------------------------------------------------------
// unfilter (public API)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    unsafe {
        let len = w.wrapping_mul(bpp);
        let mut raw = raw;

        macro_rules! at {
            ($p:expr, $i:expr) => {
                *$p.offset($i as isize)
            };
        }

        if h > 0 {
            let filter = *raw;
            raw = raw.offset(1);
            match filter {
                0 => {}
                1 => {
                    let mut x = bpp;
                    while x < len {
                        at!(raw, x) = at!(raw, x).wrapping_add(at!(raw, x - bpp));
                        x += 1;
                    }
                }
                2 => {}
                3 => {
                    let mut x = bpp;
                    while x < len {
                        at!(raw, x) = at!(raw, x).wrapping_add(at!(raw, x - bpp) / 2);
                        x += 1;
                    }
                }
                4 => {
                    let mut x = bpp;
                    while x < len {
                        at!(raw, x) = at!(raw, x).wrapping_add(cp_paeth(at!(raw, x - bpp), 0, 0));
                        x += 1;
                    }
                }
                _ => return 0,
            }
        }

        let mut prev = raw;
        raw = raw.offset(len as isize);

        let mut y: c_int = 1;
        while y < h {
            let filter = *raw;
            raw = raw.offset(1);
            match filter {
                0 => {}
                1 => {
                    let mut x: c_int = 0;
                    while x < bpp {
                        at!(raw, x) = at!(raw, x).wrapping_add(0);
                        x += 1;
                    }
                    while x < len {
                        at!(raw, x) = at!(raw, x).wrapping_add(at!(raw, x - bpp));
                        x += 1;
                    }
                }
                2 => {
                    let mut x: c_int = 0;
                    while x < bpp {
                        at!(raw, x) = at!(raw, x).wrapping_add(at!(prev, x));
                        x += 1;
                    }
                    while x < len {
                        at!(raw, x) = at!(raw, x).wrapping_add(at!(prev, x));
                        x += 1;
                    }
                }
                3 => {
                    let mut x: c_int = 0;
                    while x < bpp {
                        at!(raw, x) = at!(raw, x).wrapping_add(at!(prev, x) / 2);
                        x += 1;
                    }
                    while x < len {
                        let sum = (at!(raw, x - bpp) as c_int) + (at!(prev, x) as c_int);
                        at!(raw, x) = at!(raw, x).wrapping_add((sum / 2) as u8);
                        x += 1;
                    }
                }
                4 => {
                    let mut x: c_int = 0;
                    while x < bpp {
                        at!(raw, x) = at!(raw, x).wrapping_add(at!(prev, x));
                        x += 1;
                    }
                    while x < len {
                        at!(raw, x) = at!(raw, x).wrapping_add(cp_paeth(
                            at!(raw, x - bpp),
                            at!(prev, x),
                            at!(prev, x - bpp),
                        ));
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
