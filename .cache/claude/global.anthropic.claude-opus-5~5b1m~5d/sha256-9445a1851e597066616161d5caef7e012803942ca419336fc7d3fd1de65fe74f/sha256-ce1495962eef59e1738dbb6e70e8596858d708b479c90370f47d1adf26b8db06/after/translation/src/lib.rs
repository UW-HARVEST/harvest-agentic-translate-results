//! Rust translation of the C library in `c_src/` (a trimmed derivative of
//! `cute_png.h` by Randy Gaul -- zlib / public-domain licensed, see
//! `c_src/license.txt`).
//!
//! The translation is intentionally *literal*: every observable behaviour of
//! the C code (including its quirks and bugs, evaluation/validation order,
//! integer wrap-around, pointer arithmetic and out-of-bounds accesses) is
//! reproduced so that the exported ABI is byte-for-byte compatible.
//!
//! The C build compiles with `NDEBUG` semantics for the purposes of this
//! translation, i.e. the `assert()` calls sprinkled through the original code
//! are treated as no-ops (they are documented in comments where they appear).
//! Everything else is mirrored exactly, which is why shifts use the wrapping
//! forms (matching what x86-64 code generation does for C's UB cases) and why
//! table lookups go through raw pointers (matching C's unchecked indexing).

#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    unused_assignments,
    unused_variables,
    dead_code
)]

use core::ffi::{CStr, c_char, c_int, c_void};
use core::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn abort() -> !;
}

// ---------------------------------------------------------------------------
// assert()
//
// The reference shared library is built by `c_src/CMakeLists.txt` with no
// `CMAKE_BUILD_TYPE`, i.e. `-O0` and *without* `-DNDEBUG`, so every `assert()`
// in `c_src/src/lib.c` is live and a failing one calls `__assert_fail`, which
// prints to stderr and raises `SIGABRT`. `c_assert!` reproduces the
// process-level effect (`abort()` -> `SIGABRT`).
// ---------------------------------------------------------------------------

#[cold]
#[inline(never)]
fn cp_assert_fail() -> ! {
    unsafe { abort() }
}

macro_rules! c_assert {
    ($cond:expr) => {
        if !($cond) {
            cp_assert_fail();
        }
    };
}

/// Reproduces a C stack smash that clobbers the saved `rbp` / return address:
/// the C then `ret`s to a zeroed address (or runs its caller with a garbage
/// frame pointer) and dies with `SIGSEGV`.
#[cold]
#[inline(never)]
fn cp_stack_smash() -> ! {
    unsafe {
        ptr::read_volatile(1usize as *const u8);
    }
    unsafe { abort() }
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
// Public (exported) data objects
// ---------------------------------------------------------------------------

/// `const char *cp_error_reason;`
#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[inline]
fn set_error_reason(msg: &'static CStr) {
    unsafe {
        *(&raw mut cp_error_reason) = msg.as_ptr();
    }
}

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

/// `uint8_t cp_fixed_table[288 + 32]` -- 144x8, 112x9, 24x7, 8x8, then 32x5.
#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = cp_make_fixed_table();

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
// Layout-faithful table reads
//
// `cp_block` indexes `cp_len_extra_bits`/`cp_len_base` with `symbol - 257` and
// `cp_dist_extra_bits`/`cp_dist_base` with a distance symbol. A corrupt Huffman
// tree makes `cp_decode` return values far outside those tables' bounds (up to
// 4095), so the C reads *past* the end of a table and gets whatever the linker
// placed next.
//
// In the reference shared library the six tables are the whole of `.data`:
// 0x2a0 = 672 bytes at 0x6060, in *source* order, each table 32-byte aligned,
// gap bytes zero (verified with `objdump -s -j .data`, see `SYMBOLS.md`):
//
//     rel   0  cp_fixed_table        320 B
//     rel 320  cp_permutation_order   19 B  + 13 pad
//     rel 352  cp_len_extra_bits      31 B  +  1 pad
//     rel 384  cp_len_base           124 B  +  4 pad
//     rel 512  cp_dist_extra_bits     32 B
//     rel 544  cp_dist_base          128 B   -> ends at rel 672 (= .bss)
//
// `.bss` immediately follows: `completed.0` (8 B, rel 672..680, zero until
// `__cxa_finalize` runs) then `cp_error_reason` (8 B, rel 680..688). The RW
// LOAD segment ends at 0x6310 and is page-rounded to 0x7000, so rel 688..4000
// reads as zero and rel >= 4000 faults.
//
// Rust/LLVM order and align statics differently and that order cannot be
// controlled portably, so out-of-range indices are resolved through this model
// instead of walking off the end of a Rust static. The model reads the *live*
// statics, so a caller that mutates a table still affects out-of-range reads
// just as in C.
// ---------------------------------------------------------------------------

const OFF_FIXED_TABLE: usize = 0; // 320 bytes
const OFF_PERMUTATION_ORDER: usize = 320; // 19 bytes, then 13 gap bytes
const OFF_LEN_EXTRA_BITS: usize = 352; // 31 bytes, then 1 gap byte
const OFF_LEN_BASE: usize = 384; // 124 bytes, then 4 gap bytes
const OFF_DIST_EXTRA_BITS: usize = 512; // 32 bytes
const OFF_DIST_BASE: usize = 544; // 128 bytes
const DATA_BLOB_LEN: usize = 672;
/// End of the modelled window: `.data` + the two `.bss` objects behind it.
const MODEL_LEN: usize = 688;

/// Reads byte `off` of the reference library's `.data` (+ trailing `.bss`) blob.
unsafe fn blob_byte(off: usize) -> u8 {
    unsafe {
        let (base, rel): (*const u8, usize) = if off < OFF_FIXED_TABLE + 320 {
            (
                (&raw const cp_fixed_table) as *const u8,
                off - OFF_FIXED_TABLE,
            )
        } else if off < OFF_PERMUTATION_ORDER + 19 {
            (
                (&raw const cp_permutation_order) as *const u8,
                off - OFF_PERMUTATION_ORDER,
            )
        } else if off < OFF_LEN_EXTRA_BITS {
            return 0; // 13 gap bytes
        } else if off < OFF_LEN_EXTRA_BITS + 31 {
            (
                (&raw const cp_len_extra_bits) as *const u8,
                off - OFF_LEN_EXTRA_BITS,
            )
        } else if off < OFF_LEN_BASE {
            return 0; // 1 gap byte
        } else if off < OFF_LEN_BASE + 124 {
            ((&raw const cp_len_base) as *const u8, off - OFF_LEN_BASE)
        } else if off < OFF_DIST_EXTRA_BITS {
            return 0; // 4 gap bytes
        } else if off < OFF_DIST_EXTRA_BITS + 32 {
            (
                (&raw const cp_dist_extra_bits) as *const u8,
                off - OFF_DIST_EXTRA_BITS,
            )
        } else if off < DATA_BLOB_LEN {
            ((&raw const cp_dist_base) as *const u8, off - OFF_DIST_BASE)
        } else if off < 680 {
            return 0; // .bss `completed.0`
        } else if off < MODEL_LEN {
            // .bss `cp_error_reason` -- a runtime pointer value. The C reads
            // its own copy here; the two libraries necessarily disagree on the
            // *value*, but the structure is reproduced.
            (
                (&raw const cp_error_reason) as *const u8,
                off - 680,
            )
        } else {
            // rel 688..4000 is the zero-filled tail of the RW page in the
            // reference mapping; rel >= 4000 faults there (not reproduced).
            return 0;
        };
        *base.add(rel)
    }
}

/// `table[index]` for a `uint8_t` table at blob offset `table_off`.
#[inline]
unsafe fn at_u8(table_off: usize, index: c_int) -> u8 {
    unsafe {
        let off = table_off.wrapping_add(index as usize);
        if off >= MODEL_LEN {
            return 0;
        }
        blob_byte(off)
    }
}

/// `table[index]` for a `uint32_t` table at blob offset `table_off`.
#[inline]
unsafe fn at_u32(table_off: usize, index: c_int) -> u32 {
    unsafe {
        let off = table_off.wrapping_add((index as usize).wrapping_mul(4));
        if off >= MODEL_LEN {
            return 0;
        }
        // a read straddling the end of the blob keeps the in-blob bytes
        u32::from_le_bytes([
            blob_byte(off),
            blob_byte(off + 1),
            blob_byte(off + 2),
            blob_byte(off + 3),
        ])
    }
}

// ---------------------------------------------------------------------------
// DEFLATE decompressor
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

/// `static int cp_would_overflow(cp_state_t *s, int num_bits)`
///
/// Only ever used by an `assert()` in the original source.
unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    unsafe { (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int }
}

/// `static char *cp_ptr(cp_state_t *s)`
unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    unsafe {
        c_assert!(((*s).bits_left & 7) == 0);
        let words_at = (*s).words.wrapping_offset((*s).word_index as isize) as *mut c_char;
        words_at.wrapping_offset(-(((*s).count / 8) as isize))
    }
}

/// `static uint64_t cp_peak_bits(cp_state_t *s, int num_bits_to_read)`
unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    unsafe {
        if (*s).count < num_bits_to_read {
            if (*s).word_index < (*s).word_count {
                let word = ptr::read_unaligned((*s).words.wrapping_offset((*s).word_index as isize));
                (*s).word_index += 1;
                (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
                (*s).count += 32;
                c_assert!((*s).word_index <= (*s).word_count);
            } else if (*s).final_word_available != 0 {
                let word = (*s).final_word;
                (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
                (*s).count += (*s).bits_left;
                (*s).final_word_available = 0;
            }
        }
        (*s).bits
    }
}

/// `static uint32_t cp_consume_bits(cp_state_t *s, int num_bits_to_read)`
unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    unsafe {
        c_assert!((*s).count >= num_bits_to_read);
        let mask = 1u64.wrapping_shl(num_bits_to_read as u32).wrapping_sub(1);
        let bits = ((*s).bits & mask) as u32;
        (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
        (*s).count -= num_bits_to_read;
        (*s).bits_left -= num_bits_to_read;
        bits
    }
}

/// `static uint32_t cp_read_bits(cp_state_t *s, int num_bits_to_read)`
unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    unsafe {
        c_assert!(num_bits_to_read <= 32);
        c_assert!(num_bits_to_read >= 0);
        c_assert!((*s).bits_left > 0);
        c_assert!((*s).count <= 64);
        c_assert!(cp_would_overflow(s, num_bits_to_read) == 0);
        cp_peak_bits(s, num_bits_to_read);
        cp_consume_bits(s, num_bits_to_read)
    }
}

/// `static uint32_t cp_rev16(uint32_t a)`
fn cp_rev16(a: u32) -> u32 {
    let mut a = a;
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

/// `static int cp_build(cp_state_t *s, uint32_t *tree, uint8_t *lens, int sym_count)`
///
/// `codes`/`first`/`counts` are 256 entries instead of C's 16 so that a bogus
/// (`> 15`) code length -- only reachable if a caller mutates the public
/// `cp_fixed_table` -- stays memory safe instead of smashing the stack. For
/// every input the C code handles without tripping its own `assert(len < 16)`
/// the two are identical.
unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    unsafe {
        let mut codes = [0i32; 256];
        let mut first = [0i32; 256];
        let mut counts = [0i32; 256];

        let mut n: c_int = 0;
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
            memset(
                (&raw mut (*s).lookup) as *mut c_void,
                0,
                core::mem::size_of::<[u16; 1 << 9]>(),
            );
        }
        let mut i: c_int = 0;
        while i < sym_count {
            let len = *lens.offset(i as isize) as c_int;
            if len != 0 {
                // The C's `counts[lens[n]]++` in the first loop has already
                // walked off the end of `counts[16]` into `first`/`codes` (and
                // possibly the return address) by the time we get here, but
                // this assert always fires for the very same inputs, so the
                // corrupted values are never observable.
                c_assert!(len < 16);
                let code = codes[len as usize] as u32;
                codes[len as usize] = codes[len as usize].wrapping_add(1);
                let slot = first[len as usize] as u32;
                first[len as usize] = first[len as usize].wrapping_add(1);
                // C indexes with a uint32_t, i.e. a zero-extended offset.
                *tree.offset(slot as isize) = code
                    .wrapping_shl((32 - len) as u32)
                    | ((i as u32) << 4)
                    | (len as u32);
                if !s.is_null() && len <= 9 {
                    let lookup = (&raw mut (*s).lookup) as *mut u16;
                    let mut j = cp_rev16(code).wrapping_shr((16 - len) as u32) as c_int;
                    while j < (1 << 9) {
                        *lookup.offset(j as isize) = ((len << 9) | i) as u16;
                        j = j.wrapping_add(1i32.wrapping_shl(len as u32));
                    }
                }
            }
            i += 1;
        }
        let max_index = first[15];
        max_index
    }
}

/// `static int cp_stored(cp_state_t *s)`
unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    unsafe {
        cp_read_bits(s, (*s).count & 7);
        let LEN = cp_read_bits(s, 16) as u16;
        let NLEN = cp_read_bits(s, 16) as u16;
        if !(LEN == !NLEN) {
            set_error_reason(
                c"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
            );
            return 0;
        }
        if !((*s).bits_left / 8 <= LEN as c_int) {
            set_error_reason(c"Stored block extends beyond end of input stream.");
            return 0;
        }
        let p = cp_ptr(s);
        memcpy(
            (*s).out as *mut c_void,
            p as *const c_void,
            LEN as usize,
        );
        (*s).out = (*s).out.wrapping_offset(LEN as isize);
        1
    }
}

/// `static int cp_fixed(cp_state_t *s)`
unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    unsafe {
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
}

/// `static int cp_decode(cp_state_t *s, uint32_t *tree, int hi)`
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
        // Note: when `hi` starts at 0 the original reads `tree[-1]`, i.e. the
        // struct field preceding the tree. Reproduced verbatim.
        let key = *tree.offset((lo - 1) as isize);
        let len = 32u32.wrapping_sub(key & 0xF);
        // assert((search >> len) == (key >> len));
        // `len` is 32 when `key & 0xF == 0`; gcc emits a 32-bit `shr %cl`, whose
        // count is taken mod 32, so the shift is a no-op in that case.
        c_assert!((search >> (len & 31)) == (key >> (len & 31)));
        let code = cp_consume_bits(s, (key & 0xF) as c_int);
        let _ = code;
        ((key >> 4) & 0xFFF) as c_int
    }
}

// ---------------------------------------------------------------------------
// cp_dynamic's stack frame
//
// `cp_dynamic` writes past the end of its `uint8_t lens[288 + 32]`: the 16/17/18
// run-length symbols are only bounded by their repeat count, so `n` can run up
// to 137 entries beyond `nlit + ndst`. In the reference `.so` (gcc, `-O0`) the
// frame is (`sub rsp, 0x190`, all offsets from `rbp`, taken from
// `objdump -d`):
//
//     rbp-0x188  s          (spilled parameter, 8 B)
//     rbp-0x180  lens[320]
//     rbp-0x40   lenlens[19]           (+ 5 B tail padding)
//     rbp-0x24   sym        rbp-0x20 nlen   rbp-0x1c ndst   rbp-0x18 nlit
//     rbp-0x14   i (case 18)  rbp-0x10 i (case 17)  rbp-0xc i (case 16)
//     rbp-0x8    n          rbp-0x4  i (permutation loop)
//
// so `lens[k]` aliases, for k >= 320: `lenlens` (320..339), padding (339..348),
// `sym` (348..352), `nlen` (352..356), `ndst` (356..360), `nlit` (360..364),
// the three `i`s (364..376), `n` (376..380) and the permutation `i` (380..384).
// Clobbering `n`, `nlit` and `ndst` genuinely changes the decode, so the frame
// is modelled byte-exactly here and every access goes through it, in the same
// order (and with the same reloads-from-memory) as the `-O0` code.
//
// `lens[-1]`, read when symbol 16 arrives at `n == 0`, is the most significant
// byte of the spilled `s` pointer, i.e. 0 for any heap pointer -- so that read
// is well defined rather than indeterminate.
// ---------------------------------------------------------------------------

/// Leading padding, so that a `lens + nlit` pointer with a clobbered (small or
/// negative) `nlit` still lands inside the array.
const FR_PAD: usize = 384;
/// Index of `rbp` inside the frame array.
const FR_RBP: usize = FR_PAD + 0x190;
const FR_LEN: usize = FR_RBP + 0x200;

const I_S: usize = FR_RBP - 0x188;
const I_LENS: usize = FR_RBP - 0x180;
const I_LENLENS: usize = FR_RBP - 0x40;
const I_SYM: usize = FR_RBP - 0x24;
const I_NLEN: usize = FR_RBP - 0x20;
const I_NDST: usize = FR_RBP - 0x1c;
const I_NLIT: usize = FR_RBP - 0x18;
const I_I18: usize = FR_RBP - 0x14;
const I_I17: usize = FR_RBP - 0x10;
const I_I16: usize = FR_RBP - 0xc;
const I_N: usize = FR_RBP - 0x8;
const I_IPERM: usize = FR_RBP - 0x4;

#[inline]
fn fr_g32(fr: &[u8; FR_LEN], i: usize) -> c_int {
    c_int::from_le_bytes([fr[i], fr[i + 1], fr[i + 2], fr[i + 3]])
}

#[inline]
fn fr_s32(fr: &mut [u8; FR_LEN], i: usize, v: c_int) {
    fr[i..i + 4].copy_from_slice(&v.to_le_bytes());
}

#[inline]
fn fr_load(fr: &[u8; FR_LEN], idx: isize) -> u8 {
    if idx >= 0 && (idx as usize) < FR_LEN {
        fr[idx as usize]
    } else {
        0
    }
}

/// A byte store into the frame. Anything at or past `rbp` overwrites the saved
/// frame pointer / return address in the C, which makes it die with `SIGSEGV`
/// on `leave; ret`; that is recorded and replayed when the function returns.
#[inline]
fn fr_store(fr: &mut [u8; FR_LEN], idx: isize, v: u8, smashed: &mut bool) {
    if idx >= FR_RBP as isize {
        *smashed = true;
    }
    if idx >= 0 && (idx as usize) < FR_LEN {
        fr[idx as usize] = v;
    } else {
        *smashed = true;
    }
}

/// `static int cp_dynamic(cp_state_t *s)`
unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    unsafe {
        let mut fr = [0u8; FR_LEN];
        let mut smashed = false;
        fr[I_S..I_S + 8].copy_from_slice(&(s as usize as u64).to_le_bytes());

        // int nlit = 257 + cp_read_bits(s, 5);
        // int ndst =   1 + cp_read_bits(s, 5);
        // int nlen =   4 + cp_read_bits(s, 4);
        let v = 257i32.wrapping_add(cp_read_bits(s, 5) as c_int);
        fr_s32(&mut fr, I_NLIT, v);
        let v = 1i32.wrapping_add(cp_read_bits(s, 5) as c_int);
        fr_s32(&mut fr, I_NDST, v);
        let v = 4i32.wrapping_add(cp_read_bits(s, 4) as c_int);
        fr_s32(&mut fr, I_NLEN, v);

        // for (int i = 0; i < nlen; ++i)
        //   lenlens[cp_permutation_order[i]] = (uint8_t)cp_read_bits(s, 3);
        fr_s32(&mut fr, I_IPERM, 0);
        while fr_g32(&fr, I_IPERM) < fr_g32(&fr, I_NLEN) {
            let val = cp_read_bits(s, 3) as u8;
            // `i` is reloaded from the frame after the call, and `i++` is a
            // read-modify-write that happens *after* the store.
            let i = fr_g32(&fr, I_IPERM);
            let j = at_u8(OFF_PERMUTATION_ORDER, i) as isize;
            fr_store(&mut fr, I_LENLENS as isize + j, val, &mut smashed);
            let i = fr_g32(&fr, I_IPERM);
            fr_s32(&mut fr, I_IPERM, i.wrapping_add(1));
        }

        // s->nlen = cp_build(0, s->len, lenlens, 19);
        (*s).nlen = cp_build(
            ptr::null_mut(),
            (&raw mut (*s).len) as *mut u32,
            fr.as_ptr().add(I_LENLENS),
            19,
        ) as u32;

        // for (int n = 0; n < nlit + ndst;) { ... }
        fr_s32(&mut fr, I_N, 0);
        loop {
            let nlit_c = fr_g32(&fr, I_NLIT);
            let ndst_c = fr_g32(&fr, I_NDST);
            if !(fr_g32(&fr, I_N) < nlit_c.wrapping_add(ndst_c)) {
                break;
            }
            let sym = cp_decode(s, (&raw mut (*s).len) as *mut u32, (*s).nlen as c_int);
            fr_s32(&mut fr, I_SYM, sym);
            match sym {
                16 => {
                    // for (int i = 3 + cp_read_bits(s, 2); i; --i, ++n)
                    //   lens[n] = lens[n - 1];
                    let v = 3i32.wrapping_add(cp_read_bits(s, 2) as c_int);
                    fr_s32(&mut fr, I_I16, v);
                    while fr_g32(&fr, I_I16) != 0 {
                        let n = fr_g32(&fr, I_N);
                        let prev = fr_load(&fr, I_LENS as isize + (n as isize) - 1);
                        let n = fr_g32(&fr, I_N);
                        fr_store(&mut fr, I_LENS as isize + n as isize, prev, &mut smashed);
                        let i = fr_g32(&fr, I_I16);
                        fr_s32(&mut fr, I_I16, i.wrapping_sub(1));
                        let n = fr_g32(&fr, I_N);
                        fr_s32(&mut fr, I_N, n.wrapping_add(1));
                    }
                }
                17 => {
                    // for (int i = 3 + cp_read_bits(s, 3); i; --i, ++n) lens[n] = 0;
                    let v = 3i32.wrapping_add(cp_read_bits(s, 3) as c_int);
                    fr_s32(&mut fr, I_I17, v);
                    while fr_g32(&fr, I_I17) != 0 {
                        let n = fr_g32(&fr, I_N);
                        fr_store(&mut fr, I_LENS as isize + n as isize, 0, &mut smashed);
                        let i = fr_g32(&fr, I_I17);
                        fr_s32(&mut fr, I_I17, i.wrapping_sub(1));
                        let n = fr_g32(&fr, I_N);
                        fr_s32(&mut fr, I_N, n.wrapping_add(1));
                    }
                }
                18 => {
                    // for (int i = 11 + cp_read_bits(s, 7); i; --i, ++n) lens[n] = 0;
                    let v = 11i32.wrapping_add(cp_read_bits(s, 7) as c_int);
                    fr_s32(&mut fr, I_I18, v);
                    while fr_g32(&fr, I_I18) != 0 {
                        let n = fr_g32(&fr, I_N);
                        fr_store(&mut fr, I_LENS as isize + n as isize, 0, &mut smashed);
                        let i = fr_g32(&fr, I_I18);
                        fr_s32(&mut fr, I_I18, i.wrapping_sub(1));
                        let n = fr_g32(&fr, I_N);
                        fr_s32(&mut fr, I_N, n.wrapping_add(1));
                    }
                }
                _ => {
                    // lens[n++] = (uint8_t)sym; -- gcc bumps `n` *before* the store
                    let n = fr_g32(&fr, I_N);
                    fr_s32(&mut fr, I_N, n.wrapping_add(1));
                    let symv = fr_g32(&fr, I_SYM) as u8;
                    fr_store(&mut fr, I_LENS as isize + n as isize, symv, &mut smashed);
                }
            }
        }

        let nlit_f = fr_g32(&fr, I_NLIT);
        let ndst_f = fr_g32(&fr, I_NDST);
        // Only the 16/17/18 cases can write into `nlit`/`ndst` (the default case
        // advances `n` by one per outer iteration and the loop guard caps
        // `n < nlit + ndst <= 320`), and they only ever store zero there, so
        // these can only shrink from their initial 257..288 / 1..32.
        debug_assert!((0..=320).contains(&nlit_f) && (0..=320).contains(&ndst_f));
        let nlit_ok = (0..=320).contains(&nlit_f);
        let ndst_ok = (0..=320).contains(&ndst_f);
        if !nlit_ok || !ndst_ok {
            smashed = true;
        }

        // s->nlit = cp_build(s, s->lit, lens, nlit);
        (*s).nlit = cp_build(
            s,
            (&raw mut (*s).lit) as *mut u32,
            fr.as_ptr().add(I_LENS),
            if nlit_ok { nlit_f } else { 0 },
        ) as u32;
        // s->ndst = cp_build(0, s->dst, lens + nlit, ndst);
        let dst_idx = I_LENS as isize + nlit_f as isize;
        let dst_lens: *const u8 = if nlit_ok {
            fr.as_ptr().offset(dst_idx)
        } else {
            fr.as_ptr()
        };
        (*s).ndst = cp_build(
            ptr::null_mut(),
            (&raw mut (*s).dst) as *mut u32,
            dst_lens,
            if ndst_ok { ndst_f } else { 0 },
        ) as u32;

        if smashed {
            cp_stack_smash();
        }
        1
    }
}

/// `static int cp_block(cp_state_t *s)`
unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    unsafe {
        let len_extra = OFF_LEN_EXTRA_BITS;
        let len_base = OFF_LEN_BASE;
        let dist_extra = OFF_DIST_EXTRA_BITS;
        let dist_base = OFF_DIST_BASE;
        loop {
            let mut symbol = cp_decode(s, (&raw mut (*s).lit) as *mut u32, (*s).nlit as c_int);
            if symbol < 256 {
                if !((*s).out.wrapping_offset(1) <= (*s).out_end) {
                    set_error_reason(
                        c"Attempted to overwrite out buffer while outputting a symbol.",
                    );
                    return 0;
                }
                *(*s).out = symbol as c_char;
                (*s).out = (*s).out.wrapping_offset(1);
            } else if symbol > 256 {
                symbol -= 257;
                let length: c_int = (cp_read_bits(s, at_u8(len_extra, symbol) as c_int)
                    .wrapping_add(at_u32(len_base, symbol))) as c_int;
                let distance_symbol = cp_decode(s, (&raw mut (*s).dst) as *mut u32, (*s).ndst as c_int);
                let backwards_distance: c_int = (cp_read_bits(
                    s,
                    at_u8(dist_extra, distance_symbol) as c_int,
                )
                .wrapping_add(at_u32(dist_base, distance_symbol))) as c_int;
                if !((*s).out.wrapping_offset(-(backwards_distance as isize)) >= (*s).begin) {
                    set_error_reason(
                        c"Attempted to write before out buffer (invalid backwards distance).",
                    );
                    return 0;
                }
                if !((*s).out.wrapping_offset(length as isize) <= (*s).out_end) {
                    set_error_reason(
                        c"Attempted to overwrite out buffer while outputting a string.",
                    );
                    return 0;
                }
                let mut src = (*s).out.wrapping_offset(-(backwards_distance as isize));
                let mut dst = (*s).out;
                (*s).out = (*s).out.wrapping_offset(length as isize);
                let mut length = length;
                match backwards_distance {
                    1 => {
                        memset(dst as *mut c_void, *src as c_int, length as usize);
                    }
                    _ => {
                        while length != 0 {
                            *dst = *src;
                            dst = dst.wrapping_offset(1);
                            src = src.wrapping_offset(1);
                            length -= 1;
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

/// `int cp_inflate(void *in, int in_bytes, void *out, int out_bytes)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    in_: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        let s = calloc(1, core::mem::size_of::<cp_state_t>()) as *mut cp_state_t;
        (*s).bits = 0;
        (*s).count = 0;
        (*s).word_index = 0;
        (*s).bits_left = in_bytes.wrapping_mul(8);
        let first_bytes =
            (((in_ as usize).wrapping_add(3) & !3usize).wrapping_sub(in_ as usize)) as c_int;
        (*s).words = (in_ as *mut c_char).wrapping_offset(first_bytes as isize) as *mut u32;
        (*s).word_count = in_bytes.wrapping_sub(first_bytes) / 4;
        let last_bytes = in_bytes.wrapping_sub(first_bytes) & 3;
        let in_u8 = in_ as *const u8;
        for i in 0..first_bytes {
            (*s).bits |= (*in_u8.offset(i as isize) as u64).wrapping_shl((i * 8) as u32);
        }
        (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
        (*s).final_word = 0;
        for i in 0..last_bytes {
            (*s).final_word |= (*in_u8.offset(in_bytes.wrapping_sub(last_bytes).wrapping_add(i)
                as isize) as u32)
                .wrapping_shl((i * 8) as u32);
        }
        (*s).count = first_bytes.wrapping_mul(8);
        (*s).out = out as *mut c_char;
        (*s).out_end = (*s).out.wrapping_offset(out_bytes as isize);
        (*s).begin = out as *mut c_char;
        let mut count: c_int = 0;
        let mut bfinal: u32;
        loop {
            bfinal = cp_read_bits(s, 1);
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
                    set_error_reason(c"Detected unknown block type within input stream.");
                    free(s as *mut c_void);
                    return 0;
                }
                _ => {}
            }
            count += 1;
            if bfinal != 0 {
                break;
            }
        }
        free(s as *mut c_void);
        1
    }
}

// ---------------------------------------------------------------------------
// PNG decoding
// ---------------------------------------------------------------------------

/// `static uint8_t cp_paeth(uint8_t a, uint8_t b, uint8_t c)`
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

/// `static uint32_t cp_make32(const uint8_t *s)`
unsafe fn cp_make32(s: *const u8) -> u32 {
    unsafe {
        ((*s.offset(0) as u32) << 24)
            | ((*s.offset(1) as u32) << 16)
            | ((*s.offset(2) as u32) << 8)
            | (*s.offset(3) as u32)
    }
}

/// `static const uint8_t *cp_chunk(cp_raw_png_t *png, const char *chunk, uint32_t minlen)`
unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    unsafe {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        if memcmp(
            start.wrapping_offset(4) as *const c_void,
            chunk.as_ptr() as *const c_void,
            4,
        ) == 0
            && len >= minlen
        {
            // C: `int offset = len + 12;` -- signed, so it sign-extends.
            let offset = len.wrapping_add(12) as c_int;
            if (*png).p.wrapping_offset(offset as isize) <= (*png).end {
                (*png).p = (*png).p.wrapping_offset(offset as isize);
                return start.wrapping_offset(8);
            }
        }
        ptr::null()
    }
}

/// `static const uint8_t *cp_find(cp_raw_png_t *png, const char *chunk, uint32_t minlen)`
unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    unsafe {
        while (*png).p < (*png).end {
            let len = cp_make32((*png).p);
            let start = (*png).p;
            // C: `png->p += len + 12;` -- unsigned, so it zero-extends.
            (*png).p = (*png).p.wrapping_add(len.wrapping_add(12) as usize);
            if memcmp(
                start.wrapping_offset(4) as *const c_void,
                chunk.as_ptr() as *const c_void,
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
}

/// `static int cp_unfilter(int w, int h, int bpp, uint8_t *raw)`
unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    unsafe {
        let len = w.wrapping_mul(bpp);
        let mut raw = raw;
        let mut prev: *mut u8;
        let mut x: c_int;
        if h > 0 {
            let filter = *raw;
            raw = raw.wrapping_offset(1);
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
        raw = raw.wrapping_offset(len as isize);
        let mut y: c_int = 1;
        while y < h {
            let filter = *raw;
            raw = raw.wrapping_offset(1);
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
                        let sum = *raw.offset((x - bpp) as isize) as c_int
                            + *prev.offset(x as isize) as c_int;
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add((sum / 2) as u8);
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
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            *prev.offset(x as isize),
                            *prev.offset((x - bpp) as isize),
                        ));
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
}

/// `static void cp_convert(int bpp, int w, int h, uint8_t *src, cp_pixel_t *dst)`
unsafe fn cp_convert(bpp: c_int, w: c_int, h: c_int, src: *mut u8, dst: *mut cp_pixel_t) {
    unsafe {
        let mut src = src;
        let mut dst = dst;
        for _y in 0..h {
            src = src.wrapping_offset(1);
            let mut x: c_int = 0;
            while x < w {
                match bpp {
                    1 => {
                        *dst = cp_make_pixel(*src, *src, *src);
                        dst = dst.wrapping_offset(1);
                    }
                    2 => {
                        *dst = cp_make_pixel_a(*src, *src, *src, *src.offset(1));
                        dst = dst.wrapping_offset(1);
                    }
                    3 => {
                        *dst = cp_make_pixel(*src, *src.offset(1), *src.offset(2));
                        dst = dst.wrapping_offset(1);
                    }
                    4 => {
                        *dst =
                            cp_make_pixel_a(*src, *src.offset(1), *src.offset(2), *src.offset(3));
                        dst = dst.wrapping_offset(1);
                    }
                    _ => {}
                }
                x += 1;
                src = src.wrapping_offset(bpp as isize);
            }
        }
    }
}

/// `static uint8_t cp_get_alpha_for_indexed_image(int index, const uint8_t *trns, uint32_t trns_len)`
unsafe fn cp_get_alpha_for_indexed_image(index: c_int, trns: *const u8, trns_len: u32) -> u8 {
    unsafe {
        if trns.is_null() {
            255
        } else if (index as u32) >= trns_len {
            255
        } else {
            *trns.offset(index as isize)
        }
    }
}

/// `static void cp_depalette(int w, int h, uint8_t *src, cp_pixel_t *dst, const uint8_t *plte, const uint8_t *trns, uint32_t trns_len)`
unsafe fn cp_depalette(
    w: c_int,
    h: c_int,
    src: *mut u8,
    dst: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    unsafe {
        let mut src = src;
        let mut dst = dst;
        for _y in 0..h {
            src = src.wrapping_offset(1);
            let mut x: c_int = 0;
            while x < w {
                let c = *src as c_int;
                let r = *plte.offset((c * 3) as isize);
                let g = *plte.offset((c * 3 + 1) as isize);
                let b = *plte.offset((c * 3 + 2) as isize);
                let a = cp_get_alpha_for_indexed_image(c, trns, trns_len);
                *dst = cp_make_pixel_a(r, g, b, a);
                dst = dst.wrapping_offset(1);
                x += 1;
                src = src.wrapping_offset(1);
            }
        }
    }
}

/// `static uint32_t cp_get_chunk_byte_length(const uint8_t *chunk)`
unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    unsafe { cp_make32(chunk.wrapping_offset(-8)) }
}

/// `static int cp_out_size(cp_image_t *img, int bpp)`
fn cp_out_size(img: &cp_image_t, bpp: c_int) -> c_int {
    img.w.wrapping_add(1).wrapping_mul(img.h).wrapping_mul(bpp)
}

/// `cp_image_t load_png_mem(const uint8_t *png_data, int png_length)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    unsafe {
        const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
        let ihdr: *const u8;
        let mut first: *const u8;
        let plte: *const u8;
        let trns: *const u8;
        let bit_depth: c_int;
        let color_type: c_int;
        let mut bpp: c_int = 0;
        let w: c_int;
        let h: c_int;
        let pix_bytes: c_int;
        let compression: c_int;
        let filter: c_int;
        let interlace: c_int;
        let mut datalen: c_int;
        let mut offset: c_int;
        let out: *mut u8;
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

        macro_rules! fail {
            ($msg:expr) => {{
                set_error_reason($msg);
                free(data as *mut c_void);
                free(img.pix as *mut c_void);
                img.pix = ptr::null_mut();
                return img;
            }};
        }

        if memcmp(
            png.p as *const c_void,
            SIG.as_ptr() as *const c_void,
            8,
        ) != 0
        {
            fail!(c"incorrect file signature (is this a png file?)");
        }
        png.p = png.p.wrapping_offset(8);
        ihdr = cp_chunk(&raw mut png, b"IHDR", 13);
        if ihdr.is_null() {
            fail!(c"unable to find IHDR chunk");
        }
        bit_depth = *ihdr.offset(8) as c_int;
        color_type = *ihdr.offset(9) as c_int;
        if !(bit_depth == 8) {
            fail!(c"only bit-depth of 8 is supported");
        }
        match color_type {
            0 => bpp = 1,
            2 => bpp = 3,
            3 => bpp = 1,
            4 => bpp = 2,
            6 => bpp = 4,
            _ => {
                fail!(c"unknown color type");
            }
        }
        w = cp_make32(ihdr).wrapping_add(1) as c_int;
        h = cp_make32(ihdr.offset(4)) as c_int;
        if !(w >= 1) {
            fail!(c"invalid IHDR chunk found, image width was less than 1");
        }
        if !(h >= 1) {
            fail!(c"invalid IHDR chunk found, image height was less than 1");
        }
        if !(((w as i64).wrapping_mul(h as i64) as u64)
            .wrapping_mul(core::mem::size_of::<cp_pixel_t>() as u64)
            < c_int::MAX as u64)
        {
            fail!(c"image too large");
        }
        pix_bytes = (w as i64)
            .wrapping_mul(h as i64)
            .wrapping_mul(core::mem::size_of::<cp_pixel_t>() as i64) as c_int;
        img.w = w - 1;
        img.h = h;
        img.pix = malloc(pix_bytes as usize) as *mut cp_pixel_t;
        if img.pix.is_null() {
            fail!(c"unable to allocate raw image space");
        }
        compression = *ihdr.offset(10) as c_int;
        filter = *ihdr.offset(11) as c_int;
        interlace = *ihdr.offset(12) as c_int;
        if !(compression == 0) {
            fail!(c"only standard compression DEFLATE is supported");
        }
        if !(filter == 0) {
            fail!(c"only standard adaptive filtering is supported");
        }
        if !(interlace == 0) {
            fail!(c"interlacing is not supported");
        }
        first = png.p;
        plte = cp_find(&raw mut png, b"PLTE", 0);
        if plte.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }
        trns = cp_find(&raw mut png, b"tRNS", 0);
        if trns.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }
        datalen = 0;
        {
            let mut idat = cp_find(&raw mut png, b"IDAT", 0);
            while !idat.is_null() {
                let len = cp_get_chunk_byte_length(idat);
                datalen = (datalen as u32).wrapping_add(len) as c_int;
                idat = cp_chunk(&raw mut png, b"IDAT", 0);
            }
        }
        png.p = first;
        data = malloc(datalen as usize) as *mut u8;
        offset = 0;
        {
            let mut idat = cp_find(&raw mut png, b"IDAT", 0);
            while !idat.is_null() {
                let len = cp_get_chunk_byte_length(idat);
                memcpy(
                    data.wrapping_offset(offset as isize) as *mut c_void,
                    idat as *const c_void,
                    len as usize,
                );
                offset = (offset as u32).wrapping_add(len) as c_int;
                idat = cp_chunk(&raw mut png, b"IDAT", 0);
            }
        }
        if !(!data.is_null() && datalen >= 6) {
            fail!(c"corrupt zlib structure in DEFLATE stream");
        }
        if !((*data.offset(0) & 0x0f) == 0x08) {
            fail!(c"only zlib compression method (RFC 1950) is supported");
        }
        if !((*data.offset(0) & 0xf0) <= 0x70) {
            fail!(c"innapropriate window size detected");
        }
        if !((*data.offset(1) & 0x20) == 0) {
            fail!(c"preset dictionary is present and not supported");
        }
        if !(cp_out_size(&img, 4) >= 1) {
            fail!(c"invalid image size found");
        }
        if !(cp_out_size(&img, bpp) >= 1) {
            fail!(c"invalid image size found");
        }
        out = (img.pix as *mut u8)
            .wrapping_offset(cp_out_size(&img, 4) as isize)
            .wrapping_offset(-(cp_out_size(&img, bpp) as isize));
        if cp_inflate(
            data.wrapping_offset(2) as *mut c_void,
            datalen - 6,
            out as *mut c_void,
            pix_bytes,
        ) == 0
        {
            fail!(c"DEFLATE algorithm failed");
        }
        if cp_unfilter(img.w, img.h, bpp, out) == 0 {
            fail!(c"invalid filter byte found");
        }
        if color_type == 3 {
            if plte.is_null() {
                fail!(c"color type of indexed requires a PLTE chunk");
            }
            let trns_len: u32 = if !trns.is_null() {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_layout_matches_c() {
        assert_eq!(core::mem::size_of::<cp_state_t>(), 2464);
        assert_eq!(core::mem::align_of::<cp_state_t>(), 8);
        assert_eq!(core::mem::offset_of!(cp_state_t, out), 48);
        assert_eq!(core::mem::offset_of!(cp_state_t, lookup), 72);
        assert_eq!(core::mem::offset_of!(cp_state_t, lit), 1096);
        assert_eq!(core::mem::offset_of!(cp_state_t, dst), 2248);
        assert_eq!(core::mem::offset_of!(cp_state_t, len), 2376);
        assert_eq!(core::mem::offset_of!(cp_state_t, nlit), 2452);
    }

    /// The `.data` blob of the reference `.so` is exactly these 672 bytes: the
    /// six tables in reverse source order, each 32-byte aligned, gaps zeroed.
    #[test]
    fn data_blob_model_matches_reference() {
        unsafe {
            let mut expect = [0u8; DATA_BLOB_LEN];
            let put = |e: &mut [u8; DATA_BLOB_LEN], off: usize, src: &[u8]| {
                e[off..off + src.len()].copy_from_slice(src);
            };
            let db: Vec<u8> = (*(&raw const cp_dist_base))
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let lb: Vec<u8> = (*(&raw const cp_len_base))
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            put(&mut expect, OFF_FIXED_TABLE, &*(&raw const cp_fixed_table));
            put(
                &mut expect,
                OFF_PERMUTATION_ORDER,
                &*(&raw const cp_permutation_order),
            );
            put(&mut expect, OFF_LEN_EXTRA_BITS, &*(&raw const cp_len_extra_bits));
            put(&mut expect, OFF_LEN_BASE, &lb);
            put(&mut expect, OFF_DIST_EXTRA_BITS, &*(&raw const cp_dist_extra_bits));
            put(&mut expect, OFF_DIST_BASE, &db);
            for off in 0..DATA_BLOB_LEN {
                assert_eq!(blob_byte(off), expect[off], "blob byte {off}");
            }
            assert_eq!(blob_byte(DATA_BLOB_LEN), 0); // .bss completed.0
            // values the reference library returns for these out-of-range reads
            assert_eq!(at_u8(OFF_LEN_EXTRA_BITS, 31), 0); // 1 alignment gap byte
            assert_eq!(at_u8(OFF_LEN_EXTRA_BITS, 32), 3); // cp_len_base[0] LSB
            assert_eq!(at_u32(OFF_LEN_BASE, 31), 0); // 4 alignment gap bytes
            assert_eq!(at_u32(OFF_LEN_BASE, 32), 0); // cp_dist_extra_bits[0..4]
            assert_eq!(at_u8(OFF_DIST_EXTRA_BITS, 32), 1); // cp_dist_base[0] LSB
            assert_eq!(at_u8(OFF_PERMUTATION_ORDER, 19), 0); // 13 gap bytes
            assert_eq!(at_u8(OFF_PERMUTATION_ORDER, 32), 0); // cp_len_extra_bits[0]
            assert_eq!(at_u32(OFF_DIST_BASE, 32), 0); // .bss completed.0
            assert_eq!(at_u8(OFF_FIXED_TABLE, 320), 16); // cp_permutation_order[0]
            // in-range reads still read the tables
            for i in 0..31 {
                assert_eq!(at_u8(OFF_LEN_EXTRA_BITS, i), (*(&raw const cp_len_extra_bits))[i as usize]);
                assert_eq!(at_u32(OFF_LEN_BASE, i), (*(&raw const cp_len_base))[i as usize]);
            }
            for i in 0..32 {
                assert_eq!(at_u8(OFF_DIST_EXTRA_BITS, i), (*(&raw const cp_dist_extra_bits))[i as usize]);
                assert_eq!(at_u32(OFF_DIST_BASE, i), (*(&raw const cp_dist_base))[i as usize]);
            }
            for i in 0..19 {
                assert_eq!(
                    at_u8(OFF_PERMUTATION_ORDER, i),
                    (*(&raw const cp_permutation_order))[i as usize]
                );
            }
        }
    }

    #[test]
    fn tables_match_c_literals() {
        unsafe {
            let ft = &*(&raw const cp_fixed_table);
            assert_eq!(ft.len(), 320);
            assert!(ft[0..144].iter().all(|&v| v == 8));
            assert!(ft[144..256].iter().all(|&v| v == 9));
            assert!(ft[256..280].iter().all(|&v| v == 7));
            assert!(ft[280..288].iter().all(|&v| v == 8));
            assert!(ft[288..320].iter().all(|&v| v == 5));
            assert_eq!((*(&raw const cp_permutation_order)).len(), 19);
            assert_eq!((*(&raw const cp_len_extra_bits)).len(), 31);
            assert_eq!((*(&raw const cp_len_base)).len(), 31);
            assert_eq!((*(&raw const cp_dist_extra_bits)).len(), 32);
            assert_eq!((*(&raw const cp_dist_base)).len(), 32);
            assert_eq!((*(&raw const cp_len_base))[28], 258);
            assert_eq!((*(&raw const cp_dist_base))[29], 24577);
        }
    }

    #[test]
    fn image_layout_matches_c() {
        assert_eq!(core::mem::size_of::<cp_image_t>(), 16);
        assert_eq!(core::mem::size_of::<cp_pixel_t>(), 4);
    }
}
