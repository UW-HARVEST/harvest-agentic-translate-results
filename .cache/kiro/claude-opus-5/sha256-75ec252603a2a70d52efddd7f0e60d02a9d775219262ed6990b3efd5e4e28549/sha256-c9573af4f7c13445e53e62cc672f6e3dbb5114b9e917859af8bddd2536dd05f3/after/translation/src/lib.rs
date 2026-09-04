// Rust translation of c_src/src/lib.c (cute_png style PNG loader).
//
// Faithful, behaviour-preserving translation. Every quirk of the original C
// (including its bugs, out-of-bounds reads, inverted checks and `assert()`
// calls) is reproduced. Raw pointers are used wherever the C performs pointer
// arithmetic that may leave the bounds of an object.
//
// Exported ABI (matches `nm -D` of the C shared library):
//   cp_dist_base, cp_dist_extra_bits, cp_error_reason, cp_fixed_table,
//   cp_inflate, cp_len_base, cp_len_extra_bits, cp_permutation_order,
//   load_png_mem

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings. The C code uses malloc/calloc/free/memcpy/memset/memcmp; its
// observable behaviour (e.g. `malloc(0)` returning non-NULL, and the alignment
// of the returned block, which feeds into `cp_inflate`) depends on using the
// very same allocator.
// ---------------------------------------------------------------------------
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn abort() -> !;
}

/// Reproduces C's `assert()`. The CMake build defines no `NDEBUG`, so asserts
/// are live; a failing assertion raises SIGABRT like glibc's `__assert_fail`.
macro_rules! cp_assert {
    ($cond:expr) => {
        if !($cond) {
            unsafe { abort() }
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
// Exported mutable globals
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

// Raw-pointer accessors for the exported tables. The C indexes some of these
// with values derived from the input stream, which can run past the end of the
// array; raw pointer reads reproduce that without panicking.
#[inline]
fn fixed_table_ptr() -> *const u8 {
    (&raw const cp_fixed_table) as *const u8
}
#[inline]
fn permutation_order_ptr() -> *const u8 {
    (&raw const cp_permutation_order) as *const u8
}
#[inline]
fn len_extra_bits_ptr() -> *const u8 {
    (&raw const cp_len_extra_bits) as *const u8
}
#[inline]
fn len_base_ptr() -> *const u32 {
    (&raw const cp_len_base) as *const u32
}
#[inline]
fn dist_extra_bits_ptr() -> *const u8 {
    (&raw const cp_dist_extra_bits) as *const u8
}
#[inline]
fn dist_base_ptr() -> *const u32 {
    (&raw const cp_dist_base) as *const u32
}

#[inline]
unsafe fn set_error(msg: &'static [u8]) {
    cp_error_reason = msg.as_ptr() as *const c_char;
}

// ---------------------------------------------------------------------------
// DEFLATE state
// ---------------------------------------------------------------------------

// Layout must match the C `struct cp_state_t` byte for byte: `cp_decode` reads
// `tree[-1]`, which for `s->lit` lands in the tail of `s->lookup`.
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
    ((((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits)) < 0) as c_int
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    cp_assert!(((*s).bits_left & 7) == 0);
    ((*s).words.wrapping_offset((*s).word_index as isize) as *mut c_char)
        .wrapping_offset(-(((*s).count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.wrapping_offset((*s).word_index as isize);
            (*s).word_index = (*s).word_index.wrapping_add(1);
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count = (*s).count.wrapping_add(32);
            cp_assert!((*s).word_index <= (*s).word_count);
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count = (*s).count.wrapping_add((*s).bits_left);
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    cp_assert!((*s).count >= num_bits_to_read);
    let bits = ((*s).bits & ((1u64 << (num_bits_to_read & 63)).wrapping_sub(1))) as u32;
    (*s).bits >>= num_bits_to_read & 63;
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

unsafe fn cp_build(s: *mut cp_state_t, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];

    let mut n: c_int = 0;
    while n < sym_count {
        let l = *lens.wrapping_offset(n as isize) as usize;
        counts[l & 15] = counts[l & 15].wrapping_add(1);
        n += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    let mut k: usize = 1;
    while k <= 15 {
        codes[k] = codes[k - 1].wrapping_add(counts[k - 1]) << 1;
        first[k] = first[k - 1].wrapping_add(counts[k - 1]);
        k += 1;
    }
    if !s.is_null() {
        memset(
            (&raw mut (*s).lookup) as *mut c_void,
            0,
            core::mem::size_of::<[u16; 1 << 9]>(),
        );
    }
    let mut i: c_int = 0;
    while i < sym_count {
        let len = *lens.wrapping_offset(i as isize) as c_int;
        if len != 0 {
            cp_assert!(len < 16);
            let code = codes[(len & 15) as usize] as u32;
            codes[(len & 15) as usize] = codes[(len & 15) as usize].wrapping_add(1);
            let slot = first[(len & 15) as usize] as u32;
            first[(len & 15) as usize] = first[(len & 15) as usize].wrapping_add(1);
            *tree.wrapping_offset(slot as i32 as isize) =
                (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j: c_int = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    *((&raw mut (*s).lookup) as *mut u16).wrapping_offset(j as isize) =
                        ((len << 9) | i) as u16;
                    j = j.wrapping_add(1 << len);
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
    memcpy((*s).out as *mut c_void, p as *const c_void, LEN as usize);
    (*s).out = (*s).out.wrapping_offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    (*s).nlit = cp_build(s, (&raw mut (*s).lit) as *mut u32, fixed_table_ptr(), 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (&raw mut (*s).dst) as *mut u32,
        fixed_table_ptr().wrapping_offset(288),
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
    let key = *tree.wrapping_offset(lo.wrapping_sub(1) as isize);
    let len = 32u32.wrapping_sub(key & 0xF);
    // x86 32-bit shifts use the count modulo 32, which is what the C compiler
    // emits for the (UB) `(key & 0xF) == 0` case.
    cp_assert!((search >> (len & 31)) == (key >> (len & 31)));
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

// `cp_dynamic` declares `uint8_t lens[288 + 32]` but a run-length code (symbols
// 16/17/18) can write past the end of it, and it reads `lens[-1]` when n == 0.
// The C therefore reads and writes its own stack frame, and the observable
// result depends on the frame layout gcc chose. This mirrors that layout (from
// `objdump -d` of the reference build, gcc at -O0, offsets relative to `lens`)
// so the behaviour is reproduced deterministically instead of being a Rust
// out-of-bounds access:
//
//   rbp-0x188  s (spilled parameter)   -> lens[-8 .. -1]
//   rbp-0x180  uint8_t lens[320]       -> 0
//   rbp-0x040  uint8_t lenlens[19]     -> 320
//              (9 bytes of padding)    -> 339
//   rbp-0x024  int sym                 -> 348
//   rbp-0x020  int nlen                -> 352
//   rbp-0x01c  int ndst                -> 356
//   rbp-0x018  int nlit                -> 360
//   rbp-0x014  int i  (case 18)        -> 364
//   rbp-0x010  int i  (case 17)        -> 368
//   rbp-0x00c  int i  (case 16)        -> 372
//   rbp-0x008  int n                   -> 376
//   rbp-0x004  int i  (lenlens loop)   -> 380
//
// Every local is read back from memory on each use, exactly as gcc -O0 does, so
// a write that lands on `n`, `nlit`, `ndst` or a loop counter has the same
// effect here as it does in the C.
const DYN_LENS: usize = 8; // offset of lens[0] within the emulated frame
const DYN_FRAME: usize = DYN_LENS + 320 + 64 + 4096;

#[repr(align(8))]
struct DynFrame([u8; DYN_FRAME]);

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    let mut frame = DynFrame([0u8; DYN_FRAME]);
    let base = frame.0.as_mut_ptr();
    // `lens[-1]` picks up the top byte of the spilled `s` pointer, which is
    // always zero for a user-space x86-64 address.
    (base as *mut usize).write(s as usize);
    let lens = base.add(DYN_LENS);
    let lenlens = lens.add(320); // uint8_t lenlens[19] = {0}
    let p_sym = lens.add(348) as *mut c_int;
    let p_nlen = lens.add(352) as *mut c_int;
    let p_ndst = lens.add(356) as *mut c_int;
    let p_nlit = lens.add(360) as *mut c_int;
    let p_i18 = lens.add(364) as *mut c_int;
    let p_i17 = lens.add(368) as *mut c_int;
    let p_i16 = lens.add(372) as *mut c_int;
    let p_n = lens.add(376) as *mut c_int;
    let p_iperm = lens.add(380) as *mut c_int;

    // Keeps every frame access inside the emulated buffer; the C would simply
    // walk off its stack frame here.
    let put = |off: isize, v: u8| {
        let idx = (DYN_LENS as isize).wrapping_add(off);
        if idx >= 0 && (idx as usize) < DYN_FRAME {
            *base.offset(idx) = v;
        }
    };
    let get = |off: isize| -> u8 {
        let idx = (DYN_LENS as isize).wrapping_add(off);
        if idx >= 0 && (idx as usize) < DYN_FRAME {
            *base.offset(idx)
        } else {
            0
        }
    };

    *p_nlit = 257i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    *p_ndst = 1i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    *p_nlen = 4i32.wrapping_add(cp_read_bits(s, 4) as c_int);
    *p_iperm = 0;
    while *p_iperm < *p_nlen {
        let v = cp_read_bits(s, 3) as u8;
        let idx = *permutation_order_ptr().wrapping_offset(*p_iperm as isize) as usize;
        *lenlens.add(idx) = v;
        *p_iperm = (*p_iperm).wrapping_add(1);
    }
    (*s).nlen = cp_build(
        ptr::null_mut(),
        (&raw mut (*s).len) as *mut u32,
        lenlens,
        19,
    ) as u32;

    *p_n = 0;
    while *p_n < (*p_nlit).wrapping_add(*p_ndst) {
        *p_sym = cp_decode(s, (&raw mut (*s).len) as *mut u32, (*s).nlen as c_int);
        match *p_sym {
            16 => {
                *p_i16 = 3i32.wrapping_add(cp_read_bits(s, 2) as c_int);
                while *p_i16 != 0 {
                    let v = get((*p_n).wrapping_sub(1) as isize);
                    put(*p_n as isize, v);
                    *p_i16 = (*p_i16).wrapping_sub(1);
                    *p_n = (*p_n).wrapping_add(1);
                }
            }
            17 => {
                *p_i17 = 3i32.wrapping_add(cp_read_bits(s, 3) as c_int);
                while *p_i17 != 0 {
                    put(*p_n as isize, 0);
                    *p_i17 = (*p_i17).wrapping_sub(1);
                    *p_n = (*p_n).wrapping_add(1);
                }
            }
            18 => {
                *p_i18 = 11i32.wrapping_add(cp_read_bits(s, 7) as c_int);
                while *p_i18 != 0 {
                    put(*p_n as isize, 0);
                    *p_i18 = (*p_i18).wrapping_sub(1);
                    *p_n = (*p_n).wrapping_add(1);
                }
            }
            _ => {
                // gcc -O0 stores the incremented `n` before the assignment, so
                // a write that lands on `n` cannot be undone by the increment.
                let old = *p_n;
                *p_n = old.wrapping_add(1);
                put(old as isize, *p_sym as u8);
            }
        }
    }
    (*s).nlit = cp_build(s, (&raw mut (*s).lit) as *mut u32, lens, *p_nlit) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (&raw mut (*s).dst) as *mut u32,
        lens.wrapping_offset(*p_nlit as isize),
        *p_ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    loop {
        let mut symbol = cp_decode(s, (&raw mut (*s).lit) as *mut u32, (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.wrapping_offset(1) <= (*s).out_end) {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.wrapping_offset(1);
        } else if symbol > 256 {
            symbol = symbol.wrapping_sub(257);
            let len_bits = *len_extra_bits_ptr().wrapping_offset(symbol as isize) as c_int;
            let mut length = cp_read_bits(s, len_bits)
                .wrapping_add(*len_base_ptr().wrapping_offset(symbol as isize))
                as c_int;
            let distance_symbol = cp_decode(s, (&raw mut (*s).dst) as *mut u32, (*s).ndst as c_int);
            let dist_bits =
                *dist_extra_bits_ptr().wrapping_offset(distance_symbol as isize) as c_int;
            let backwards_distance = cp_read_bits(s, dist_bits)
                .wrapping_add(*dist_base_ptr().wrapping_offset(distance_symbol as isize))
                as c_int;
            if !((*s).out.wrapping_offset(-(backwards_distance as isize)) >= (*s).begin) {
                set_error(b"Attempted to write before out buffer (invalid backwards distance).\0");
                return 0;
            }
            if !((*s).out.wrapping_offset(length as isize) <= (*s).out_end) {
                set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                return 0;
            }
            let mut src = (*s).out.wrapping_offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.wrapping_offset(length as isize);
            match backwards_distance {
                1 => {
                    memset(dst as *mut c_void, *src as c_int, length as isize as usize);
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
    (*s).words = (in_ as *mut u8).wrapping_offset(first_bytes as isize) as *mut u32;
    (*s).word_count = in_bytes.wrapping_sub(first_bytes) / 4;
    let last_bytes = in_bytes.wrapping_sub(first_bytes) & 3;
    let mut i: c_int = 0;
    while i < first_bytes {
        (*s).bits |= (*(in_ as *const u8).wrapping_offset(i as isize) as u64) << (i * 8);
        i += 1;
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    let mut i: c_int = 0;
    while i < last_bytes {
        (*s).final_word |= (*(in_ as *const u8)
            .wrapping_offset(in_bytes.wrapping_sub(last_bytes).wrapping_add(i) as isize)
            as u32)
            << (i * 8);
        i += 1;
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
                set_error(b"Detected unknown block type within input stream.\0");
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

// ---------------------------------------------------------------------------
// PNG decoding
// ---------------------------------------------------------------------------

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: c_int = (a as c_int).wrapping_add(b as c_int).wrapping_sub(c as c_int);
    let pa = p.wrapping_sub(a as c_int).wrapping_abs();
    let pb = p.wrapping_sub(b as c_int).wrapping_abs();
    let pc = p.wrapping_sub(c as c_int).wrapping_abs();
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

unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if memcmp(
        start.wrapping_offset(4) as *const c_void,
        chunk as *const c_void,
        4,
    ) == 0
        && len >= minlen
    {
        // `int offset = len + 12;` -- signed, so it sign-extends for the
        // pointer arithmetic (unlike cp_find below, which keeps it unsigned).
        let offset = len.wrapping_add(12) as c_int;
        if (*png).p.wrapping_offset(offset as isize) <= (*png).end {
            (*png).p = (*png).p.wrapping_offset(offset as isize);
            return start.wrapping_offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        // `png->p += len + 12;` -- `len + 12` has type uint32_t, so it is
        // zero-extended for the pointer arithmetic.
        (*png).p = (*png).p.wrapping_add(len.wrapping_add(12) as usize);
        if memcmp(
            start.wrapping_offset(4) as *const c_void,
            chunk as *const c_void,
            4,
        ) == 0
            && len >= minlen
            && (*png).p <= (*png).end
        {
            return start.wrapping_offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw_in: *mut u8) -> c_int {
    let len = w.wrapping_mul(bpp);
    let mut raw = raw_in;
    let mut prev: *mut u8;
    let mut x: c_int;

    macro_rules! r {
        ($i:expr) => {
            *raw.wrapping_offset(($i) as isize)
        };
    }

    if h > 0 {
        let filter = *raw;
        raw = raw.wrapping_offset(1);
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    r!(x) = r!(x).wrapping_add(r!(x.wrapping_sub(bpp)));
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    r!(x) = r!(x).wrapping_add(r!(x.wrapping_sub(bpp)) / 2);
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    r!(x) = r!(x).wrapping_add(cp_paeth(r!(x.wrapping_sub(bpp)), 0, 0));
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    prev = raw;
    raw = raw.wrapping_offset(len as isize);

    macro_rules! pv {
        ($i:expr) => {
            *prev.wrapping_offset(($i) as isize)
        };
    }

    let mut y: c_int = 1;
    while y < h {
        let filter = *raw;
        raw = raw.wrapping_offset(1);
        match filter {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    r!(x) = r!(x).wrapping_add(0);
                    x += 1;
                }
                while x < len {
                    r!(x) = r!(x).wrapping_add(r!(x.wrapping_sub(bpp)));
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    r!(x) = r!(x).wrapping_add(pv!(x));
                    x += 1;
                }
                while x < len {
                    r!(x) = r!(x).wrapping_add(pv!(x));
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    r!(x) = r!(x).wrapping_add(pv!(x) / 2);
                    x += 1;
                }
                while x < len {
                    let sum = (r!(x.wrapping_sub(bpp)) as c_int).wrapping_add(pv!(x) as c_int) / 2;
                    r!(x) = r!(x).wrapping_add(sum as u8);
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    r!(x) = r!(x).wrapping_add(pv!(x));
                    x += 1;
                }
                while x < len {
                    let p = cp_paeth(r!(x.wrapping_sub(bpp)), pv!(x), pv!(x.wrapping_sub(bpp)));
                    r!(x) = r!(x).wrapping_add(p);
                    x += 1;
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

unsafe fn cp_convert(bpp: c_int, w: c_int, h: c_int, src_in: *mut u8, dst_in: *mut cp_pixel_t) {
    let mut src = src_in;
    let mut dst = dst_in;
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
                    *dst = cp_make_pixel(*src, *src.wrapping_offset(1), *src.wrapping_offset(2));
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
            x += 1;
            src = src.wrapping_offset(bpp as isize);
        }
        y += 1;
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
    src_in: *mut u8,
    dst_in: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    let mut src = src_in;
    let mut dst = dst_in;
    let mut y: c_int = 0;
    while y < h {
        src = src.wrapping_offset(1);
        let mut x: c_int = 0;
        while x < w {
            let c = *src as c_int;
            let r = *plte.wrapping_offset(c.wrapping_mul(3) as isize);
            let g = *plte.wrapping_offset(c.wrapping_mul(3).wrapping_add(1) as isize);
            let b = *plte.wrapping_offset(c.wrapping_mul(3).wrapping_add(2) as isize);
            let a = cp_get_alpha_for_indexed_image(c, trns, trns_len);
            *dst = cp_make_pixel_a(r, g, b, a);
            dst = dst.wrapping_offset(1);
            x += 1;
            src = src.wrapping_offset(1);
        }
        y += 1;
    }
}

unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    cp_make32(chunk.wrapping_offset(-8))
}

unsafe fn cp_out_size(img: *const cp_image_t, bpp: c_int) -> c_int {
    (*img)
        .w
        .wrapping_add(1)
        .wrapping_mul((*img).h)
        .wrapping_mul(bpp)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let sig: &[u8; 9] = b"\x89PNG\r\n\x1a\n\0";

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

    macro_rules! cp_err {
        ($msg:expr) => {{
            set_error($msg);
            free(data as *mut c_void);
            free(img.pix as *mut c_void);
            img.pix = ptr::null_mut();
            return img;
        }};
    }

    if !(memcmp(png.p as *const c_void, sig.as_ptr() as *const c_void, 8) == 0) {
        cp_err!(b"incorrect file signature (is this a png file?)\0");
    }
    png.p = png.p.wrapping_offset(8);

    let ihdr = cp_chunk(
        &mut png as *mut cp_raw_png_t,
        b"IHDR\0".as_ptr() as *const c_char,
        13,
    );
    if ihdr.is_null() {
        cp_err!(b"unable to find IHDR chunk\0");
    }

    let bit_depth = *ihdr.wrapping_offset(8) as c_int;
    let color_type = *ihdr.wrapping_offset(9) as c_int;
    if !(bit_depth == 8) {
        cp_err!(b"only bit-depth of 8 is supported\0");
    }

    let bpp: c_int = match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => cp_err!(b"unknown color type\0"),
    };

    let w: c_int = cp_make32(ihdr).wrapping_add(1) as c_int;
    let h: c_int = cp_make32(ihdr.wrapping_offset(4)) as c_int;
    if !(w >= 1) {
        cp_err!(b"invalid IHDR chunk found, image width was less than 1\0");
    }
    if !(h >= 1) {
        cp_err!(b"invalid IHDR chunk found, image height was less than 1\0");
    }
    // (int64_t)w * h * sizeof(cp_pixel_t) < INT_MAX -- the sizeof operand makes
    // the multiplication and the comparison unsigned.
    if !(((w as i64).wrapping_mul(h as i64) as u64)
        .wrapping_mul(core::mem::size_of::<cp_pixel_t>() as u64)
        < c_int::MAX as u64)
    {
        cp_err!(b"image too large\0");
    }
    let pix_bytes: c_int = ((w.wrapping_mul(h) as i64 as u64)
        .wrapping_mul(core::mem::size_of::<cp_pixel_t>() as u64)) as c_int;
    img.w = w.wrapping_sub(1);
    img.h = h;
    img.pix = malloc(pix_bytes as isize as usize) as *mut cp_pixel_t;
    if img.pix.is_null() {
        cp_err!(b"unable to allocate raw image space\0");
    }

    let compression = *ihdr.wrapping_offset(10) as c_int;
    let filter = *ihdr.wrapping_offset(11) as c_int;
    let interlace = *ihdr.wrapping_offset(12) as c_int;
    if !(compression == 0) {
        cp_err!(b"only standard compression DEFLATE is supported\0");
    }
    if !(filter == 0) {
        cp_err!(b"only standard adaptive filtering is supported\0");
    }
    if !(interlace == 0) {
        cp_err!(b"interlacing is not supported\0");
    }

    let mut first = png.p;
    let plte = cp_find(
        &mut png as *mut cp_raw_png_t,
        b"PLTE\0".as_ptr() as *const c_char,
        0,
    );
    if plte.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    let trns = cp_find(
        &mut png as *mut cp_raw_png_t,
        b"tRNS\0".as_ptr() as *const c_char,
        0,
    );
    if trns.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }

    let mut datalen: c_int = 0;
    let mut idat = cp_find(
        &mut png as *mut cp_raw_png_t,
        b"IDAT\0".as_ptr() as *const c_char,
        0,
    );
    while !idat.is_null() {
        let len = cp_get_chunk_byte_length(idat);
        datalen = datalen.wrapping_add(len as c_int);
        idat = cp_chunk(
            &mut png as *mut cp_raw_png_t,
            b"IDAT\0".as_ptr() as *const c_char,
            0,
        );
    }
    png.p = first;
    data = malloc(datalen as isize as usize) as *mut u8;
    let mut offset: c_int = 0;
    let mut idat = cp_find(
        &mut png as *mut cp_raw_png_t,
        b"IDAT\0".as_ptr() as *const c_char,
        0,
    );
    while !idat.is_null() {
        let len = cp_get_chunk_byte_length(idat);
        memcpy(
            data.wrapping_offset(offset as isize) as *mut c_void,
            idat as *const c_void,
            len as usize,
        );
        offset = offset.wrapping_add(len as c_int);
        idat = cp_chunk(
            &mut png as *mut cp_raw_png_t,
            b"IDAT\0".as_ptr() as *const c_char,
            0,
        );
    }

    if !(!data.is_null() && datalen >= 6) {
        cp_err!(b"corrupt zlib structure in DEFLATE stream\0");
    }
    if !((*data.wrapping_offset(0) as c_int & 0x0f) == 0x08) {
        cp_err!(b"only zlib compression method (RFC 1950) is supported\0");
    }
    if !((*data.wrapping_offset(0) as c_int & 0xf0) <= 0x70) {
        cp_err!(b"innapropriate window size detected\0");
    }
    if !((*data.wrapping_offset(1) as c_int & 0x20) == 0) {
        cp_err!(b"preset dictionary is present and not supported\0");
    }
    if !(cp_out_size(&img as *const cp_image_t, 4) >= 1) {
        cp_err!(b"invalid image size found\0");
    }
    if !(cp_out_size(&img as *const cp_image_t, bpp) >= 1) {
        cp_err!(b"invalid image size found\0");
    }

    let out = (img.pix as *mut u8)
        .wrapping_offset(cp_out_size(&img as *const cp_image_t, 4) as isize)
        .wrapping_offset(-(cp_out_size(&img as *const cp_image_t, bpp) as isize));

    if cp_inflate(
        data.wrapping_offset(2) as *mut c_void,
        datalen.wrapping_sub(6),
        out as *mut c_void,
        pix_bytes,
    ) == 0
    {
        cp_err!(b"DEFLATE algorithm failed\0");
    }
    if cp_unfilter(img.w, img.h, bpp, out) == 0 {
        cp_err!(b"invalid filter byte found\0");
    }

    if color_type == 3 {
        if plte.is_null() {
            cp_err!(b"color type of indexed requires a PLTE chunk\0");
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
    img
}
