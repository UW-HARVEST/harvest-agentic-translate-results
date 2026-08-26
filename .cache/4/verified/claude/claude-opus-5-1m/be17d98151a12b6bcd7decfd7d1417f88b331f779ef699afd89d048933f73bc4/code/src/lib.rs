//! Rust translation of the C library in `c_src/` (a `pinflate` DEFLATE
//! decompressor derived from cute_png / cute_headers by Randy Gaul).
//!
//! The translation is intentionally literal: it mirrors the original control
//! flow, the exact order of validation checks, the exact error strings, and the
//! (buggy) semantics of the original code.
//!
//! `assert()` calls from the C source are reproduced as real, aborting checks.
//! The reference `.so` is built by `c_src/CMakeLists.txt` with no
//! `CMAKE_BUILD_TYPE`, i.e. **without** `-DNDEBUG`, so `__assert_fail` is live
//! in the C library and hostile input makes it print a diagnostic and `abort()`
//! (`SIGABRT`).  `cp_assert_fail()` below reproduces that exactly; omitting the
//! asserts would make the Rust port silently return where the C port dies.
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
// assert() reproduction
// ---------------------------------------------------------------------------

/// Reproduces a failed glibc `assert()`: a diagnostic on `stderr` followed by
/// `abort()` (`SIGABRT`).  The C reference library is compiled without
/// `-DNDEBUG` (see `c_src/CMakeLists.txt`, which sets no `CMAKE_BUILD_TYPE`),
/// so every `assert` in `lib.c` is live and observable by callers as a process
/// abort.
#[cold]
#[inline(never)]
fn cp_assert_fail(expr: &str, line: u32, func: &str) -> ! {
    use std::io::Write;
    let mut err = std::io::stderr();
    let _ = write!(
        err,
        "pinflate: c_src/src/lib.c:{line}: {func}: Assertion `{expr}' failed.\n"
    );
    let _ = err.flush();
    std::process::abort()
}

macro_rules! cp_assert {
    ($cond:expr, $expr:literal, $line:literal, $func:literal) => {
        if !($cond) {
            cp_assert_fail($expr, $line, $func);
        }
    };
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

unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    cp_assert!(
        ((*s).bits_left & 7) == 0,
        "!(s->bits_left & 7)",
        95,
        "cp_ptr"
    );
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
            cp_assert!(
                (*s).word_index <= (*s).word_count,
                "s->word_index <= s->word_count",
                104,
                "cp_peak_bits"
            );
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
    cp_assert!(
        (*s).count >= num_bits_to_read,
        "s->count >= num_bits_to_read",
        115,
        "cp_consume_bits"
    );
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
    cp_assert!(
        num_bits_to_read <= 32,
        "num_bits_to_read <= 32",
        123,
        "cp_read_bits"
    );
    cp_assert!(
        num_bits_to_read >= 0,
        "num_bits_to_read >= 0",
        124,
        "cp_read_bits"
    );
    cp_assert!((*s).bits_left > 0, "s->bits_left > 0", 125, "cp_read_bits");
    cp_assert!((*s).count <= 64, "s->count <= 64", 126, "cp_read_bits");
    cp_assert!(
        cp_would_overflow(s, num_bits_to_read) == 0,
        "!cp_would_overflow(s, num_bits_to_read)",
        127,
        "cp_read_bits"
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
            cp_assert!(len < 16, "len < 16", 154, "cp_build");
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
    let len = 32u32.wrapping_sub(key & 0xF);
    // `assert((search >> len) == (key >> len));`
    //
    // `len` is `uint32_t` and can be 32 (when `key & 0xF == 0`, e.g. for a
    // `tree[-1]` read of a zeroed word), which makes the C shift undefined.
    // gcc at -O0 emits a variable `shr %cl, %reg`, so x86-64 truncates the
    // shift count modulo 32 -- reproduced here with `& 31`.
    cp_assert!(
        (search >> (len & 31)) == (key >> (len & 31)),
        "(search >> len) == (key >> len)",
        217,
        "cp_decode"
    );
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

// ---------------------------------------------------------------------------
// cp_dynamic()'s stack frame
// ---------------------------------------------------------------------------
//
// `cp_dynamic()` declares `uint8_t lens[288 + 32]` and fills it with
//
//     for (int n = 0; n < nlit + ndst;) { ... case 18: for (int i = 11 +
//     cp_read_bits(s, 7); i; --i, ++n) lens[n] = 0; ... }
//
// None of the run-length cases clamp `n` against `nlit + ndst`, so a run that
// starts just below the limit writes up to 137 bytes *past* `lens`, straight
// into `cp_dynamic()`'s other locals.  That is not a theoretical concern: the
// C library observably wedges in an infinite loop on such input because the
// overrun rewrites the loop counters `n` and `i` themselves.
//
// To reproduce it, `lens` and the surrounding locals are modelled inside one
// byte array laid out exactly like the frame gcc 11 -O0 emits for
// `cp_dynamic()` on x86-64 (`sub $0x190,%rsp`; verified against
// `objdump -d` of `c_src/src/lib.c.o`).  Offsets below are `0x190 + (offset
// relative to %rbp)`, i.e. index 0 is the lowest byte of the frame.

/// `sub $0x190,%rsp`
const FR: usize = 0x190;
/// spilled `cp_state_t *s` argument (`-0x188(%rbp)`)
const FR_S: usize = FR - 0x188;
/// `uint8_t lens[288 + 32]` (`-0x180(%rbp)`)
const FR_LENS: usize = FR - 0x180;
/// `uint8_t lenlens[19]` (`-0x40(%rbp)`)
const FR_LENLENS: usize = FR - 0x40;
/// `int sym` (`-0x24(%rbp)`)
const FR_SYM: usize = FR - 0x24;
/// `int nlen` (`-0x20(%rbp)`)
const FR_NLEN: usize = FR - 0x20;
/// `int ndst` (`-0x1c(%rbp)`)
const FR_NDST: usize = FR - 0x1c;
/// `int nlit` (`-0x18(%rbp)`)
const FR_NLIT: usize = FR - 0x18;
/// `int i` of `case 18` (`-0x14(%rbp)`)
const FR_I18: usize = FR - 0x14;
/// `int i` of `case 17` (`-0x10(%rbp)`)
const FR_I17: usize = FR - 0x10;
/// `int i` of `case 16` (`-0xc(%rbp)`)
const FR_I16: usize = FR - 0xc;
/// `int n` (`-0x8(%rbp)`)
const FR_N: usize = FR - 0x8;
/// `int i` of the code-length-permutation loop (`-0x4(%rbp)`)
const FR_IPERM: usize = FR - 0x4;

/// Frame model size.  The real frame is `FR` bytes plus the saved `%rbp` and
/// return address; the extra room absorbs the part of the overrun that would
/// run off into `pinflate()`'s frame in the C original.
const FRAME_CAP: usize = 4096;

#[inline]
fn fr_i32(f: &[u8; FRAME_CAP], off: usize) -> c_int {
    c_int::from_le_bytes([f[off], f[off + 1], f[off + 2], f[off + 3]])
}

#[inline]
fn fr_set_i32(f: &mut [u8; FRAME_CAP], off: usize, v: c_int) {
    f[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

/// `lens[k]`.  `k` is signed and may be `-1`: `case 16` reads `lens[n - 1]`
/// with `n == 0`, which in the C original lands on the most significant byte
/// of the spilled `s` pointer -- always 0 on x86-64, where user-space
/// addresses are below 2^48.  Modelling the frame reproduces that for free.
#[inline]
fn fr_lens_get(f: &[u8; FRAME_CAP], k: c_int) -> u8 {
    let off = (FR_LENS as isize).wrapping_add(k as isize);
    if off >= 0 && (off as usize) < FRAME_CAP {
        f[off as usize]
    } else {
        0
    }
}

#[inline]
fn fr_lens_set(f: &mut [u8; FRAME_CAP], k: c_int, v: u8) {
    let off = (FR_LENS as isize).wrapping_add(k as isize);
    if off >= 0 && (off as usize) < FRAME_CAP {
        f[off as usize] = v;
    }
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    let mut f = [0u8; FRAME_CAP];
    // gcc spills the `s` argument to -0x188(%rbp), directly below `lens`.
    f[FR_S..FR_S + 8].copy_from_slice(&(s as usize as u64).to_le_bytes());

    // uint8_t lenlens[19] = {0};  -- `f` is already zeroed.
    let _ = FR_LENLENS;

    // int nlit = 257 + cp_read_bits(s, 5);
    fr_set_i32(&mut f, FR_NLIT, 257i32.wrapping_add(cp_read_bits(s, 5) as c_int));
    // int ndst = 1 + cp_read_bits(s, 5);
    fr_set_i32(&mut f, FR_NDST, 1i32.wrapping_add(cp_read_bits(s, 5) as c_int));
    // int nlen = 4 + cp_read_bits(s, 4);
    fr_set_i32(&mut f, FR_NLEN, 4i32.wrapping_add(cp_read_bits(s, 4) as c_int));

    // for (int i = 0; i < nlen; ++i)
    //   lenlens[cp_permutation_order[i]] = (uint8_t)cp_read_bits(s, 3);
    let perm = addr_of_mut!(cp_permutation_order) as *const u8;
    fr_set_i32(&mut f, FR_IPERM, 0);
    while fr_i32(&f, FR_IPERM) < fr_i32(&f, FR_NLEN) {
        let i = fr_i32(&f, FR_IPERM);
        let idx = *perm.wrapping_offset(i as isize) as c_int;
        let v = cp_read_bits(s, 3) as u8;
        let off = (FR_LENLENS as isize).wrapping_add(idx as isize);
        if off >= 0 && (off as usize) < FRAME_CAP {
            f[off as usize] = v;
        }
        let t = fr_i32(&f, FR_IPERM).wrapping_add(1);
        fr_set_i32(&mut f, FR_IPERM, t);
    }

    // s->nlen = cp_build(0, s->len, lenlens, 19);
    (*s).nlen = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).len) as *mut u32,
        f.as_ptr().add(FR_LENLENS),
        19,
    ) as u32;

    // for (int n = 0; n < nlit + ndst;) { ... }
    fr_set_i32(&mut f, FR_N, 0);
    while fr_i32(&f, FR_N) < fr_i32(&f, FR_NLIT).wrapping_add(fr_i32(&f, FR_NDST)) {
        let sym = cp_decode(s, addr_of_mut!((*s).len) as *mut u32, (*s).nlen as c_int);
        fr_set_i32(&mut f, FR_SYM, sym);
        match fr_i32(&f, FR_SYM) {
            16 => {
                // for (int i = 3 + cp_read_bits(s, 2); i; --i, ++n)
                //   lens[n] = lens[n - 1];
                fr_set_i32(&mut f, FR_I16, 3i32.wrapping_add(cp_read_bits(s, 2) as c_int));
                while fr_i32(&f, FR_I16) != 0 {
                    let n = fr_i32(&f, FR_N);
                    let prev = fr_lens_get(&f, n.wrapping_sub(1));
                    fr_lens_set(&mut f, n, prev);
                    let t = fr_i32(&f, FR_I16).wrapping_sub(1);
                    fr_set_i32(&mut f, FR_I16, t);
                    let t = fr_i32(&f, FR_N).wrapping_add(1);
                    fr_set_i32(&mut f, FR_N, t);
                }
            }
            17 => {
                // for (int i = 3 + cp_read_bits(s, 3); i; --i, ++n) lens[n] = 0;
                fr_set_i32(&mut f, FR_I17, 3i32.wrapping_add(cp_read_bits(s, 3) as c_int));
                while fr_i32(&f, FR_I17) != 0 {
                    let n = fr_i32(&f, FR_N);
                    fr_lens_set(&mut f, n, 0);
                    let t = fr_i32(&f, FR_I17).wrapping_sub(1);
                    fr_set_i32(&mut f, FR_I17, t);
                    let t = fr_i32(&f, FR_N).wrapping_add(1);
                    fr_set_i32(&mut f, FR_N, t);
                }
            }
            18 => {
                // for (int i = 11 + cp_read_bits(s, 7); i; --i, ++n) lens[n] = 0;
                fr_set_i32(&mut f, FR_I18, 11i32.wrapping_add(cp_read_bits(s, 7) as c_int));
                while fr_i32(&f, FR_I18) != 0 {
                    let n = fr_i32(&f, FR_N);
                    fr_lens_set(&mut f, n, 0);
                    let t = fr_i32(&f, FR_I18).wrapping_sub(1);
                    fr_set_i32(&mut f, FR_I18, t);
                    let t = fr_i32(&f, FR_N).wrapping_add(1);
                    fr_set_i32(&mut f, FR_N, t);
                }
            }
            _ => {
                // lens[n++] = (uint8_t)sym;
                //
                // gcc stores the incremented `n` *before* performing the byte
                // store, so a store that aliases `n` wins.
                let old_n = fr_i32(&f, FR_N);
                fr_set_i32(&mut f, FR_N, old_n.wrapping_add(1));
                let v = fr_i32(&f, FR_SYM) as u8;
                fr_lens_set(&mut f, old_n, v);
            }
        }
    }

    // s->nlit = cp_build(s, s->lit, lens, nlit);
    let nlit_final = fr_i32(&f, FR_NLIT);
    (*s).nlit = cp_build(
        s,
        addr_of_mut!((*s).lit) as *mut u32,
        f.as_ptr().add(FR_LENS),
        nlit_final,
    ) as u32;
    // s->ndst = cp_build(0, s->dst, lens + nlit, ndst);
    let nlit_again = fr_i32(&f, FR_NLIT);
    let ndst_final = fr_i32(&f, FR_NDST);
    (*s).ndst = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).dst) as *mut u32,
        f.as_ptr().add(FR_LENS).wrapping_offset(nlit_again as isize),
        ndst_final,
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
