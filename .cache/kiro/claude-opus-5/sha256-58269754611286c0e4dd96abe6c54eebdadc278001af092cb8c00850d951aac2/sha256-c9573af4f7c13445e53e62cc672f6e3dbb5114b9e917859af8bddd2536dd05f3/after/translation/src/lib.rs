//! Rust translation of c_src/src/lib.c (a cut-down cute_png style DEFLATE/PNG helper).
//!
//! The translation is intentionally literal: every observable behaviour of the C
//! implementation (including its quirks and bugs) is reproduced.
//!
//! # `assert()` semantics
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and never defines `NDEBUG`,
//! so the reference `.so` is compiled with `assert()` **live** (its dynamic
//! symbol table imports `__assert_fail`). A failing assertion therefore
//! `abort()`s the process, and that is an observable part of the C's behaviour
//! on malformed input.
//!
//! The `c_asserts` feature (enabled by default) reproduces the assertions
//! literally; combined with `panic = "abort"` this aborts on exactly the same
//! inputs as the C. Building with `--no-default-features` drops them, matching a
//! C build with `-DNDEBUG`.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_macros)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

/// Mirrors C's `assert()`: active unless the C was compiled with `NDEBUG`.
#[cfg(feature = "c_asserts")]
macro_rules! c_assert {
    ($cond:expr, $name:literal) => {
        if !$cond {
            panic!(concat!("Assertion `", $name, "' failed."));
        }
    };
}

#[cfg(not(feature = "c_asserts"))]
macro_rules! c_assert {
    ($cond:expr, $name:literal) => {};
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// `struct cp_pixel_t` from include/lib.h
#[repr(C)]
#[derive(Copy, Clone)]
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
// Public (exported, writable) data objects
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[inline]
unsafe fn set_error(msg: &'static [u8]) {
    // msg must be NUL terminated.
    *(&raw mut cp_error_reason) = msg.as_ptr() as *const c_char;
}

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = [
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
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

// ---------------------------------------------------------------------------
// Internal image type (unused by the public ABI, kept for fidelity)
// ---------------------------------------------------------------------------

#[repr(C)]
struct cp_image_t {
    w: c_int,
    h: c_int,
    pix: *mut cp_pixel_t,
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

impl cp_state_t {
    fn zeroed() -> cp_state_t {
        cp_state_t {
            bits: 0,
            count: 0,
            words: ptr::null_mut(),
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: 0,
            final_word: 0,
            out: ptr::null_mut(),
            out_end: ptr::null_mut(),
            begin: ptr::null_mut(),
            lookup: [0; 1 << 9],
            lit: [0; 288],
            dst: [0; 32],
            len: [0; 19],
            nlit: 0,
            ndst: 0,
            nlen: 0,
        }
    }
}

unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    let s = &*s;
    (((s.bits_left.wrapping_add(s.count)).wrapping_sub(num_bits)) < 0) as c_int
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    let s = &*s;
    c_assert!((s.bits_left & 7) == 0, "!(s->bits_left & 7)");
    (s.words.offset(s.word_index as isize) as *mut c_char).offset(-((s.count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    let st = &mut *s;
    if st.count < num_bits_to_read {
        if st.word_index < st.word_count {
            let word = ptr::read(st.words.offset(st.word_index as isize));
            st.word_index += 1;
            st.bits |= (word as u64) << st.count;
            st.count += 32;
            c_assert!(st.word_index <= st.word_count, "s->word_index <= s->word_count");
        } else if st.final_word_available != 0 {
            let word = st.final_word;
            st.bits |= (word as u64) << st.count;
            st.count += st.bits_left;
            st.final_word_available = 0;
        }
    }
    st.bits
}

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    let st = &mut *s;
    c_assert!(st.count >= num_bits_to_read, "s->count >= num_bits_to_read");
    let bits = (st.bits & (((1u64) << num_bits_to_read).wrapping_sub(1))) as u32;
    st.bits >>= num_bits_to_read;
    st.count -= num_bits_to_read;
    st.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    c_assert!(num_bits_to_read <= 32, "num_bits_to_read <= 32");
    c_assert!(num_bits_to_read >= 0, "num_bits_to_read >= 0");
    c_assert!((*s).bits_left > 0, "s->bits_left > 0");
    c_assert!((*s).count <= 64, "s->count <= 64");
    c_assert!(
        cp_would_overflow(s, num_bits_to_read) == 0,
        "!cp_would_overflow(s, num_bits_to_read)"
    );
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
    // C: `int n, codes[16], first[16], counts[16] = {0};`
    //
    // The C then does `counts[lens[n]]++` with no range check, and `lens[n]` can
    // be up to 255 when a malformed dynamic block (or a caller-mutated
    // `cp_fixed_table`) yields a code length >= 16 — an out-of-bounds stack
    // access, i.e. undefined behaviour (`ERRORS.md` row U6). These arrays are
    // widened to 256 entries so the Rust absorbs the same index instead of
    // faulting on a bounds check: with `c_asserts` on, `assert(len < 16)` then
    // aborts at exactly the point the C's assert does; with `-DNDEBUG` semantics
    // the C's own result is undefined.
    let mut codes: [c_int; 256] = [0; 256];
    let mut first: [c_int; 256] = [0; 256];
    let mut counts: [c_int; 256] = [0; 256];

    let mut n: c_int = 0;
    while n < sym_count {
        let l = ptr::read(lens.offset(n as isize)) as usize;
        counts[l] += 1;
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
        ptr::write_bytes((*s).lookup.as_mut_ptr() as *mut u8, 0, 2 * (1 << 9));
    }

    let mut i: c_int = 0;
    while i < sym_count {
        let len = ptr::read(lens.offset(i as isize)) as c_int;
        if len != 0 {
            c_assert!(len < 16, "len < 16");
            let code = codes[len as usize] as u32;
            codes[len as usize] = codes[len as usize].wrapping_add(1);
            let slot = first[len as usize] as u32;
            first[len as usize] = first[len as usize].wrapping_add(1);
            ptr::write(
                tree.offset(slot as isize),
                // `code << (32 - len)`: with a malformed `len > 32` the C's shift
                // count is out of range; `wrapping_shl` reproduces the masking the
                // hardware (and hence the compiled C) performs.
                code.wrapping_shl((32 - len) as u32) | ((i as u32) << 4) | (len as u32),
            );
            if !s.is_null() && len <= 9 {
                let mut j: c_int = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    (*s).lookup[j as usize] = (((len as u32) << 9) | (i as u32)) as u16;
                    j += 1 << len;
                }
            }
        }
        i += 1;
    }

    first[15]
}

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    let p: *mut c_char;
    cp_read_bits(s, (*s).count & 7);
    let len_val = cp_read_bits(s, 16) as u16;
    let nlen_val = cp_read_bits(s, 16) as u16;
    if !(len_val == !nlen_val) {
        set_error(
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0",
        );
        return 0;
    }
    if !((*s).bits_left / 8 <= len_val as c_int) {
        set_error(b"Stored block extends beyond end of input stream.\0");
        return 0;
    }
    p = cp_ptr(s);
    ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, len_val as usize);
    (*s).out = (*s).out.offset(len_val as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    let table = (&raw mut cp_fixed_table) as *mut u8;
    let lit = (*s).lit.as_mut_ptr();
    (*s).nlit = cp_build(s, lit, table as *const u8, 288) as u32;
    let dst = (*s).dst.as_mut_ptr();
    (*s).ndst = cp_build(
        ptr::null_mut(),
        dst,
        table.offset(288) as *const u8,
        32,
    ) as u32;
    1
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < ptr::read(tree.offset(guess as isize)) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = ptr::read(tree.offset((lo - 1) as isize));
    // `uint32_t len = (32 - (key & 0xF));` can be 32, in which case the C's
    // `>> len` is a 32-bit shift by 32; `wrapping_shr` reproduces what the
    // hardware (and therefore the compiled C) actually does.
    let _len: u32 = 32u32.wrapping_sub(key & 0xF);
    c_assert!(
        search.wrapping_shr(_len) == key.wrapping_shr(_len),
        "(search >> len) == (key >> len)"
    );
    let code = cp_consume_bits(s, (key & 0xF) as c_int);
    let _ = code;
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit: c_int = 257 + cp_read_bits(s, 5) as c_int;
    let ndst: c_int = 1 + cp_read_bits(s, 5) as c_int;
    let nlen: c_int = 4 + cp_read_bits(s, 4) as c_int;
    let perm = (&raw mut cp_permutation_order) as *mut u8;
    for i in 0..nlen {
        let idx = ptr::read(perm.offset(i as isize)) as usize;
        lenlens[idx] = cp_read_bits(s, 3) as u8;
    }
    let lenp = (*s).len.as_mut_ptr();
    (*s).nlen = cp_build(ptr::null_mut(), lenp, lenlens.as_ptr(), 19) as u32;

    // uint8_t lens[288 + 32];
    //
    // The C code can run past the end of this array with malformed input (a
    // repeat code may overshoot nlit + ndst) and can also read lens[-1] when
    // symbol 16 appears first. The backing store below is padded on both sides
    // so that the arithmetic stays in bounds; for well formed streams the
    // behaviour is identical to the C version.
    let mut lens_backing: [u8; 1 + (288 + 32) + 160] = [0; 1 + (288 + 32) + 160];
    let lens: *mut u8 = lens_backing.as_mut_ptr().offset(1);

    let mut n: c_int = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, (*s).len.as_mut_ptr(), (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as c_int;
                while i != 0 {
                    ptr::write(
                        lens.offset(n as isize),
                        ptr::read(lens.offset((n - 1) as isize)),
                    );
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as c_int;
                while i != 0 {
                    ptr::write(lens.offset(n as isize), 0);
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as c_int;
                while i != 0 {
                    ptr::write(lens.offset(n as isize), 0);
                    i -= 1;
                    n += 1;
                }
            }
            _ => {
                ptr::write(lens.offset(n as isize), sym as u8);
                n += 1;
            }
        }
    }

    let lit = (*s).lit.as_mut_ptr();
    (*s).nlit = cp_build(s, lit, lens as *const u8, nlit) as u32;
    let dstp = (*s).dst.as_mut_ptr();
    (*s).ndst = cp_build(
        ptr::null_mut(),
        dstp,
        lens.offset(nlit as isize) as *const u8,
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    loop {
        let mut symbol = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.offset(1) <= (*s).out_end) {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return 0;
            }
            ptr::write((*s).out, symbol as c_char);
            (*s).out = (*s).out.offset(1);
        } else if symbol > 256 {
            symbol -= 257;
            let len_extra = ptr::read(((&raw mut cp_len_extra_bits) as *const u8).offset(symbol as isize));
            let len_base = ptr::read(((&raw mut cp_len_base) as *const u32).offset(symbol as isize));
            let length: c_int =
                (cp_read_bits(s, len_extra as c_int) as c_int).wrapping_add(len_base as c_int);
            let distance_symbol = cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as c_int);
            let dist_extra = ptr::read(
                ((&raw mut cp_dist_extra_bits) as *const u8).offset(distance_symbol as isize),
            );
            let dist_base = ptr::read(
                ((&raw mut cp_dist_base) as *const u32).offset(distance_symbol as isize),
            );
            let backwards_distance: c_int =
                (cp_read_bits(s, dist_extra as c_int) as c_int).wrapping_add(dist_base as c_int);
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
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst as *mut u8, ptr::read(src as *const u8), length as usize);
                }
                _ => {
                    let mut length = length;
                    while length != 0 {
                        length -= 1;
                        ptr::write(dst, ptr::read(src));
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
    input: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let boxed: Box<cp_state_t> = Box::new(cp_state_t::zeroed());
    let s: *mut cp_state_t = Box::into_raw(boxed);

    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let in_addr = input as usize;
    let first_bytes: c_int = (((in_addr.wrapping_add(3)) & !3usize).wrapping_sub(in_addr)) as c_int;
    (*s).words = ((input as *mut c_char).offset(first_bytes as isize)) as *mut u32;
    (*s).word_count = (in_bytes.wrapping_sub(first_bytes)) / 4;
    let last_bytes: c_int = (in_bytes.wrapping_sub(first_bytes)) & 3;
    for i in 0..first_bytes {
        let b = ptr::read((input as *const u8).offset(i as isize));
        (*s).bits |= (b as u64) << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        let b = ptr::read(
            (input as *const u8).offset((in_bytes.wrapping_sub(last_bytes) + i) as isize),
        );
        (*s).final_word |= ((b as c_int) << (i * 8)) as u32;
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
                set_error(b"Detected unknown block type within input stream.\0");
                ok = false;
                break;
            }
            _ => {}
        }
        count = count.wrapping_add(1);
        if bfinal != 0 {
            break;
        }
    }
    let _ = count;

    drop(Box::from_raw(s));
    if ok {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// PNG helpers (all `static` in C, so not part of the exported ABI)
// ---------------------------------------------------------------------------

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

#[repr(C)]
struct cp_raw_png_t {
    p: *const u8,
    end: *const u8,
}

unsafe fn cp_make32(s: *const u8) -> u32 {
    ((ptr::read(s.offset(0)) as u32) << 24)
        | ((ptr::read(s.offset(1)) as u32) << 16)
        | ((ptr::read(s.offset(2)) as u32) << 8)
        | (ptr::read(s.offset(3)) as u32)
}

unsafe fn cp_memcmp4(a: *const u8, b: *const c_char) -> c_int {
    for i in 0..4isize {
        let x = ptr::read(a.offset(i));
        let y = ptr::read(b.offset(i) as *const u8);
        if x != y {
            return if x < y { -1 } else { 1 };
        }
    }
    0
}

unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if cp_memcmp4(start.offset(4), chunk) == 0 && len >= minlen {
        let offset = len.wrapping_add(12) as c_int;
        if (*png).p.offset(offset as isize) <= (*png).end {
            (*png).p = (*png).p.offset(offset as isize);
            return start.offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.offset(len.wrapping_add(12) as isize);
        if cp_memcmp4(start.offset(4), chunk) == 0 && len >= minlen && (*png).p <= (*png).end {
            return start.offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len: c_int = w.wrapping_mul(bpp);
    let mut raw = raw;
    let prev0: *mut u8;
    let mut x: c_int;
    if h > 0 {
        let f = ptr::read(raw);
        raw = raw.offset(1);
        match f {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    let v = ptr::read(raw.offset((x - bpp) as isize));
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    let v = ptr::read(raw.offset((x - bpp) as isize)) / 2;
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let v = cp_paeth(ptr::read(raw.offset((x - bpp) as isize)), 0, 0);
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
                    x += 1;
                }
            }
            _ => return 0,
        }
    }
    prev0 = raw;
    let mut prev = prev0;
    raw = raw.offset(len as isize);
    let mut y: c_int = 1;
    while y < h {
        let f = ptr::read(raw);
        raw = raw.offset(1);
        match f {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(0));
                    x += 1;
                }
                while x < len {
                    let v = ptr::read(raw.offset((x - bpp) as isize));
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    let v = ptr::read(prev.offset(x as isize));
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
                    x += 1;
                }
                while x < len {
                    let v = ptr::read(prev.offset(x as isize));
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    let v = ptr::read(prev.offset(x as isize)) / 2;
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
                    x += 1;
                }
                while x < len {
                    let a = ptr::read(raw.offset((x - bpp) as isize)) as c_int;
                    let b = ptr::read(prev.offset(x as isize)) as c_int;
                    let v = ((a + b) / 2) as u8;
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    let v = ptr::read(prev.offset(x as isize));
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
                    x += 1;
                }
                while x < len {
                    let v = cp_paeth(
                        ptr::read(raw.offset((x - bpp) as isize)),
                        ptr::read(prev.offset(x as isize)),
                        ptr::read(prev.offset((x - bpp) as isize)),
                    );
                    let cur = ptr::read(raw.offset(x as isize));
                    ptr::write(raw.offset(x as isize), cur.wrapping_add(v));
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

// ---------------------------------------------------------------------------
// convert_pix (public)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn convert_pix(
    bpp: c_int,
    w: c_int,
    h: c_int,
    src: *mut u8,
    dst: *mut cp_pixel_t,
) {
    let mut src = src;
    let mut dst = dst;
    let mut y: c_int = 0;
    while y < h {
        src = src.offset(1);
        let mut x: c_int = 0;
        while x < w {
            match bpp {
                1 => {
                    let v = ptr::read(src.offset(0));
                    ptr::write(dst, cp_make_pixel(v, v, v));
                    dst = dst.offset(1);
                }
                2 => {
                    let v = ptr::read(src.offset(0));
                    let a = ptr::read(src.offset(1));
                    ptr::write(dst, cp_make_pixel_a(v, v, v, a));
                    dst = dst.offset(1);
                }
                3 => {
                    let r = ptr::read(src.offset(0));
                    let g = ptr::read(src.offset(1));
                    let b = ptr::read(src.offset(2));
                    ptr::write(dst, cp_make_pixel(r, g, b));
                    dst = dst.offset(1);
                }
                4 => {
                    let r = ptr::read(src.offset(0));
                    let g = ptr::read(src.offset(1));
                    let b = ptr::read(src.offset(2));
                    let a = ptr::read(src.offset(3));
                    ptr::write(dst, cp_make_pixel_a(r, g, b, a));
                    dst = dst.offset(1);
                }
                _ => {}
            }
            x += 1;
            src = src.offset(bpp as isize);
        }
        y += 1;
    }
}
