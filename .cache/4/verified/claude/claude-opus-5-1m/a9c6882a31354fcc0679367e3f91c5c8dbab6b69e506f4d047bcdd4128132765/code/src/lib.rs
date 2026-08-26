//! Rust translation of c_src/src/lib.c (cute_png style deflate/PNG helpers).
//!
//! The C translation unit is compiled into a shared library exporting:
//!   convert_pix, cp_inflate,
//!   cp_error_reason, cp_fixed_table, cp_permutation_order,
//!   cp_len_extra_bits, cp_len_base, cp_dist_extra_bits, cp_dist_base
//!
//! Behaviour (including the bugs of the original) is reproduced exactly.
//! The C sources are compiled with assertions ENABLED (no NDEBUG), so a failed
//! assertion aborts the process; `cp_assert!` mirrors that.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use core::ptr::addr_of_mut;

// ---------------------------------------------------------------------------
// assert() emulation: glibc's assert prints to stderr then abort()s.  We keep
// the process-level behaviour (SIGABRT) identical.
// ---------------------------------------------------------------------------

#[cold]
#[inline(never)]
fn cp_assert_fail() -> ! {
    std::process::abort()
}

macro_rules! cp_assert {
    ($cond:expr) => {
        if !($cond) {
            crate::cp_assert_fail();
        }
    };
}

// ---------------------------------------------------------------------------
// Public types (include/lib.h)
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
// Exported data symbols
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

/// `uint8_t cp_fixed_table[288 + 32]`
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

// Error strings (identical bytes to the C string literals).
const CP_ERR_LEN_NLEN: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const CP_ERR_STORED_BEYOND: &[u8] = b"Stored block extends beyond end of input stream.\0";
const CP_ERR_OUT_SYMBOL: &[u8] =
    b"Attempted to overwrite out buffer while outputting a symbol.\0";
const CP_ERR_BEFORE_OUT: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const CP_ERR_OUT_STRING: &[u8] =
    b"Attempted to overwrite out buffer while outputting a string.\0";
const CP_ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";

#[inline]
unsafe fn cp_set_error(msg: &[u8]) {
    cp_error_reason = msg.as_ptr() as *const c_char;
}

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

#[inline]
unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    cp_assert!(((*s).bits_left & 7) == 0);
    ((*s).words.offset((*s).word_index as isize) as *mut c_char)
        .wrapping_offset(-(((*s).count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.offset((*s).word_index as isize);
            (*s).word_index = (*s).word_index.wrapping_add(1);
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add(32);
            cp_assert!((*s).word_index <= (*s).word_count);
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
    cp_assert!((*s).count >= num_bits_to_read);
    let mask = (1u64).wrapping_shl(num_bits_to_read as u32).wrapping_sub(1);
    let bits = ((*s).bits & mask) as u32;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count = (*s).count.wrapping_sub(num_bits_to_read);
    (*s).bits_left = (*s).bits_left.wrapping_sub(num_bits_to_read);
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    cp_assert!(num_bits_to_read <= 32);
    cp_assert!(num_bits_to_read >= 0);
    cp_assert!((*s).bits_left > 0);
    cp_assert!((*s).count <= 64);
    cp_assert!(cp_would_overflow(s, num_bits_to_read) == 0);
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
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    // The C code declares `int counts[16] = {0}` and indexes it with the raw
    // (up to 255) length values; widened here so out-of-range lengths (which
    // are undefined behaviour in C, and abort on the `len < 16` assert right
    // afterwards) do not trap before that point.
    let mut counts = [0i32; 256];

    let mut n: c_int = 0;
    while n < sym_count {
        let ci = *lens.offset(n as isize) as usize;
        counts[ci] = counts[ci].wrapping_add(1);
        n = n.wrapping_add(1);
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    let mut n: usize = 1;
    while n <= 15 {
        codes[n] = codes[n - 1].wrapping_add(counts[n - 1]).wrapping_shl(1);
        first[n] = first[n - 1].wrapping_add(counts[n - 1]);
        n += 1;
    }
    if !s.is_null() {
        ptr::write_bytes(addr_of_mut!((*s).lookup) as *mut u8, 0, 2 * (1 << 9));
    }
    let mut i: c_int = 0;
    while i < sym_count {
        let len = *lens.offset(i as isize) as usize;
        if len != 0 {
            cp_assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] = codes[len].wrapping_add(1);
            let slot = first[len] as u32;
            first[len] = first[len].wrapping_add(1);
            *tree.offset(slot as isize) =
                (code.wrapping_shl(32 - len as u32)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j: c_int = (cp_rev16(code) >> (16 - len as u32)) as c_int;
                let lookup = addr_of_mut!((*s).lookup) as *mut u16;
                while j < (1 << 9) {
                    *lookup.offset(j as isize) = ((len << 9) | (i as usize)) as u16;
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
        cp_set_error(CP_ERR_LEN_NLEN);
        return 0;
    }
    if !((*s).bits_left / 8 <= LEN as c_int) {
        cp_set_error(CP_ERR_STORED_BEYOND);
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, LEN as usize);
    (*s).out = (*s).out.wrapping_offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    let table = addr_of_mut!(cp_fixed_table) as *const u8;
    (*s).nlit = cp_build(s, addr_of_mut!((*s).lit) as *mut u32, table, 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).dst) as *mut u32,
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
        let guess = lo.wrapping_add(hi) >> 1;
        if search < *tree.offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess.wrapping_add(1);
        }
    }
    let key = *tree.offset(lo.wrapping_sub(1) as isize);
    let len = 32 - (key & 0xF);
    // C shifts a uint32_t by `len`, which is 32 when (key & 0xF) == 0; on x86
    // that degenerates to a shift by 0 (count masked to 5 bits).
    cp_assert!(search.wrapping_shr(len) == key.wrapping_shr(len));
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

// ---------------------------------------------------------------------------
// cp_dynamic: exact stack-frame emulation
//
// `cp_dynamic` declares `uint8_t lens[288 + 32]` and lets the repeat codes
// 16/17/18 run PAST its end (a run can add up to 138 entries while the loop
// bound is at most 320), so the C code writes over the rest of its own stack
// frame.  It also reads `lens[-1]` when code 16 arrives with n == 0.  Both are
// undefined behaviour in C, but the compiled C library has one definite
// behaviour and this translation has to reproduce it, so the frame is modelled
// byte-for-byte.  Layout taken from `objdump -d` of the C `.so` built exactly as
// the project's CMakeLists.txt builds it (gcc, -O0, no NDEBUG); offsets are
// relative to %rbp and the array below is indexed by `0x190 - <rbp offset>`:
//
//   rbp-0x188  void *s              (so lens[-8..-1] alias the pointer bytes,
//                                    and lens[-1] is its most significant byte)
//   rbp-0x180  uint8_t lens[320]
//   rbp-0x040  uint8_t lenlens[19]  == lens[320..338]
//   rbp-0x02d  9 bytes of padding   == lens[339..347]
//   rbp-0x024  int sym              == lens[348..351]
//   rbp-0x020  int nlen             == lens[352..355]
//   rbp-0x01c  int ndst             == lens[356..359]
//   rbp-0x018  int nlit             == lens[360..363]
//   rbp-0x014  int i  (code 18)     == lens[364..367]
//   rbp-0x010  int i  (code 17)     == lens[368..371]
//   rbp-0x00c  int i  (code 16)     == lens[372..375]
//   rbp-0x008  int n                == lens[376..379]
//   rbp-0x004  int i  (header loop) == lens[380..383]
//
// The consequence the tests actually observe: a stream whose last code-length
// symbol is an 18 with a long run drives n up to 376, where writing lens[376]
// clears the low byte of `n` itself, so n snaps back to 256 and the inner run
// loop never terminates - `cp_inflate` hangs.  With this frame model the Rust
// build hangs identically instead of finishing with a different answer.
// ---------------------------------------------------------------------------

const CPD_FRAME: usize = 0x190;
/// room for the writes that in C would land in the *caller's* frame
const CPD_SLACK: usize = 1024;

const CPD_S: usize = CPD_FRAME - 0x188;
const CPD_LENS: usize = CPD_FRAME - 0x180;
const CPD_LENLENS: usize = CPD_FRAME - 0x040;
const CPD_SYM: usize = CPD_FRAME - 0x024;
const CPD_NLEN: usize = CPD_FRAME - 0x020;
const CPD_NDST: usize = CPD_FRAME - 0x01c;
const CPD_NLIT: usize = CPD_FRAME - 0x018;
const CPD_I18: usize = CPD_FRAME - 0x014;
const CPD_I17: usize = CPD_FRAME - 0x010;
const CPD_I16: usize = CPD_FRAME - 0x00c;
const CPD_N: usize = CPD_FRAME - 0x008;
const CPD_I: usize = CPD_FRAME - 0x004;

#[inline(always)]
unsafe fn fr_get(fp: *mut u8, off: usize) -> c_int {
    (fp.add(off) as *const c_int).read_unaligned()
}

#[inline(always)]
unsafe fn fr_set(fp: *mut u8, off: usize, v: c_int) {
    (fp.add(off) as *mut c_int).write_unaligned(v)
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    let mut frame = [0u8; CPD_FRAME + CPD_SLACK];
    let fp = frame.as_mut_ptr();
    // the incoming parameter is spilled to rbp-0x188
    (fp.add(CPD_S) as *mut usize).write_unaligned(s as usize);

    let lens = fp.add(CPD_LENS);
    let lenlens = fp.add(CPD_LENLENS);
    // C: `uint8_t lenlens[19] = {0};`
    ptr::write_bytes(lenlens, 0, 19);

    fr_set(fp, CPD_NLIT, 257i32.wrapping_add(cp_read_bits(s, 5) as c_int));
    fr_set(fp, CPD_NDST, 1i32.wrapping_add(cp_read_bits(s, 5) as c_int));
    fr_set(fp, CPD_NLEN, 4i32.wrapping_add(cp_read_bits(s, 4) as c_int));

    let perm = addr_of_mut!(cp_permutation_order) as *const u8;
    fr_set(fp, CPD_I, 0);
    while fr_get(fp, CPD_I) < fr_get(fp, CPD_NLEN) {
        // gcc evaluates cp_read_bits() first, then indexes cp_permutation_order
        let v = cp_read_bits(s, 3) as u8;
        let idx = *perm.offset(fr_get(fp, CPD_I) as isize) as usize;
        *lenlens.add(idx) = v;
        fr_set(fp, CPD_I, fr_get(fp, CPD_I).wrapping_add(1));
    }
    (*s).nlen = cp_build(ptr::null_mut(), addr_of_mut!((*s).len) as *mut u32, lenlens, 19) as u32;

    fr_set(fp, CPD_N, 0);
    while fr_get(fp, CPD_N) < fr_get(fp, CPD_NLIT).wrapping_add(fr_get(fp, CPD_NDST)) {
        let sym = cp_decode(s, addr_of_mut!((*s).len) as *mut u32, (*s).nlen as c_int);
        fr_set(fp, CPD_SYM, sym);
        match fr_get(fp, CPD_SYM) {
            16 => {
                fr_set(fp, CPD_I16, 3i32.wrapping_add(cp_read_bits(s, 2) as c_int));
                while fr_get(fp, CPD_I16) != 0 {
                    let v = *lens.offset(fr_get(fp, CPD_N).wrapping_sub(1) as isize);
                    *lens.offset(fr_get(fp, CPD_N) as isize) = v;
                    fr_set(fp, CPD_I16, fr_get(fp, CPD_I16).wrapping_sub(1));
                    fr_set(fp, CPD_N, fr_get(fp, CPD_N).wrapping_add(1));
                }
            }
            17 => {
                fr_set(fp, CPD_I17, 3i32.wrapping_add(cp_read_bits(s, 3) as c_int));
                while fr_get(fp, CPD_I17) != 0 {
                    *lens.offset(fr_get(fp, CPD_N) as isize) = 0;
                    fr_set(fp, CPD_I17, fr_get(fp, CPD_I17).wrapping_sub(1));
                    fr_set(fp, CPD_N, fr_get(fp, CPD_N).wrapping_add(1));
                }
            }
            18 => {
                fr_set(fp, CPD_I18, 11i32.wrapping_add(cp_read_bits(s, 7) as c_int));
                while fr_get(fp, CPD_I18) != 0 {
                    *lens.offset(fr_get(fp, CPD_N) as isize) = 0;
                    fr_set(fp, CPD_I18, fr_get(fp, CPD_I18).wrapping_sub(1));
                    fr_set(fp, CPD_N, fr_get(fp, CPD_N).wrapping_add(1));
                }
            }
            _ => {
                // gcc commits `n + 1` to memory *before* storing lens[old n]
                let old = fr_get(fp, CPD_N);
                fr_set(fp, CPD_N, old.wrapping_add(1));
                *lens.offset(old as isize) = fr_get(fp, CPD_SYM) as u8;
            }
        }
    }
    (*s).nlit = cp_build(
        s,
        addr_of_mut!((*s).lit) as *mut u32,
        lens,
        fr_get(fp, CPD_NLIT),
    ) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).dst) as *mut u32,
        lens.offset(fr_get(fp, CPD_NLIT) as isize),
        fr_get(fp, CPD_NDST),
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    let len_extra = addr_of_mut!(cp_len_extra_bits) as *const u8;
    let len_base = addr_of_mut!(cp_len_base) as *const u32;
    let dist_extra = addr_of_mut!(cp_dist_extra_bits) as *const u8;
    let dist_base = addr_of_mut!(cp_dist_base) as *const u32;
    loop {
        let mut symbol = cp_decode(s, addr_of_mut!((*s).lit) as *mut u32, (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.wrapping_offset(1) as isize <= (*s).out_end as isize) {
                cp_set_error(CP_ERR_OUT_SYMBOL);
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.wrapping_offset(1);
        } else if symbol > 256 {
            symbol = symbol.wrapping_sub(257);
            let length = (cp_read_bits(s, *len_extra.offset(symbol as isize) as c_int))
                .wrapping_add(*len_base.offset(symbol as isize)) as c_int;
            let distance_symbol =
                cp_decode(s, addr_of_mut!((*s).dst) as *mut u32, (*s).ndst as c_int);
            let backwards_distance = (cp_read_bits(
                s,
                *dist_extra.offset(distance_symbol as isize) as c_int,
            ))
            .wrapping_add(*dist_base.offset(distance_symbol as isize)) as c_int;
            if !(((*s).out as isize).wrapping_sub(backwards_distance as isize)
                >= (*s).begin as isize)
            {
                cp_set_error(CP_ERR_BEFORE_OUT);
                return 0;
            }
            if !(((*s).out as isize).wrapping_add(length as isize) <= (*s).out_end as isize) {
                cp_set_error(CP_ERR_OUT_STRING);
                return 0;
            }
            let mut src = (*s).out.wrapping_offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.wrapping_offset(length as isize);
            let mut length = length;
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
                }
                _ => {
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

unsafe fn cp_inflate_run(s: *mut cp_state_t) -> c_int {
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
                cp_set_error(CP_ERR_UNKNOWN_BLOCK);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let layout = std::alloc::Layout::new::<cp_state_t>();
    let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;
    if s.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let first_bytes = (((input as usize).wrapping_add(3) & !3usize).wrapping_sub(input as usize))
        as u32 as c_int;
    (*s).words = (input as *mut u8).wrapping_offset(first_bytes as isize) as *mut u32;
    (*s).word_count = in_bytes.wrapping_sub(first_bytes).wrapping_div(4);
    let last_bytes = in_bytes.wrapping_sub(first_bytes) & 3;
    let mut i: c_int = 0;
    while i < first_bytes {
        (*s).bits |= (*(input as *const u8).offset(i as isize) as u64)
            .wrapping_shl(i.wrapping_mul(8) as u32);
        i = i.wrapping_add(1);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    let mut i: c_int = 0;
    while i < last_bytes {
        (*s).final_word |= (*(input as *const u8)
            .offset(in_bytes.wrapping_sub(last_bytes).wrapping_add(i) as isize) as u32)
            .wrapping_shl(i.wrapping_mul(8) as u32);
        i = i.wrapping_add(1);
    }
    (*s).count = first_bytes.wrapping_mul(8);
    (*s).out = out as *mut c_char;
    (*s).out_end = (*s).out.wrapping_offset(out_bytes as isize);
    (*s).begin = out as *mut c_char;

    let result = cp_inflate_run(s);

    std::alloc::dealloc(s as *mut u8, layout);
    result
}

// ---------------------------------------------------------------------------
// PNG helpers (static in C, kept for completeness)
// ---------------------------------------------------------------------------

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: c_int = (a as c_int).wrapping_add(b as c_int).wrapping_sub(c as c_int);
    let pa: c_int = p.wrapping_sub(a as c_int).wrapping_abs();
    let pb: c_int = p.wrapping_sub(b as c_int).wrapping_abs();
    let pc: c_int = p.wrapping_sub(c as c_int).wrapping_abs();
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

unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    let same = cp_memcmp(start.offset(4), chunk as *const u8, 4) == 0;
    if same && len >= minlen {
        let offset = len.wrapping_add(12) as c_int;
        if ((*png).p.wrapping_offset(offset as isize) as isize) <= (*png).end as isize {
            (*png).p = (*png).p.wrapping_offset(offset as isize);
            return start.offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    while ((*png).p as isize) < (*png).end as isize {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.wrapping_offset(len.wrapping_add(12) as c_int as isize);
        if cp_memcmp(start.offset(4), chunk as *const u8, 4) == 0
            && len >= minlen
            && ((*png).p as isize) <= (*png).end as isize
        {
            return start.offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_memcmp(a: *const u8, b: *const u8, n: usize) -> c_int {
    let mut i = 0usize;
    while i < n {
        let x = *a.add(i);
        let y = *b.add(i);
        if x != y {
            return x as c_int - y as c_int;
        }
        i += 1;
    }
    0
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
                    let v = *raw.offset(x.wrapping_sub(bpp) as isize);
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    let v = *raw.offset(x.wrapping_sub(bpp) as isize) / 2;
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let v = cp_paeth(*raw.offset(x.wrapping_sub(bpp) as isize), 0, 0);
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
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
                x = 0;
                while x < bpp {
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(0);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = *raw.offset(x.wrapping_sub(bpp) as isize);
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    let v = *prev.offset(x as isize);
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = *prev.offset(x as isize);
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    let v = *prev.offset(x as isize) / 2;
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = ((*raw.offset(x.wrapping_sub(bpp) as isize) as c_int
                        + *prev.offset(x as isize) as c_int)
                        / 2) as u8;
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    let v = *prev.offset(x as isize);
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
                while x < len {
                    let v = cp_paeth(
                        *raw.offset(x.wrapping_sub(bpp) as isize),
                        *prev.offset(x as isize),
                        *prev.offset(x.wrapping_sub(bpp) as isize),
                    );
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(v);
                    x = x.wrapping_add(1);
                }
            }
            _ => return 0,
        }
        y = y.wrapping_add(1);
        prev = raw;
        raw = raw.offset(len as isize);
    }
    1
}

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
        src = src.wrapping_offset(1);
        let mut x: c_int = 0;
        while x < w {
            match bpp {
                1 => {
                    *dst = cp_make_pixel(*src.offset(0), *src.offset(0), *src.offset(0));
                    dst = dst.wrapping_offset(1);
                }
                2 => {
                    *dst = cp_make_pixel_a(
                        *src.offset(0),
                        *src.offset(0),
                        *src.offset(0),
                        *src.offset(1),
                    );
                    dst = dst.wrapping_offset(1);
                }
                3 => {
                    *dst = cp_make_pixel(*src.offset(0), *src.offset(1), *src.offset(2));
                    dst = dst.wrapping_offset(1);
                }
                4 => {
                    *dst = cp_make_pixel_a(
                        *src.offset(0),
                        *src.offset(1),
                        *src.offset(2),
                        *src.offset(3),
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
