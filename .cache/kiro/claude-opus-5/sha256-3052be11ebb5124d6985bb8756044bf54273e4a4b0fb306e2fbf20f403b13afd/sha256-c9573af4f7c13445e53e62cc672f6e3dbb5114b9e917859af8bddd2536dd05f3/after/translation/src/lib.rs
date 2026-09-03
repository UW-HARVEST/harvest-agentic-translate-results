//! Rust translation of c_src/src/lib.c (a DEFLATE / "pinflate" decompressor).
//!
//! The translation is deliberately literal: control flow, order of validation
//! checks, integer widths, wrap-around behaviour and even the original code's
//! quirks are reproduced exactly.  `assert()` calls from the C source are not
//! reproduced (the C library is built as a shared object with `NDEBUG` in
//! release configuration, where they compile away).

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(static_mut_refs)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr::{addr_of, addr_of_mut};

// ---------------------------------------------------------------------------
// Exported globals (all non-`static` objects of the C translation unit)
//
// LAYOUT MATTERS.  `cp_block` indexes `cp_len_extra_bits[symbol]`,
// `cp_len_base[symbol]`, `cp_dist_extra_bits[sym]` and `cp_dist_base[sym]` with
// values that come out of `cp_decode`.  A corrupted Huffman tree (reachable
// because `cp_decode` reads `tree[lo - 1]`, and because the tables themselves
// are writable exports) makes those indices exceed the array bounds, so the C
// reads whatever *follows* the array in its `.data` section.  Six independent
// Rust `static`s are laid out in an arbitrary order by the linker, which makes
// those reads return different bytes and the outputs diverge.
//
// The tables are therefore emitted as ONE assembly blob reproducing the exact
// relative offsets of the reference `.so` (measured with `nm -D`):
//
//     +  0   cp_fixed_table        (320 bytes)
//     +320   cp_permutation_order  (19 bytes)  + 13 bytes of zero padding
//     +352   cp_len_extra_bits     (31 bytes)  +  1 byte  of zero padding
//     +384   cp_len_base           (124 bytes) +  4 bytes of zero padding
//     +512   cp_dist_extra_bits    (32 bytes)
//     +544   cp_dist_base          (128 bytes) +  8 bytes of zero padding
//     +680   cp_error_reason       (8 bytes)
//     +688   (zero tail; in the C this is where its `.bss`/ELF neighbourhood
//             starts, so out-of-range reads beyond +688 are approximated by
//             zeros -- see VERIFICATION.md)
// ---------------------------------------------------------------------------

/// `uint8_t cp_fixed_table[288 + 32]` -- 144 x 8, 112 x 9, 24 x 7, 8 x 8, 32 x 5,
/// exactly as spelled out in the C source literal.
#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = {
    let mut t = [0u8; 288 + 32];
    let mut i = 0usize;
    while i < 144 { t[i] = 8; i += 1; }
    while i < 256 { t[i] = 9; i += 1; }
    while i < 280 { t[i] = 7; i += 1; }
    while i < 288 { t[i] = 8; i += 1; }
    while i < 320 { t[i] = 5; i += 1; }
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

/// `const char *cp_error_reason;`
#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = std::ptr::null();

// ---------------------------------------------------------------------------
// Shadow copy of the six tables, laid out exactly like the reference `.so`
//
// `cp_block` indexes `cp_len_extra_bits[symbol]`, `cp_len_base[symbol]`,
// `cp_dist_extra_bits[sym]` and `cp_dist_base[sym]` with values produced by
// `cp_decode`.  Those indices are only guaranteed in range while the Huffman
// tree is well formed; `cp_decode` reads `tree[lo - 1]` (out of bounds by one
// when `lo == 0`) and the tables are writable exports, so a symbol up to 4095
// is reachable, and the C then reads whatever FOLLOWS the array in its `.data`
// section.  Six independent Rust statics are placed in an arbitrary order by
// the linker, so those reads would return different bytes and the outputs would
// diverge.
//
// All table reads therefore go through `CP_SHADOW`, whose field offsets
// reproduce the reference `.so` byte for byte (verified by
// `tests/layout_parity.rs`):
//
//     +  0   cp_fixed_table        320 B
//     +320   cp_permutation_order   19 B  (+13 B zero padding)
//     +352   cp_len_extra_bits      31 B  (+ 1 B zero padding)
//     +384   cp_len_base           124 B  (+ 4 B zero padding)
//     +512   cp_dist_extra_bits     32 B
//     +544   cp_dist_base          128 B  (+ 8 B zero padding)
//     +680   cp_error_reason         8 B  (NULL at every entry to `pinflate`)
//     +688   zero tail, large enough that even index 4095 stays inside it
//
// The shadow is refreshed from the exported (writable) tables on every entry to
// `pinflate`, so a consumer that pokes an export still gets the C's behaviour.
// ---------------------------------------------------------------------------

/// Largest index reachable is 4095 (`(key >> 4) & 0xFFF`), i.e. offset
/// 544 + 4*4095 = 16924 for `cp_dist_base`; the tail is sized well past that.
const CP_TAIL: usize = 32768;

#[repr(C, align(64))]
struct CpTables {
    fixed_table: [u8; 320],
    permutation_order: [u8; 19],
    _pad_a: [u8; 13],
    len_extra_bits: [u8; 31],
    _pad_b: [u8; 1],
    len_base: [u32; 31],
    _pad_c: [u8; 4],
    dist_extra_bits: [u8; 32],
    dist_base: [u32; 32],
    _pad_d: [u8; 8],
    error_reason_slot: u64,
    tail: [u8; CP_TAIL],
}

static mut CP_SHADOW: CpTables = CpTables {
    fixed_table: [0; 320],
    permutation_order: [0; 19],
    _pad_a: [0; 13],
    len_extra_bits: [0; 31],
    _pad_b: [0; 1],
    len_base: [0; 31],
    _pad_c: [0; 4],
    dist_extra_bits: [0; 32],
    dist_base: [0; 32],
    _pad_d: [0; 8],
    error_reason_slot: 0,
    tail: [0; CP_TAIL],
};

/// Byte offsets of each table inside `CpTables`, checked at compile time
/// against the reference `.so`'s measured layout.
const _: () = {
    assert!(std::mem::offset_of!(CpTables, fixed_table) == 0);
    assert!(std::mem::offset_of!(CpTables, permutation_order) == 320);
    assert!(std::mem::offset_of!(CpTables, len_extra_bits) == 352);
    assert!(std::mem::offset_of!(CpTables, len_base) == 384);
    assert!(std::mem::offset_of!(CpTables, dist_extra_bits) == 512);
    assert!(std::mem::offset_of!(CpTables, dist_base) == 544);
    assert!(std::mem::offset_of!(CpTables, error_reason_slot) == 680);
    assert!(std::mem::offset_of!(CpTables, tail) == 688);
};

/// Refresh the shadow from the (writable) exported tables.  The C reads the
/// live objects, so this must run at every entry to `pinflate`.
#[inline]
unsafe fn cp_sync_tables() {
    let s = addr_of_mut!(CP_SHADOW);
    std::ptr::copy_nonoverlapping(
        addr_of!(cp_fixed_table) as *const u8,
        addr_of_mut!((*s).fixed_table) as *mut u8,
        320,
    );
    std::ptr::copy_nonoverlapping(
        addr_of!(cp_permutation_order) as *const u8,
        addr_of_mut!((*s).permutation_order) as *mut u8,
        19,
    );
    std::ptr::copy_nonoverlapping(
        addr_of!(cp_len_extra_bits) as *const u8,
        addr_of_mut!((*s).len_extra_bits) as *mut u8,
        31,
    );
    std::ptr::copy_nonoverlapping(
        addr_of!(cp_len_base) as *const u8,
        addr_of_mut!((*s).len_base) as *mut u8,
        124,
    );
    std::ptr::copy_nonoverlapping(
        addr_of!(cp_dist_extra_bits) as *const u8,
        addr_of_mut!((*s).dist_extra_bits) as *mut u8,
        32,
    );
    std::ptr::copy_nonoverlapping(
        addr_of!(cp_dist_base) as *const u8,
        addr_of_mut!((*s).dist_base) as *mut u8,
        128,
    );
    // The C reads `cp_error_reason`'s bytes at offset +680 when an index runs
    // that far; it is NULL on entry to every call that has not yet failed.
    (*s).error_reason_slot = 0;
}

#[inline]
unsafe fn shadow_fixed_table() -> *const u8 {
    addr_of!((*addr_of!(CP_SHADOW)).fixed_table) as *const u8
}
#[inline]
unsafe fn shadow_perm() -> *const u8 {
    addr_of!((*addr_of!(CP_SHADOW)).permutation_order) as *const u8
}
#[inline]
unsafe fn shadow_len_extra() -> *const u8 {
    addr_of!((*addr_of!(CP_SHADOW)).len_extra_bits) as *const u8
}
#[inline]
unsafe fn shadow_len_base() -> *const u32 {
    addr_of!((*addr_of!(CP_SHADOW)).len_base) as *const u32
}
#[inline]
unsafe fn shadow_dist_extra() -> *const u8 {
    addr_of!((*addr_of!(CP_SHADOW)).dist_extra_bits) as *const u8
}
#[inline]
unsafe fn shadow_dist_base() -> *const u32 {
    addr_of!((*addr_of!(CP_SHADOW)).dist_base) as *const u32
}

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

// ---------------------------------------------------------------------------
// assert() replication
//
// `c_src/CMakeLists.txt` sets no CMAKE_BUILD_TYPE and no -DNDEBUG, so the ten
// `assert()`s in lib.c are LIVE in the reference shared object (`nm -D` lists
// `U __assert_fail@GLIBC_2.2.5`).  Malformed input therefore makes the C library
// die with SIGABRT instead of returning.  Reproducing that is required for
// behavioural identity, and it is free on the valid paths: whenever the C does
// not abort the predicate holds, so the check is a no-op.
// ---------------------------------------------------------------------------

/// Equivalent of glibc's `__assert_fail`: diagnose on stderr, then `abort()`
/// (SIGABRT), exactly like the C library.
#[cold]
#[inline(never)]
fn cp_assert_fail(line: u32, func: &str, expr: &str) -> ! {
    use std::io::Write;
    let mut err = std::io::stderr();
    let _ = writeln!(
        err,
        "translation: c_src/src/lib.c:{}: {}: Assertion `{}' failed.",
        line, func, expr
    );
    let _ = err.flush();
    std::process::abort()
}

macro_rules! cp_assert {
    ($cond:expr, $line:expr, $func:expr, $expr:expr) => {
        if !($cond) {
            cp_assert_fail($line, $func, $expr);
        }
    };
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

// ---------------------------------------------------------------------------
// Bit reader
// ---------------------------------------------------------------------------

#[inline]
fn cp_would_overflow(s: &cp_state_t, num_bits: c_int) -> c_int {
    ((s.bits_left.wrapping_add(s.count)).wrapping_sub(num_bits) < 0) as c_int
}

#[inline]
unsafe fn cp_ptr(s: &cp_state_t) -> *mut c_char {
    cp_assert!((s.bits_left & 7) == 0, 95, "cp_ptr", "!(s->bits_left & 7)");
    // (char *)(s->words + s->word_index) - (s->count / 8)
    (s.words.offset(s.word_index as isize) as *mut c_char)
        .offset(-((s.count / 8) as isize))
}

unsafe fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = *s.words.offset(s.word_index as isize);
            s.word_index = s.word_index.wrapping_add(1);
            s.bits |= (word as u64).wrapping_shl(s.count as u32);
            s.count = s.count.wrapping_add(32);
            cp_assert!(
                s.word_index <= s.word_count,
                104,
                "cp_peak_bits",
                "s->word_index <= s->word_count"
            );
        } else if s.final_word_available != 0 {
            let word = s.final_word;
            s.bits |= (word as u64).wrapping_shl(s.count as u32);
            s.count = s.count.wrapping_add(s.bits_left);
            s.final_word_available = 0;
        }
    }
    s.bits
}

#[inline]
fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    cp_assert!(
        s.count >= num_bits_to_read,
        115,
        "cp_consume_bits",
        "s->count >= num_bits_to_read"
    );
    let mask = (1u64.wrapping_shl(num_bits_to_read as u32)).wrapping_sub(1);
    let bits = (s.bits & mask) as u32;
    s.bits = s.bits.wrapping_shr(num_bits_to_read as u32);
    s.count = s.count.wrapping_sub(num_bits_to_read);
    s.bits_left = s.bits_left.wrapping_sub(num_bits_to_read);
    bits
}

unsafe fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    cp_assert!(num_bits_to_read <= 32, 123, "cp_read_bits", "num_bits_to_read <= 32");
    cp_assert!(num_bits_to_read >= 0, 124, "cp_read_bits", "num_bits_to_read >= 0");
    cp_assert!(s.bits_left > 0, 125, "cp_read_bits", "s->bits_left > 0");
    cp_assert!(s.count <= 64, 126, "cp_read_bits", "s->count <= 64");
    cp_assert!(
        cp_would_overflow(s, num_bits_to_read) == 0,
        127,
        "cp_read_bits",
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
        counts[*lens.offset(n as isize) as usize] =
            counts[*lens.offset(n as isize) as usize].wrapping_add(1);
        n += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15usize {
        codes[n] = codes[n - 1].wrapping_add(counts[n - 1]).wrapping_shl(1);
        first[n] = first[n - 1].wrapping_add(counts[n - 1]);
    }
    if !s.is_null() {
        std::ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, 1 << 9);
    }
    for i in 0..sym_count {
        let len = *lens.offset(i as isize) as usize;
        if len != 0 {
            cp_assert!(len < 16, 154, "cp_build", "len < 16");
            let code = codes[len] as u32;
            codes[len] = codes[len].wrapping_add(1);
            let slot = first[len] as u32;
            first[len] = first[len].wrapping_add(1);
            *tree.offset(slot as isize) =
                code.wrapping_shl((32u32).wrapping_sub(len as u32)) | ((i as u32) << 4) | len as u32;
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    (*s).lookup[j] = ((len << 9) | i as usize) as u16;
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

unsafe fn cp_stored(s: &mut cp_state_t) -> c_int {
    cp_read_bits(s, s.count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        set_error(ERR_LEN_NLEN);
        return 0;
    }
    if !(s.bits_left / 8 <= LEN as c_int) {
        set_error(ERR_STORED_BEYOND);
        return 0;
    }
    let p = cp_ptr(s);
    std::ptr::copy_nonoverlapping(p as *const u8, s.out as *mut u8, LEN as usize);
    s.out = s.out.offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: &mut cp_state_t) -> c_int {
    let table = shadow_fixed_table();
    let sp: *mut cp_state_t = s;
    let lit = addr_of_mut!((*sp).lit) as *mut u32;
    let dst = addr_of_mut!((*sp).dst) as *mut u32;
    (*sp).nlit = cp_build(sp, lit, table, 288) as u32;
    (*sp).ndst = cp_build(std::ptr::null_mut(), dst, table.offset(288), 32) as u32;
    1
}

unsafe fn cp_decode(s: &mut cp_state_t, tree: *const u32, hi: c_int) -> c_int {
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
    // assert((search >> len) == (key >> len)) with `uint32_t len = 32 - (key & 0xF)`.
    // When `key & 0xF == 0` the C shifts a uint32_t by 32, which the x86 `shr`
    // instruction performs modulo 32 (i.e. no shift at all); `wrapping_shr`
    // reproduces that exactly.
    let alen: u32 = 32u32.wrapping_sub(key & 0xF);
    cp_assert!(
        search.wrapping_shr(alen) == key.wrapping_shr(alen),
        217,
        "cp_decode",
        "(search >> len) == (key >> len)"
    );
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

// ---------------------------------------------------------------------------
// `cp_dynamic` and its stack frame
//
// `uint8_t lens[288 + 32]` is 320 bytes, but the 16/17/18 run opcodes advance
// `n` without re-checking the loop bound, so `n` can reach
// (nlit + ndst - 1) + 138 = 457.  The C therefore writes up to 137 bytes past
// the array -- straight over the function's own locals.  That is not random:
// gcc -O0 lays the frame out deterministically, and the reference object file
// (`objdump -d lib.c.o`) gives the exact offsets:
//
//     lens      rbp-0x180   ->  frame + 0     (320 bytes)
//     lenlens   rbp-0x040   ->  frame + 320   (19 bytes)   <- lens[320..339]
//     (padding)                 frame + 339
//     sym       rbp-0x024   ->  frame + 348
//     nlen      rbp-0x020   ->  frame + 352   <- lens[352..356]
//     ndst      rbp-0x01c   ->  frame + 356   <- lens[356..360]
//     nlit      rbp-0x018   ->  frame + 360   <- lens[360..364]
//     i (case 18) rbp-0x014 ->  frame + 364
//     i (case 17) rbp-0x010 ->  frame + 368
//     i (case 16) rbp-0x00c ->  frame + 372
//     n         rbp-0x008   ->  frame + 376   <- lens[376..380]
//     i (hclen) rbp-0x004   ->  frame + 380
//     saved rbp rbp+0x000   ->  frame + 384
//     ret addr  rbp+0x008   ->  frame + 392
//
// So an overflowing run rewrites `nlen`, `ndst`, `nlit`, the run counter and
// even `n` itself while the loop is running.  All of those variables are
// therefore kept *inside* a byte frame here and accessed at exactly those
// offsets, in exactly the order the disassembly shows, so the Rust reproduces
// the C's behaviour instead of quietly writing into padding.
//
// The same mechanism covers `lenlens[cp_permutation_order[i]]`: that export is
// writable, the index is a `uint8_t`, and offset 320 + 32 is `nlen`.
// ---------------------------------------------------------------------------

const FR_LENS: usize = 0;
const FR_LENLENS: usize = 320;
const FR_SYM: usize = 348;
const FR_NLEN: usize = 352;
const FR_NDST: usize = 356;
const FR_NLIT: usize = 360;
const FR_I18: usize = 364;
const FR_I17: usize = 368;
const FR_I16: usize = 372;
const FR_N: usize = 376;
const FR_I0: usize = 380;
/// Beyond this the C clobbers the saved `rbp` (384) and the return address
/// (392); `leave; ret` then resumes with a smashed frame, which faults.
const FR_RBP: usize = 384;

/// Bytes reserved below `lens` so that a negative index (only reachable when
/// `n` itself has been clobbered) stays inside the buffer.
const FR_BELOW: usize = 1024;
const FR_ABOVE: usize = 8192;

/// Writing over the saved `rbp` (frame+384..392) or the return address
/// (frame+392..400) is unrecoverable: `leave; ret` then resumes with a bogus
/// frame pointer or jumps to a code-length byte.  Reproduced as a real fault.
/// Stray bytes ABOVE frame+400 land in the caller's frame and are inert.
#[cold]
#[inline(never)]
unsafe fn cp_stack_smash() -> ! {
    std::ptr::write_volatile(std::ptr::null_mut::<u8>(), 0u8);
    std::process::abort()
}

struct Frame {
    b: Box<[u8]>,
}

impl Frame {
    fn new() -> Frame {
        Frame {
            b: vec![0u8; FR_BELOW + FR_ABOVE].into_boxed_slice(),
        }
    }
    #[inline]
    unsafe fn at(&self, off: isize) -> usize {
        let i = FR_BELOW as isize + off;
        if i < 0 || i as usize >= self.b.len() {
            cp_stack_smash();
        }
        i as usize
    }
    #[inline]
    unsafe fn get_i32(&self, off: usize) -> c_int {
        let i = self.at(off as isize);
        let mut v = [0u8; 4];
        v.copy_from_slice(&self.b[i..i + 4]);
        c_int::from_le_bytes(v)
    }
    #[inline]
    unsafe fn set_i32(&mut self, off: usize, v: c_int) {
        let i = self.at(off as isize);
        self.b[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }
    #[inline]
    unsafe fn get_u8(&self, off: isize) -> u8 {
        let i = self.at(off);
        self.b[i]
    }
    #[inline]
    unsafe fn set_u8(&mut self, off: isize, v: u8) {
        if off >= FR_RBP as isize && off < (FR_RBP + 16) as isize {
            // saved rbp (+384..392) or return address (+392..400) destroyed
            cp_stack_smash();
        }
        let i = self.at(off);
        self.b[i] = v;
    }
    /// Raw pointer to `lens + k`, as the C hands to `cp_build`.  `count` is the
    /// number of bytes `cp_build` will read, so a clobbered `nlit`/`ndst` that
    /// would send it off the frame is reported as the stack smash it is.
    #[inline]
    unsafe fn lens_ptr(&self, k: c_int, count: c_int) -> *const u8 {
        let lo = FR_LENS as isize + k as isize;
        let hi = lo + count.max(0) as isize;
        let _ = self.at(lo);
        if hi > (self.b.len() - FR_BELOW) as isize {
            cp_stack_smash();
        }
        self.b.as_ptr().add(self.at(lo))
    }
}

unsafe fn cp_dynamic(s: &mut cp_state_t) -> c_int {
    let mut fr = Frame::new();
    // `uint8_t lenlens[19] = {0};` -- only those 19 bytes are initialised; the
    // rest of the frame is indeterminate in C and zero here (see
    // VERIFICATION.md; the only indeterminate read the C can make is
    // `lens[n - 1]` with `n == 0`).

    fr.set_i32(FR_NLIT, 257i32.wrapping_add(cp_read_bits(s, 5) as c_int));
    fr.set_i32(FR_NDST, 1i32.wrapping_add(cp_read_bits(s, 5) as c_int));
    fr.set_i32(FR_NLEN, 4i32.wrapping_add(cp_read_bits(s, 4) as c_int));

    let order = shadow_perm();
    fr.set_i32(FR_I0, 0);
    loop {
        let i = fr.get_i32(FR_I0);
        if i >= fr.get_i32(FR_NLEN) {
            break;
        }
        let bits = cp_read_bits(s, 3) as u8;
        let idx = *order.offset(i as isize) as usize;
        fr.set_u8((FR_LENLENS + idx) as isize, bits);
        fr.set_i32(FR_I0, fr.get_i32(FR_I0).wrapping_add(1));
    }

    let sp: *mut cp_state_t = s;
    let len_tree = addr_of_mut!((*sp).len) as *mut u32;
    (*sp).nlen = cp_build(
        std::ptr::null_mut(),
        len_tree,
        fr.b.as_ptr().add(FR_BELOW + FR_LENLENS),
        19,
    ) as u32;

    fr.set_i32(FR_N, 0);
    loop {
        let nlit = fr.get_i32(FR_NLIT);
        let ndst = fr.get_i32(FR_NDST);
        if !(fr.get_i32(FR_N) < nlit.wrapping_add(ndst)) {
            break;
        }
        let sym = cp_decode(s, len_tree as *const u32, (*sp).nlen as c_int);
        fr.set_i32(FR_SYM, sym);
        match sym {
            16 => {
                fr.set_i32(FR_I16, 3i32.wrapping_add(cp_read_bits(s, 2) as c_int));
                while fr.get_i32(FR_I16) != 0 {
                    let n = fr.get_i32(FR_N);
                    // `lens[n] = lens[n - 1]` -- with n == 0 the C reads one
                    // byte below the array (indeterminate); zero is used.
                    let v = if n == 0 {
                        0
                    } else {
                        fr.get_u8(FR_LENS as isize + (n - 1) as isize)
                    };
                    fr.set_u8(FR_LENS as isize + n as isize, v);
                    fr.set_i32(FR_I16, fr.get_i32(FR_I16).wrapping_sub(1));
                    fr.set_i32(FR_N, fr.get_i32(FR_N).wrapping_add(1));
                }
            }
            17 => {
                fr.set_i32(FR_I17, 3i32.wrapping_add(cp_read_bits(s, 3) as c_int));
                while fr.get_i32(FR_I17) != 0 {
                    let n = fr.get_i32(FR_N);
                    fr.set_u8(FR_LENS as isize + n as isize, 0);
                    fr.set_i32(FR_I17, fr.get_i32(FR_I17).wrapping_sub(1));
                    fr.set_i32(FR_N, fr.get_i32(FR_N).wrapping_add(1));
                }
            }
            18 => {
                fr.set_i32(FR_I18, 11i32.wrapping_add(cp_read_bits(s, 7) as c_int));
                while fr.get_i32(FR_I18) != 0 {
                    let n = fr.get_i32(FR_N);
                    fr.set_u8(FR_LENS as isize + n as isize, 0);
                    fr.set_i32(FR_I18, fr.get_i32(FR_I18).wrapping_sub(1));
                    fr.set_i32(FR_N, fr.get_i32(FR_N).wrapping_add(1));
                }
            }
            _ => {
                // `lens[n++] = (uint8_t)sym;` -- the disassembly increments `n`
                // BEFORE the store, which matters when the store aliases `n`.
                let old_n = fr.get_i32(FR_N);
                fr.set_i32(FR_N, old_n.wrapping_add(1));
                let v = fr.get_i32(FR_SYM) as u8;
                fr.set_u8(FR_LENS as isize + old_n as isize, v);
            }
        }
    }

    let nlit = fr.get_i32(FR_NLIT);
    let ndst = fr.get_i32(FR_NDST);
    let lit = addr_of_mut!((*sp).lit) as *mut u32;
    let dst = addr_of_mut!((*sp).dst) as *mut u32;
    (*sp).nlit = cp_build(sp, lit, fr.lens_ptr(0, nlit), nlit) as u32;
    (*sp).ndst = cp_build(std::ptr::null_mut(), dst, fr.lens_ptr(nlit, ndst), ndst) as u32;
    1
}

unsafe fn cp_block(s: &mut cp_state_t) -> c_int {
    let sp: *mut cp_state_t = s;
    let lit = addr_of_mut!((*sp).lit) as *const u32;
    let dst_tree = addr_of_mut!((*sp).dst) as *const u32;
    let len_extra = shadow_len_extra();
    let len_base = shadow_len_base();
    let dist_extra = shadow_dist_extra();
    let dist_base = shadow_dist_base();

    loop {
        let mut symbol = cp_decode(s, lit, (*sp).nlit as c_int);
        if symbol < 256 {
            if !(s.out.wrapping_offset(1) <= s.out_end) {
                set_error(ERR_OUT_SYMBOL);
                return 0;
            }
            *s.out = symbol as c_char;
            s.out = s.out.offset(1);
        } else if symbol > 256 {
            symbol = symbol.wrapping_sub(257);
            let length: c_int = (cp_read_bits(s, *len_extra.offset(symbol as isize) as c_int)
                as c_int)
                .wrapping_add(*len_base.offset(symbol as isize) as c_int);
            let distance_symbol = cp_decode(s, dst_tree, (*sp).ndst as c_int);
            let backwards_distance: c_int =
                (cp_read_bits(s, *dist_extra.offset(distance_symbol as isize) as c_int) as c_int)
                    .wrapping_add(*dist_base.offset(distance_symbol as isize) as c_int);
            if !(s.out.wrapping_offset(-(backwards_distance as isize)) >= s.begin) {
                set_error(ERR_BACK_DISTANCE);
                return 0;
            }
            if !(s.out.wrapping_offset(length as isize) <= s.out_end) {
                set_error(ERR_OUT_STRING);
                return 0;
            }
            let mut src = s.out.offset(-(backwards_distance as isize));
            let mut dst = s.out;
            s.out = s.out.offset(length as isize);
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
    cp_sync_tables();
    let mut boxed = cp_state_t::zeroed();
    let s: &mut cp_state_t = &mut boxed;

    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes.wrapping_mul(8);

    let in_addr = in_ as usize;
    let first_bytes: c_int = (((in_addr + 3) & !3usize) - in_addr) as c_int;
    s.words = (in_ as *mut c_char).offset(first_bytes as isize) as *mut u32;
    s.word_count = in_bytes.wrapping_sub(first_bytes) / 4;
    let last_bytes: c_int = in_bytes.wrapping_sub(first_bytes) & 3;

    let in_u8 = in_ as *const u8;
    for i in 0..first_bytes {
        s.bits |= (*in_u8.offset(i as isize) as u64) << (i.wrapping_mul(8) as u32 & 63);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        s.final_word |=
            (*in_u8.offset(in_bytes.wrapping_sub(last_bytes).wrapping_add(i) as isize) as u32)
                << (i * 8);
    }
    s.count = first_bytes.wrapping_mul(8);
    s.out = out as *mut c_char;
    s.out_end = s.out.wrapping_offset(out_bytes as isize);
    s.begin = out as *mut c_char;

    let mut _count: c_int = 0;
    let mut bfinal: u32;
    loop {
        bfinal = cp_read_bits(s, 1);
        let btype = cp_read_bits(s, 2);
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
                set_error(ERR_BLOCK_TYPE);
                return 0;
            }
            _ => {}
        }
        _count = _count.wrapping_add(1);
        if bfinal != 0 {
            break;
        }
    }
    1
}
