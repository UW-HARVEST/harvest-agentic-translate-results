//! Rust translation of c_src/src/lib.c (a DEFLATE / "pinflate" decompressor).
//!
//! The translation is deliberately literal: control flow, order of validation
//! checks, integer widths, wrap-around behaviour and even the original code's
//! quirks are reproduced exactly.
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so `CMAKE_C_FLAGS` is
//! empty and `NDEBUG` is **not** defined: the ground-truth shared object
//! imports `__assert_fail` and dies with `SIGABRT` on a violated `assert()`.
//! Every `assert()` of the C source is therefore reproduced here by
//! [`cp_assert`], which raises `SIGABRT` via `std::process::abort()`.

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

/// Faithful stand-in for glibc's `assert()` in a non-`NDEBUG` build: on
/// failure the process dies with `SIGABRT`.  `line` is the line number of the
/// corresponding `assert()` in `c_src/src/lib.c`; it is written to stderr so
/// that a divergence report says which assert fired (glibc's own message does
/// the same for the C build).
#[inline]
fn cp_assert(cond: bool, line: u32) {
    if !cond {
        let msg = format!("pinflate: src/lib.rs: c_src/src/lib.c:{line}: assertion failed\n");
        unsafe {
            libc_write(2, msg.as_bytes());
        }
        std::process::abort();
    }
}

unsafe fn libc_write(fd: i32, buf: &[u8]) {
    extern "C" {
        fn write(fd: i32, buf: *const c_void, n: usize) -> isize;
    }
    let mut off = 0usize;
    while off < buf.len() {
        let n = write(fd, buf.as_ptr().add(off) as *const c_void, buf.len() - off);
        if n <= 0 {
            break;
        }
        off += n as usize;
    }
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

/// Pointer to a Huffman-tree field of `cp_state_t`, derived from the pointer to
/// the WHOLE struct rather than from the field.
///
/// This matters: `cp_decode` reads `tree[lo - 1]`, and when `lo == 0` that is a
/// read one `uint32_t` *before* the array.  In C it lands on the preceding
/// struct member (`lookup` for `lit`, `lit` for `dst`, `dst` for `len`).  A
/// Rust pointer created with `addr_of_mut!((*s).lit)` only carries provenance
/// over `lit`, so reading at offset -1 is undefined behaviour and an optimising
/// build miscompiles it.  Going through the struct base keeps the provenance of
/// the entire allocation, so the out-of-bounds-for-the-field read stays
/// in-bounds for the object and behaves exactly like the C.
#[inline]
unsafe fn tree_ptr(s: *mut cp_state_t, off: usize) -> *mut u32 {
    (s as *mut u8).add(off) as *mut u32
}

const OFF_LOOKUP: usize = std::mem::offset_of!(cp_state_t, lookup);
const OFF_LIT: usize = std::mem::offset_of!(cp_state_t, lit);
const OFF_DST: usize = std::mem::offset_of!(cp_state_t, dst);
const OFF_LEN: usize = std::mem::offset_of!(cp_state_t, len);

// ---------------------------------------------------------------------------
// Bit reader
// ---------------------------------------------------------------------------

#[inline]
unsafe fn cp_would_overflow(s: *const cp_state_t, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

#[inline]
unsafe fn cp_ptr(s: *const cp_state_t) -> *mut c_char {
    cp_assert(((*s).bits_left & 7) == 0, 95);
    // (char *)(s->words + s->word_index) - (s->count / 8)
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
            cp_assert((*s).word_index <= (*s).word_count, 104);
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count += (*s).bits_left;
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

#[inline]
unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    cp_assert((*s).count >= num_bits_to_read, 115);
    let mask = (1u64.wrapping_shl(num_bits_to_read as u32)).wrapping_sub(1);
    let bits = ((*s).bits & mask) as u32;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    cp_assert(num_bits_to_read <= 32, 123);
    cp_assert(num_bits_to_read >= 0, 124);
    cp_assert((*s).bits_left > 0, 125);
    cp_assert((*s).count <= 64, 126);
    cp_assert(cp_would_overflow(s, num_bits_to_read) == 0, 127);
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
        std::ptr::write_bytes((s as *mut u8).add(OFF_LOOKUP), 0, 1024);
    }
    for i in 0..sym_count {
        let len = *lens.offset(i as isize) as usize;
        if len != 0 {
            cp_assert(len < 16, 154);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as u32;
            first[len] += 1;
            *tree.offset(slot as isize) =
                code.wrapping_shl((32u32).wrapping_sub(len as u32)) | ((i as u32) << 4) | len as u32;
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    *((s as *mut u8).add(OFF_LOOKUP) as *mut u16).add(j) =
                        ((len << 9) | i as usize) as u16;
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

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        set_error(ERR_LEN_NLEN);
        return 0;
    }
    if !((*s).bits_left / 8 <= LEN as c_int) {
        set_error(ERR_STORED_BEYOND);
        return 0;
    }
    let p = cp_ptr(s);
    std::ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, LEN as usize);
    (*s).out = (*s).out.offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    let table = addr_of_mut!(cp_fixed_table) as *const u8;
    let lit = tree_ptr(s, OFF_LIT);
    let dst = tree_ptr(s, OFF_DST);
    (*s).nlit = cp_build(s, lit, table, 288) as u32;
    (*s).ndst = cp_build(std::ptr::null_mut(), dst, table.offset(288), 32) as u32;
    1
}

/// `s` is a raw pointer, not `&mut`, on purpose: `tree` always points INTO the
/// same `cp_state_t` (it is `s->lit`, `s->dst` or `s->len`).  A `&mut`
/// parameter is `noalias` to LLVM, which would license reordering the reads
/// through `tree` against the writes through `s` -- and an optimised build then
/// disagrees with the C.  Found by the release-profile run of CONFIGS.md rows
/// 25 and 45.
unsafe fn cp_decode(s: *mut cp_state_t, tree: *const u32, hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.wrapping_offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.wrapping_offset((lo - 1) as isize);
    // assert((search >> len) == (key >> len)) with `len = 32 - (key & 0xF)`.
    // In C a shift count of 32 on a `uint32_t` is UB; on x86-64 the shift
    // count is masked to 5 bits, so `len == 32` degenerates to a shift of 0.
    // `& 31` reproduces that exactly.
    let sh = (32u32.wrapping_sub(key & 0xF)) & 31;
    cp_assert((search >> sh) == (key >> sh), 217);
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

// ---------------------------------------------------------------------------
// `cp_dynamic`'s stack frame
//
// `c_src/CMakeLists.txt` sets no CMAKE_BUILD_TYPE, so the ground truth is an
// unoptimised gcc build.  `cp_dynamic` declares `uint8_t lens[288 + 32]`, but
// the repeat opcodes 16/17/18 can push `n` well past 320 -- the original has no
// bound check.  In C those stray bytes land on the *neighbouring locals*, and
// once they reach `n` itself the loop restarts, which is how a malformed stream
// makes the original spin forever.  Reproducing that behaviour requires
// reproducing gcc's frame layout byte for byte:
//
// ```text
//   offset from %rbp   object
//   -0x188             cp_state_t *s          (spilled parameter)
//   -0x180 .. -0x41    uint8_t lens[288 + 32]
//   -0x40  .. -0x2e    uint8_t lenlens[19]
//   -0x2d  .. -0x25    (padding)
//   -0x24              int sym
//   -0x20              int nlen
//   -0x1c              int ndst
//   -0x18              int nlit
//   -0x14              int i   (case 18 counter)
//   -0x10              int i   (case 17 counter)
//   -0xc               int i   (case 16 counter)
//   -0x8               int n
//   -0x4               int i   (HCLEN read loop counter)
// ```
//
// Verified against `objdump -d` of the built `.so`.  Note that `lens[-1]`
// (read by opcode 16 when `n == 0`) aliases the top byte of the spilled `s`
// pointer, which is zero for every canonical x86-64 address.
//
// Because `n` is reset as soon as a write reaches offset `-0x8`, `n` can never
// exceed 379, so the modelled frame covers every reachable index.
// ---------------------------------------------------------------------------

const FR_RBP: usize = 0x190;
const FR_S: usize = FR_RBP - 0x188;
const FR_LENS: usize = FR_RBP - 0x180;
const FR_LENLENS: usize = FR_RBP - 0x40;
const FR_SYM: usize = FR_RBP - 0x24;
const FR_NLEN: usize = FR_RBP - 0x20;
const FR_NDST: usize = FR_RBP - 0x1c;
const FR_NLIT: usize = FR_RBP - 0x18;
const FR_I18: usize = FR_RBP - 0x14;
const FR_I17: usize = FR_RBP - 0x10;
const FR_I16: usize = FR_RBP - 0xc;
const FR_N: usize = FR_RBP - 0x8;
const FR_I: usize = FR_RBP - 0x4;
/// Zeroed padding in front of / behind the modelled frame, so that a corrupted
/// `nlit` making `cp_build` read outside the frame reads zeros instead of
/// trapping (in C it would read further up the stack -- see CONFIGS.md
/// "Excluded").
const FR_PRE: usize = 0x100;
const FR_POST: usize = 0x4000;

#[inline]
unsafe fn fr_geti(fb: *const u8, off: usize) -> c_int {
    (fb.add(off) as *const c_int).read_unaligned()
}

#[inline]
unsafe fn fr_seti(fb: *mut u8, off: usize, v: c_int) {
    (fb.add(off) as *mut c_int).write_unaligned(v)
}

/// `lens[k]`, with `k` allowed to be negative or to run past 320 exactly as in
/// the C.
#[inline]
unsafe fn fr_lens_get(fb: *const u8, k: c_int) -> u8 {
    *fb.offset(FR_LENS as isize + k as isize)
}

#[inline]
unsafe fn fr_lens_set(fb: *mut u8, k: c_int, v: u8) {
    *fb.offset(FR_LENS as isize + k as isize) = v;
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {

    let mut frame: Vec<u8> = vec![0u8; FR_PRE + FR_RBP + FR_POST];
    let fb = frame.as_mut_ptr().add(FR_PRE);
    // the spilled `s` parameter (what `lens[-8 .. 0]` aliases)
    (fb.add(FR_S) as *mut u64).write_unaligned(s as usize as u64);

    // uint8_t lenlens[19] = {0};
    std::ptr::write_bytes(fb.add(FR_LENLENS), 0, 19);

    fr_seti(fb, FR_NLIT, 257 + cp_read_bits(s, 5) as c_int);
    fr_seti(fb, FR_NDST, 1 + cp_read_bits(s, 5) as c_int);
    fr_seti(fb, FR_NLEN, 4 + cp_read_bits(s, 4) as c_int);

    let order = addr_of!(cp_permutation_order) as *const u8;
    fr_seti(fb, FR_I, 0);
    while fr_geti(fb, FR_I) < fr_geti(fb, FR_NLEN) {
        let bits = cp_read_bits(s, 3) as u8;
        let idx = *order.offset(fr_geti(fb, FR_I) as isize) as usize;
        *fb.add(FR_LENLENS + idx) = bits;
        fr_seti(fb, FR_I, fr_geti(fb, FR_I) + 1);
    }

    let len_tree = tree_ptr(s, OFF_LEN);
    (*s).nlen = cp_build(
        std::ptr::null_mut(),
        len_tree,
        fb.add(FR_LENLENS) as *const u8,
        19,
    ) as u32;

    fr_seti(fb, FR_N, 0);
    while fr_geti(fb, FR_N) < fr_geti(fb, FR_NLIT).wrapping_add(fr_geti(fb, FR_NDST)) {
        let sym = cp_decode(s, len_tree as *const u32, (*s).nlen as c_int);
        fr_seti(fb, FR_SYM, sym);
        match fr_geti(fb, FR_SYM) {
            16 => {
                // for (int i = 3 + cp_read_bits(s, 2); i; --i, ++n)
                //   lens[n] = lens[n - 1];
                fr_seti(fb, FR_I16, 3 + cp_read_bits(s, 2) as c_int);
                while fr_geti(fb, FR_I16) != 0 {
                    let n = fr_geti(fb, FR_N);
                    let v = fr_lens_get(fb, n - 1);
                    fr_lens_set(fb, n, v);
                    fr_seti(fb, FR_I16, fr_geti(fb, FR_I16) - 1);
                    fr_seti(fb, FR_N, fr_geti(fb, FR_N) + 1);
                }
            }
            17 => {
                fr_seti(fb, FR_I17, 3 + cp_read_bits(s, 3) as c_int);
                while fr_geti(fb, FR_I17) != 0 {
                    fr_lens_set(fb, fr_geti(fb, FR_N), 0);
                    fr_seti(fb, FR_I17, fr_geti(fb, FR_I17) - 1);
                    fr_seti(fb, FR_N, fr_geti(fb, FR_N) + 1);
                }
            }
            18 => {
                fr_seti(fb, FR_I18, 11 + cp_read_bits(s, 7) as c_int);
                while fr_geti(fb, FR_I18) != 0 {
                    fr_lens_set(fb, fr_geti(fb, FR_N), 0);
                    fr_seti(fb, FR_I18, fr_geti(fb, FR_I18) - 1);
                    fr_seti(fb, FR_N, fr_geti(fb, FR_N) + 1);
                }
            }
            _ => {
                // lens[n++] = (uint8_t)sym;  -- gcc stores the incremented `n`
                // BEFORE the byte write, which matters when `lens[n]` aliases
                // `n` itself.
                let old = fr_geti(fb, FR_N);
                fr_seti(fb, FR_N, old.wrapping_add(1));
                fr_lens_set(fb, old, fr_geti(fb, FR_SYM) as u8);
            }
        }
    }

    let nlit = fr_geti(fb, FR_NLIT);
    let ndst = fr_geti(fb, FR_NDST);
    let lit = tree_ptr(s, OFF_LIT);
    let dst = tree_ptr(s, OFF_DST);
    (*s).nlit = cp_build(s, lit, fb.add(FR_LENS) as *const u8, nlit) as u32;
    (*s).ndst = cp_build(
        std::ptr::null_mut(),
        dst,
        (fb.add(FR_LENS) as *const u8).wrapping_offset(nlit as isize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    let lit = tree_ptr(s, OFF_LIT) as *const u32;
    let dst_tree = tree_ptr(s, OFF_DST) as *const u32;
    let len_extra = addr_of!(cp_len_extra_bits) as *const u8;
    let len_base = addr_of!(cp_len_base) as *const u32;
    let dist_extra = addr_of!(cp_dist_extra_bits) as *const u8;
    let dist_base = addr_of!(cp_dist_base) as *const u32;

    loop {
        let mut symbol = cp_decode(s, lit, (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.wrapping_offset(1) <= (*s).out_end) {
                set_error(ERR_OUT_SYMBOL);
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.offset(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length: c_int = cp_read_bits(s, *len_extra.offset(symbol as isize) as c_int)
                as c_int
                + *len_base.offset(symbol as isize) as c_int;
            let distance_symbol = cp_decode(s, dst_tree, (*s).ndst as c_int);
            let backwards_distance: c_int =
                cp_read_bits(s, *dist_extra.offset(distance_symbol as isize) as c_int) as c_int
                    + *dist_base.offset(distance_symbol as isize) as c_int;
            if !((*s).out.wrapping_offset(-(backwards_distance as isize)) >= (*s).begin) {
                set_error(ERR_BACK_DISTANCE);
                return 0;
            }
            if !((*s).out.wrapping_offset(length as isize) <= (*s).out_end) {
                set_error(ERR_OUT_STRING);
                return 0;
            }
            let mut src = (*s).out.offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.offset(length as isize);
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
    // `calloc(1, sizeof(cp_state_t))` / `free(s)`.  The state is held as a raw
    // pointer for the whole call: `cp_decode` and `cp_build` receive pointers
    // INTO it, so a `&mut` (which is `noalias` and, under Stacked Borrows,
    // invalidates raw pointers derived from it) must never be formed.
    let s: *mut cp_state_t = Box::into_raw(cp_state_t::zeroed());

    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);

    let in_addr = in_ as usize;
    let first_bytes: c_int = (((in_addr + 3) & !3usize) - in_addr) as c_int;
    (*s).words = (in_ as *mut c_char).offset(first_bytes as isize) as *mut u32;
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes: c_int = (in_bytes - first_bytes) & 3;

    let in_u8 = in_ as *const u8;
    for i in 0..first_bytes {
        (*s).bits |= (*in_u8.offset(i as isize) as u64) << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        (*s).final_word |=
            (*in_u8.offset((in_bytes - last_bytes + i) as isize) as u32) << (i * 8);
    }
    (*s).count = first_bytes * 8;
    (*s).out = out as *mut c_char;
    (*s).out_end = (*s).out.wrapping_offset(out_bytes as isize);
    (*s).begin = out as *mut c_char;

    let mut _count: c_int = 0;
    let mut bfinal: u32;
    loop {
        bfinal = cp_read_bits(s, 1);
        let btype = cp_read_bits(s, 2);
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    drop(Box::from_raw(s));
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    drop(Box::from_raw(s));
                    return 0;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    drop(Box::from_raw(s));
                    return 0;
                }
            }
            3 => {
                set_error(ERR_BLOCK_TYPE);
                drop(Box::from_raw(s));
                return 0;
            }
            _ => {}
        }
        _count += 1;
        if bfinal != 0 {
            break;
        }
    }
    drop(Box::from_raw(s));
    1
}
