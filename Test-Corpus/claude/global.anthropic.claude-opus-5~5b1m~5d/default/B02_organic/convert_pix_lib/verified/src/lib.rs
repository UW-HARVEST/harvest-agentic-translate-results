//! Rust translation of the C library in `c_src/` (a stripped-down `cute_png`).
//!
//! The translation is deliberately literal: it mirrors the C control flow,
//! integer widths, wrap-around arithmetic, pointer arithmetic and the exact
//! order of validation checks.  Bugs present in the original C are reproduced
//! rather than fixed.
//!
//! Exported ABI (matches `nm -D` on the C shared object):
//!   * `convert_pix`            (function)
//!   * `cp_inflate`             (function)
//!   * `cp_error_reason`        (data, `const char *`)
//!   * `cp_fixed_table`         (data, `uint8_t[288+32]`)
//!   * `cp_permutation_order`   (data, `uint8_t[19]`)
//!   * `cp_len_extra_bits`      (data, `uint8_t[29+2]`)
//!   * `cp_len_base`            (data, `uint32_t[29+2]`)
//!   * `cp_dist_extra_bits`     (data, `uint8_t[30+2]`)
//!   * `cp_dist_base`           (data, `uint32_t[30+2]`)

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// assert() emulation
//
// The reference C library is compiled by the supplied CMakeLists.txt without
// NDEBUG, so `assert()` is live and a failing assertion terminates the process
// with SIGABRT.  Reproduce that: report on stderr, then abort.
// ---------------------------------------------------------------------------

/// Absolute path of `c_src/src/lib.c`, as cmake hands it to the C compiler
/// (`__FILE__`).  Supplied by `build.rs`.
const CP_C_FILE: &str = env!("CP_C_SOURCE_PATH");

unsafe extern "C" {
    /// glibc's `assert` prefixes its message with this.
    static program_invocation_short_name: *const c_char;
}

fn cp_progname() -> String {
    // Mirror glibc: __progname / program_invocation_short_name.
    let p = unsafe { program_invocation_short_name };
    if !p.is_null() {
        let mut n = 0usize;
        unsafe {
            while *p.add(n) != 0 {
                n += 1;
            }
            return String::from_utf8_lossy(std::slice::from_raw_parts(p as *const u8, n))
                .into_owned();
        }
    }
    String::new()
}

/// Reproduce glibc's `__assert_fail`:
/// `"%s%s%s:%u: %s%sAssertion `%s' failed.\n"` followed by `abort()`.
#[cold]
#[inline(never)]
fn cp_assert_fail(assertion: &str, function: &str, line: u32) -> ! {
    use std::io::Write;
    let progname = cp_progname();
    let sep = if progname.is_empty() { "" } else { ": " };
    let mut err = std::io::stderr();
    let _ = write!(
        err,
        "{}{}{}:{}: {}: Assertion `{}' failed.\n",
        progname, sep, CP_C_FILE, line, function, assertion
    );
    let _ = err.flush();
    std::process::abort();
}

/// `assert(cond)` where `c_expr`/`c_line` are the *C* source text and line, so
/// that a failing assertion produces exactly the reference diagnostic.
macro_rules! cp_assert {
    ($cond:expr, $func:expr, $c_expr:expr, $c_line:expr) => {
        if !($cond) {
            cp_assert_fail($c_expr, $func, $c_line);
        }
    };
}

// ---------------------------------------------------------------------------
// include/lib.h
// ---------------------------------------------------------------------------

/// `typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

// ---------------------------------------------------------------------------
// src/lib.c
// ---------------------------------------------------------------------------

/// `typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

#[inline]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    let mut p = cp_pixel_t { r: 0, g: 0, b: 0, a: 0 };
    p.r = r;
    p.g = g;
    p.b = b;
    p.a = a;
    p
}

#[inline]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    let mut p = cp_pixel_t { r: 0, g: 0, b: 0, a: 0 };
    p.r = r;
    p.g = g;
    p.b = b;
    p.a = 0xFF;
    p
}

// --- exported globals ------------------------------------------------------

/// `const char *cp_error_reason;`
#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

const fn cp_build_fixed_table() -> [u8; 288 + 32] {
    // 144 * 8, 112 * 9, 24 * 7, 8 * 8, 32 * 5  (verified against the C object)
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

/// `uint8_t cp_fixed_table[288 + 32]`
#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = cp_build_fixed_table();

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
// Reproduction of the C object's `.data` image
//
// `cp_block` indexes `cp_len_extra_bits` / `cp_len_base` with `symbol - 257`
// and `cp_dist_extra_bits` / `cp_dist_base` with `distance_symbol`, and the C
// range-checks NEITHER.  Both indices come out of `cp_decode`, which returns
// `(key >> 4) & 0xFFF`, so they can be far past the end of the arrays.
//
// The out-of-bounds case IS reachable: `cp_decode` reads `tree[-1]` whenever it
// is handed an empty tree (`hi == 0`), and a dynamic block may legally declare
// `HDIST` distance codes and give all of them code length 0, leaving
// `s->ndst == 0`.  `s->dst[-1]` is then `s->lit[287]`, a well-formed entry whose
// symbol field is `287`, so `cp_dist_extra_bits[287]` / `cp_dist_base[287]` get
// read — 255 entries past the end.
//
// The C therefore reads whatever the linker placed next in `.data`.  From
// `readelf -SW` / `readelf -sW` on the reference object (`.data` at 0x5040,
// size 0x2a0; `.bss` at 0x52e0, size 0x10) that image is:
//
//     +0x000  cp_fixed_table          320 bytes
//     +0x140  cp_permutation_order     19 bytes  (+13 bytes padding)
//     +0x160  cp_len_extra_bits        31 bytes  (+1  byte  padding)
//     +0x180  cp_len_base             124 bytes  (+4  bytes padding)
//     +0x200  cp_dist_extra_bits       32 bytes
//     +0x220  cp_dist_base            128 bytes   -> `.data` ends at +0x2a0
//     +0x2a0  `.bss`: libc's `completed.0` guard byte, 0 while the library is
//             in use, plus 7 bytes of padding
//     +0x2a8  `.bss`: cp_error_reason (an 8-byte pointer)
//     +0x2b0  remainder of the last mapped page: zero
//
// Rust gives its statics a different order and no padding, so the reads have to
// go through this translation layer instead of through plain indexing.
// ---------------------------------------------------------------------------

const CPD_FIXED: isize = 0x000;
const CPD_PERM: isize = 0x140;
const CPD_LEN_EXTRA: isize = 0x160;
const CPD_LEN_BASE: isize = 0x180;
const CPD_DIST_EXTRA: isize = 0x200;
const CPD_DIST_BASE: isize = 0x220;
const CPD_ERROR_REASON: isize = 0x2a8;

/// One byte of the C's `.data`/`.bss` image, `off` bytes past `cp_fixed_table`.
///
/// Offsets that fall in inter-object padding, in `.bss` holes, or past the end
/// of `.bss` read as zero — exactly what the C sees, because the rest of the
/// final mapped page of the data segment is zero-filled by the loader.
///
/// Offsets outside the C's mapped data segment (below `-0x1040` or above
/// `+0xfc0`) would fault in the C.  They are unreachable here: `cp_decode`
/// returns at most `4095` and, as argued above, at most `287` in the only
/// out-of-bounds situation that exists, so the largest offset ever produced is
/// `CPD_DIST_BASE + 4 * 287 + 3 = 0x69f`.
unsafe fn cp_data_byte(off: isize) -> u8 {
    if off >= CPD_FIXED && off < CPD_FIXED + 320 {
        *((&raw const cp_fixed_table) as *const u8).offset(off - CPD_FIXED)
    } else if off >= CPD_PERM && off < CPD_PERM + 19 {
        *((&raw const cp_permutation_order) as *const u8).offset(off - CPD_PERM)
    } else if off >= CPD_LEN_EXTRA && off < CPD_LEN_EXTRA + 31 {
        *((&raw const cp_len_extra_bits) as *const u8).offset(off - CPD_LEN_EXTRA)
    } else if off >= CPD_LEN_BASE && off < CPD_LEN_BASE + 124 {
        *((&raw const cp_len_base) as *const u8).offset(off - CPD_LEN_BASE)
    } else if off >= CPD_DIST_EXTRA && off < CPD_DIST_EXTRA + 32 {
        *((&raw const cp_dist_extra_bits) as *const u8).offset(off - CPD_DIST_EXTRA)
    } else if off >= CPD_DIST_BASE && off < CPD_DIST_BASE + 128 {
        *((&raw const cp_dist_base) as *const u8).offset(off - CPD_DIST_BASE)
    } else if off >= CPD_ERROR_REASON && off < CPD_ERROR_REASON + 8 {
        *((&raw const cp_error_reason) as *const u8).offset(off - CPD_ERROR_REASON)
    } else {
        0
    }
}

/// A little-endian `uint32_t` read out of the same image.
unsafe fn cp_data_u32(off: isize) -> u32 {
    (cp_data_byte(off) as u32)
        | ((cp_data_byte(off + 1) as u32) << 8)
        | ((cp_data_byte(off + 2) as u32) << 16)
        | ((cp_data_byte(off + 3) as u32) << 24)
}

/// `cp_len_extra_bits[i]`, including out-of-bounds `i`.
unsafe fn cp_len_extra_at(i: c_int) -> u8 {
    if (i as u32) < 31 {
        *((&raw const cp_len_extra_bits) as *const u8).offset(i as isize)
    } else {
        cp_data_byte(CPD_LEN_EXTRA + i as isize)
    }
}

/// `cp_len_base[i]`, including out-of-bounds `i`.
unsafe fn cp_len_base_at(i: c_int) -> u32 {
    if (i as u32) < 31 {
        *((&raw const cp_len_base) as *const u32).offset(i as isize)
    } else {
        cp_data_u32(CPD_LEN_BASE + (i as isize) * 4)
    }
}

/// `cp_dist_extra_bits[i]`, including out-of-bounds `i`.
unsafe fn cp_dist_extra_at(i: c_int) -> u8 {
    if (i as u32) < 32 {
        *((&raw const cp_dist_extra_bits) as *const u8).offset(i as isize)
    } else {
        cp_data_byte(CPD_DIST_EXTRA + i as isize)
    }
}

/// `cp_dist_base[i]`, including out-of-bounds `i`.
unsafe fn cp_dist_base_at(i: c_int) -> u32 {
    if (i as u32) < 32 {
        *((&raw const cp_dist_base) as *const u32).offset(i as isize)
    } else {
        cp_data_u32(CPD_DIST_BASE + (i as isize) * 4)
    }
}

// --- error strings (byte-for-byte identical to the C literals) -------------

const ERR_STORED_COMPLEMENT: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const ERR_STORED_BEYOND: &[u8] = b"Stored block extends beyond end of input stream.\0";
const ERR_OUT_SYMBOL: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.\0";
const ERR_BAD_DISTANCE: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const ERR_OUT_STRING: &[u8] = b"Attempted to overwrite out buffer while outputting a string.\0";
const ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";

// --- inflate state ---------------------------------------------------------

/// `typedef struct cp_state_t { ... } cp_state_t;`
///
/// `#[repr(C)]` with the original field order so that the (deliberately
/// reproduced) out-of-bounds `tree[-1]` read in `cp_decode` lands on the same
/// neighbouring field as it does in C.
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
    out: *mut u8,
    out_end: *mut u8,
    begin: *mut u8,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut u8 {
    cp_assert!(((*s).bits_left & 7) == 0, "cp_ptr", "!(s->bits_left & 7)", 89);
    ((*s).words as *mut u8)
        .wrapping_offset((*s).word_index as isize * 4)
        .wrapping_offset(-(((*s).count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.wrapping_offset((*s).word_index as isize);
            (*s).word_index = (*s).word_index.wrapping_add(1);
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add(32);
            cp_assert!(
                (*s).word_index <= (*s).word_count,
                "cp_peak_bits",
                "s->word_index <= s->word_count",
                98
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
        "cp_consume_bits",
        "s->count >= num_bits_to_read",
        109
    );
    let bits = ((*s).bits & (1u64.wrapping_shl(num_bits_to_read as u32).wrapping_sub(1))) as u32;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count = (*s).count.wrapping_sub(num_bits_to_read);
    (*s).bits_left = (*s).bits_left.wrapping_sub(num_bits_to_read);
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    cp_assert!(num_bits_to_read <= 32, "cp_read_bits", "num_bits_to_read <= 32", 117);
    cp_assert!(num_bits_to_read >= 0, "cp_read_bits", "num_bits_to_read >= 0", 118);
    cp_assert!((*s).bits_left > 0, "cp_read_bits", "s->bits_left > 0", 119);
    cp_assert!((*s).count <= 64, "cp_read_bits", "s->count <= 64", 120);
    cp_assert!(
        cp_would_overflow(s, num_bits_to_read) == 0,
        "cp_read_bits",
        "!cp_would_overflow(s, num_bits_to_read)",
        121
    );
    cp_peak_bits(s, num_bits_to_read);
    let bits = cp_consume_bits(s, num_bits_to_read);
    bits
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

    let mut n: c_int = 0;
    while n < sym_count {
        let l = *lens.wrapping_offset(n as isize) as usize;
        if l >= 16 {
            // In C this is an out-of-bounds `counts[]` increment; the very same
            // symbol then trips `assert(len < 16)` below, aborting the process.
            // Abort here: observably identical (SIGABRT, no library output).
            cp_assert_fail("len < 16", "cp_build", 148);
        }
        counts[l] += 1;
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
        ptr::write_bytes((&raw mut (*s).lookup) as *mut u16, 0, 1 << 9);
    }
    let mut i: c_int = 0;
    while i < sym_count {
        let len = *lens.wrapping_offset(i as isize) as c_int;
        if len != 0 {
            cp_assert!(len < 16, "cp_build", "len < 16", 148);
            let code = codes[len as usize] as u32;
            codes[len as usize] = codes[len as usize].wrapping_add(1);
            let slot = first[len as usize] as u32;
            first[len as usize] = first[len as usize].wrapping_add(1);
            *tree.wrapping_offset(slot as i32 as isize) = code
                .wrapping_shl((32 - len) as u32)
                | ((i as u32) << 4)
                | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    *((&raw mut (*s).lookup) as *mut u16).wrapping_offset(j as isize) =
                        ((len << 9) | i) as u16;
                    j = j.wrapping_add(1 << len);
                }
            }
        }
        i += 1;
    }
    let max_index = first[15];
    max_index
}

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    let p: *mut u8;
    cp_read_bits(s, (*s).count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        cp_error_reason = ERR_STORED_COMPLEMENT.as_ptr() as *const c_char;
        return 0;
    }
    if !((*s).bits_left / 8 <= LEN as c_int) {
        cp_error_reason = ERR_STORED_BEYOND.as_ptr() as *const c_char;
        return 0;
    }
    p = cp_ptr(s);
    ptr::copy_nonoverlapping(p as *const u8, (*s).out, LEN as usize);
    (*s).out = (*s).out.wrapping_add(LEN as usize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    let table = (&raw mut cp_fixed_table) as *mut u8;
    (*s).nlit = cp_build(s, (&raw mut (*s).lit) as *mut u32, table, 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (&raw mut (*s).dst) as *mut u32,
        table.wrapping_add(288),
        32,
    ) as u32;
    1
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi: c_int) -> c_int {
    let mut hi = hi;
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
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
    cp_assert!(
        search.wrapping_shr(len) == key.wrapping_shr(len),
        "cp_decode",
        "(search >> len) == (key >> len)",
        211
    );
    let code = cp_consume_bits(s, (key & 0xF) as c_int);
    let _ = code;
    ((key >> 4) & 0xFFF) as c_int
}

// ---------------------------------------------------------------------------
// `cp_dynamic`'s stack frame
//
// The run-length loop
//
//     for (int n = 0; n < nlit + ndst;) { ... for (int i = 11 + …; i; --i, ++n)
//                                                 lens[n] = 0; ... }
//
// tests `n < nlit + ndst` only *between* symbols, while one symbol 16/17/18
// writes 3..=138 entries.  The final run can therefore write up to 137 bytes
// past the end of `uint8_t lens[288 + 32]` — and the bytes it lands on are the
// *other locals of the same frame*, several of which the code reads afterwards.
//
// `objdump -d` on the reference object pins the (-O0, no NDEBUG) frame down
// exactly.  Offsets below are relative to the start of this array, which stands
// in for `%rbp-0x190`:
//
//     0x008  spilled `s` argument        (%rbp-0x188) — `lens[-1]` is its top
//                                        byte, always 0 on x86-64 userspace
//     0x010  uint8_t lens[288 + 32]      (%rbp-0x180)
//     0x150  uint8_t lenlens[19]         (%rbp-0x040)  dead after the first build
//     0x16c  int sym                     (%rbp-0x024)  reassigned each pass
//     0x170  int nlen                    (%rbp-0x020)  dead after the HCLEN loop
//     0x174  int ndst                    (%rbp-0x01c)  READ after the loop
//     0x178  int nlit                    (%rbp-0x018)  READ after the loop
//     0x17c  int i  (symbol 18 counter)  (%rbp-0x014)
//     0x180  int i  (symbol 17 counter)  (%rbp-0x010)
//     0x184  int i  (symbol 16 counter)  (%rbp-0x00c)
//     0x188  int n                       (%rbp-0x008)  the loop variable itself
//     0x18c  int i  (HCLEN counter)      (%rbp-0x004)
//     0x190  saved %rbp                  (%rbp+0x000)
//     0x198  return address              (%rbp+0x008)
//
// Modelling the frame explicitly (instead of using Rust locals) is what makes
// the overshoot reproduce the C: zeroing `ndst`/`nlit` changes the trees that
// get built, and zeroing `n` restarts the loop at a lower index.
//
// LIMITATION: an overshoot that reaches `lens[384]` corrupts the saved frame
// pointer and then the return address, so the C dies with SIGSEGV somewhere
// after `cp_dynamic` returns.  That is not reproducible here; see the note at
// the bottom of `ERRORS.md`.
// ---------------------------------------------------------------------------

const DF_S: usize = 0x008;
const DF_LENS: usize = 0x010;
const DF_LENLENS: usize = 0x150;
const DF_SYM: usize = 0x16c;
const DF_NLEN: usize = 0x170;
const DF_NDST: usize = 0x174;
const DF_NLIT: usize = 0x178;
const DF_I18: usize = 0x17c;
const DF_I17: usize = 0x180;
const DF_I16: usize = 0x184;
const DF_N: usize = 0x188;
const DF_I: usize = 0x18c;
/// One past the largest `lens` index the loop can reach
/// (`nlit + ndst - 1 + 138 = 456`), rounded up.
const DF_SIZE: usize = 0x400;

struct DynFrame {
    b: [u8; DF_SIZE],
}

impl DynFrame {
    fn new() -> DynFrame {
        // The C leaves the frame uninitialised apart from `lenlens = {0}`, but
        // the only byte it ever reads without writing first is `lens[-1]`
        // (see `DF_S`), so zeroing is faithful.
        DynFrame { b: [0u8; DF_SIZE] }
    }

    #[inline]
    fn get_i32(&self, off: usize) -> c_int {
        unsafe { (self.b.as_ptr().add(off) as *const c_int).read_unaligned() }
    }

    #[inline]
    fn set_i32(&mut self, off: usize, v: c_int) {
        unsafe { (self.b.as_mut_ptr().add(off) as *mut c_int).write_unaligned(v) }
    }

    /// Byte offset of `lens[i]`, clamped so a runaway index cannot become
    /// Rust-level UB.  Clamping is unreachable for every input the C can
    /// survive: the bytes the overshoot writes are code lengths (`<= 18`), which
    /// only ever shrink `nlit`/`ndst`, so `n` stays below `457`.
    #[inline]
    fn lens_off(i: c_int) -> usize {
        let off = DF_LENS as isize + i as isize;
        if off < 0 {
            0
        } else if (off as usize) >= DF_SIZE {
            DF_SIZE - 1
        } else {
            off as usize
        }
    }

    #[inline]
    fn lens_get(&self, i: c_int) -> u8 {
        self.b[Self::lens_off(i)]
    }

    #[inline]
    fn lens_set(&mut self, i: c_int, v: u8) {
        let o = Self::lens_off(i);
        self.b[o] = v;
    }

    #[inline]
    fn lens_ptr(&mut self) -> *mut u8 {
        unsafe { self.b.as_mut_ptr().add(DF_LENS) }
    }
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    let mut fr = DynFrame::new();
    // the spilled `s` argument; `lens[-1]` is its most significant byte
    (fr.b.as_mut_ptr().add(DF_S) as *mut u64).write_unaligned(s as u64);

    // `uint8_t lenlens[19] = {0};` — already zero

    let nlit: c_int = 257i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    fr.set_i32(DF_NLIT, nlit);
    let ndst: c_int = 1i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    fr.set_i32(DF_NDST, ndst);
    let nlen: c_int = 4i32.wrapping_add(cp_read_bits(s, 4) as c_int);
    fr.set_i32(DF_NLEN, nlen);

    let perm = (&raw mut cp_permutation_order) as *mut u8;
    fr.set_i32(DF_I, 0);
    // `for (int i = 0; i < nlen; ++i)
    //      lenlens[cp_permutation_order[i]] = (uint8_t)cp_read_bits(s, 3);`
    //
    // gcc calls `cp_read_bits` first, then reloads `i` for the index, then
    // stores, then does a read-modify-write `++i`.  Every access to `i` and
    // `nlen` therefore has to go through the frame: `cp_permutation_order` is an
    // exported writable global with no range check on its entries, so an entry
    // `> 18` makes the store land on `cp_dynamic`'s own locals — including `i`
    // itself, which then continues from the value that was just stored.
    while fr.get_i32(DF_I) < fr.get_i32(DF_NLEN) {
        let v = cp_read_bits(s, 3) as u8;
        let i = fr.get_i32(DF_I);
        let slot = *perm.wrapping_offset(i as isize) as usize;
        let off = DF_LENLENS + slot;
        if off < DF_SIZE {
            fr.b[off] = v;
        }
        fr.set_i32(DF_I, fr.get_i32(DF_I).wrapping_add(1));
    }
    (*s).nlen = cp_build(
        ptr::null_mut(),
        (&raw mut (*s).len) as *mut u32,
        unsafe { fr.b.as_ptr().add(DF_LENLENS) },
        19,
    ) as u32;

    fr.set_i32(DF_N, 0);
    while fr.get_i32(DF_N) < fr.get_i32(DF_NLIT).wrapping_add(fr.get_i32(DF_NDST)) {
        let sym = cp_decode(s, (&raw mut (*s).len) as *mut u32, (*s).nlen as c_int);
        fr.set_i32(DF_SYM, sym);
        match fr.get_i32(DF_SYM) {
            16 => {
                // `for (int i = 3 + cp_read_bits(s, 2); i; --i, ++n)
                //      lens[n] = lens[n - 1];`
                fr.set_i32(DF_I16, 3i32.wrapping_add(cp_read_bits(s, 2) as c_int));
                while fr.get_i32(DF_I16) != 0 {
                    let n = fr.get_i32(DF_N);
                    let v = fr.lens_get(n.wrapping_sub(1));
                    let n2 = fr.get_i32(DF_N); // the C reloads `n` for the store
                    fr.lens_set(n2, v);
                    fr.set_i32(DF_I16, fr.get_i32(DF_I16).wrapping_sub(1));
                    fr.set_i32(DF_N, fr.get_i32(DF_N).wrapping_add(1));
                }
            }
            17 => {
                // `for (int i = 3 + cp_read_bits(s, 3); i; --i, ++n) lens[n] = 0;`
                fr.set_i32(DF_I17, 3i32.wrapping_add(cp_read_bits(s, 3) as c_int));
                while fr.get_i32(DF_I17) != 0 {
                    let n = fr.get_i32(DF_N);
                    fr.lens_set(n, 0);
                    fr.set_i32(DF_I17, fr.get_i32(DF_I17).wrapping_sub(1));
                    fr.set_i32(DF_N, fr.get_i32(DF_N).wrapping_add(1));
                }
            }
            18 => {
                // `for (int i = 11 + cp_read_bits(s, 7); i; --i, ++n) lens[n] = 0;`
                fr.set_i32(DF_I18, 11i32.wrapping_add(cp_read_bits(s, 7) as c_int));
                while fr.get_i32(DF_I18) != 0 {
                    let n = fr.get_i32(DF_N);
                    fr.lens_set(n, 0);
                    fr.set_i32(DF_I18, fr.get_i32(DF_I18).wrapping_sub(1));
                    fr.set_i32(DF_N, fr.get_i32(DF_N).wrapping_add(1));
                }
            }
            _ => {
                // `lens[n++] = (uint8_t)sym;`  gcc bumps `n` *before* storing,
                // which matters when the store lands on `n` itself.
                let n = fr.get_i32(DF_N);
                fr.set_i32(DF_N, n.wrapping_add(1));
                let v = fr.get_i32(DF_SYM) as u8;
                fr.lens_set(n, v);
            }
        }
    }

    let nlit_v = fr.get_i32(DF_NLIT);
    let ndst_v = fr.get_i32(DF_NDST);
    // Keep `cp_build`'s reads inside the emulated frame.  Unreachable for any
    // input the C survives (see `lens_off`), so this never alters behaviour.
    let build_nlit = if nlit_v < 0 {
        0
    } else if nlit_v > 320 {
        320
    } else {
        nlit_v
    };
    let build_ndst = if ndst_v < 0 {
        0
    } else if ndst_v > 320 - build_nlit {
        320 - build_nlit
    } else {
        ndst_v
    };
    let lens = fr.lens_ptr();
    (*s).nlit = cp_build(s, (&raw mut (*s).lit) as *mut u32, lens, build_nlit) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (&raw mut (*s).dst) as *mut u32,
        lens.wrapping_offset(build_nlit as isize),
        build_ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    loop {
        let mut symbol = cp_decode(s, (&raw mut (*s).lit) as *mut u32, (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.wrapping_add(1) <= (*s).out_end) {
                cp_error_reason = ERR_OUT_SYMBOL.as_ptr() as *const c_char;
                return 0;
            }
            *(*s).out = symbol as u8;
            (*s).out = (*s).out.wrapping_add(1);
        } else if symbol > 256 {
            symbol = symbol.wrapping_sub(257);
            let length = cp_read_bits(s, cp_len_extra_at(symbol) as c_int)
                .wrapping_add(cp_len_base_at(symbol)) as c_int;
            let distance_symbol = cp_decode(s, (&raw mut (*s).dst) as *mut u32, (*s).ndst as c_int);
            let backwards_distance = cp_read_bits(s, cp_dist_extra_at(distance_symbol) as c_int)
                .wrapping_add(cp_dist_base_at(distance_symbol))
                as c_int;
            if !((*s).out.wrapping_offset(-(backwards_distance as isize)) >= (*s).begin) {
                cp_error_reason = ERR_BAD_DISTANCE.as_ptr() as *const c_char;
                return 0;
            }
            if !((*s).out.wrapping_offset(length as isize) <= (*s).out_end) {
                cp_error_reason = ERR_OUT_STRING.as_ptr() as *const c_char;
                return 0;
            }
            let mut src = (*s).out.wrapping_offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.wrapping_offset(length as isize);
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst, *src, length as usize);
                }
                _ => {
                    let mut length = length;
                    loop {
                        let test = length;
                        length = length.wrapping_sub(1);
                        if test == 0 {
                            break;
                        }
                        *dst = *src;
                        dst = dst.wrapping_add(1);
                        src = src.wrapping_add(1);
                    }
                }
            }
        } else {
            break;
        }
    }
    1
}

/// `int cp_inflate(void *in, int in_bytes, void *out, int out_bytes)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    r#in: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let layout = std::alloc::Layout::new::<cp_state_t>();
    let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let in_addr = r#in as usize;
    let first_bytes = ((in_addr.wrapping_add(3) & !3usize).wrapping_sub(in_addr)) as c_int;
    (*s).words = (r#in as *mut u8).wrapping_offset(first_bytes as isize) as *mut u32;
    (*s).word_count = in_bytes.wrapping_sub(first_bytes) / 4;
    let last_bytes = in_bytes.wrapping_sub(first_bytes) & 3;
    let mut i: c_int = 0;
    while i < first_bytes {
        (*s).bits |= (*(r#in as *mut u8).wrapping_offset(i as isize) as u64)
            .wrapping_shl((i.wrapping_mul(8)) as u32);
        i += 1;
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    let mut i: c_int = 0;
    while i < last_bytes {
        (*s).final_word |= (*(r#in as *mut u8)
            .wrapping_offset(in_bytes.wrapping_sub(last_bytes).wrapping_add(i) as isize)
            as u32)
            .wrapping_shl((i.wrapping_mul(8)) as u32);
        i += 1;
    }
    (*s).count = first_bytes.wrapping_mul(8);
    (*s).out = out as *mut u8;
    (*s).out_end = (*s).out.wrapping_offset(out_bytes as isize);
    (*s).begin = out as *mut u8;
    let mut count: c_int = 0;
    let mut bfinal: c_int;
    let result: c_int;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
        let mut failed = false;
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    failed = true;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    failed = true;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    failed = true;
                }
            }
            3 => {
                cp_error_reason = ERR_UNKNOWN_BLOCK.as_ptr() as *const c_char;
                failed = true;
            }
            _ => {}
        }
        if failed {
            result = 0;
            break;
        }
        count = count.wrapping_add(1);
        if bfinal != 0 {
            result = 1;
            break;
        }
    }
    let _ = count;
    std::alloc::dealloc(s as *mut u8, layout);
    result
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: c_int = a as c_int + b as c_int - c as c_int;
    let pa: c_int = (p - a as c_int).wrapping_abs();
    let pb: c_int = (p - b as c_int).wrapping_abs();
    let pc: c_int = (p - c as c_int).wrapping_abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// `typedef struct cp_raw_png_t { const uint8_t *p; const uint8_t *end; } cp_raw_png_t;`
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

unsafe fn cp_memcmp4(a: *const u8, b: *const u8) -> bool {
    // memcmp(start + 4, chunk, 4) == 0
    let mut i = 0isize;
    while i < 4 {
        if *a.wrapping_offset(i) != *b.wrapping_offset(i) {
            return false;
        }
        i += 1;
    }
    true
}

unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if cp_memcmp4(start.wrapping_offset(4), chunk as *const u8) && len >= minlen {
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
        (*png).p = (*png).p.wrapping_offset(len.wrapping_add(12) as c_int as isize);
        if cp_memcmp4(start.wrapping_offset(4), chunk as *const u8)
            && len >= minlen
            && (*png).p <= (*png).end
        {
            return start.wrapping_offset(8);
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len: c_int = w.wrapping_mul(bpp);
    let mut raw = raw;
    let prev: *mut u8;
    let mut x: c_int;
    if h > 0 {
        let filter = *raw;
        raw = raw.wrapping_add(1);
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    let v = *raw.wrapping_offset((x - bpp) as isize);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    let v = *raw.wrapping_offset((x - bpp) as isize) / 2;
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let v = cp_paeth(*raw.wrapping_offset((x - bpp) as isize), 0, 0);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x += 1;
                }
            }
            _ => return 0,
        }
    }
    prev = raw;
    let mut prev = prev;
    raw = raw.wrapping_offset(len as isize);
    let mut y: c_int = 1;
    while y < h {
        let filter = *raw;
        raw = raw.wrapping_add(1);
        match filter {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(0);
                    x += 1;
                }
                while x < len {
                    let v = *raw.wrapping_offset((x - bpp) as isize);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    let v = *prev.wrapping_offset(x as isize);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let v = *prev.wrapping_offset(x as isize);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    let v = *prev.wrapping_offset(x as isize) / 2;
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let a = *raw.wrapping_offset((x - bpp) as isize) as c_int;
                    let b = *prev.wrapping_offset(x as isize) as c_int;
                    let v = ((a + b) / 2) as u8;
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    let v = *prev.wrapping_offset(x as isize);
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let v = cp_paeth(
                        *raw.wrapping_offset((x - bpp) as isize),
                        *prev.wrapping_offset(x as isize),
                        *prev.wrapping_offset((x - bpp) as isize),
                    );
                    let t = raw.wrapping_offset(x as isize);
                    *t = (*t).wrapping_add(v);
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

/// `void convert_pix(int bpp, int w, int h, uint8_t *src, cp_pixel_t *dst)`
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
        src = src.wrapping_add(1);
        let mut x: c_int = 0;
        while x < w {
            match bpp {
                1 => {
                    *dst = cp_make_pixel(
                        *src.wrapping_offset(0),
                        *src.wrapping_offset(0),
                        *src.wrapping_offset(0),
                    );
                    dst = dst.wrapping_add(1);
                }
                2 => {
                    *dst = cp_make_pixel_a(
                        *src.wrapping_offset(0),
                        *src.wrapping_offset(0),
                        *src.wrapping_offset(0),
                        *src.wrapping_offset(1),
                    );
                    dst = dst.wrapping_add(1);
                }
                3 => {
                    *dst = cp_make_pixel(
                        *src.wrapping_offset(0),
                        *src.wrapping_offset(1),
                        *src.wrapping_offset(2),
                    );
                    dst = dst.wrapping_add(1);
                }
                4 => {
                    *dst = cp_make_pixel_a(
                        *src.wrapping_offset(0),
                        *src.wrapping_offset(1),
                        *src.wrapping_offset(2),
                        *src.wrapping_offset(3),
                    );
                    dst = dst.wrapping_add(1);
                }
                _ => {}
            }
            x = x.wrapping_add(1);
            src = src.wrapping_offset(bpp as isize);
        }
        y = y.wrapping_add(1);
    }
}
