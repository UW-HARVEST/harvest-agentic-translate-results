// Rust translation of c_src/src/lib.c (`pinflate` DEFLATE decompressor).
//
// The translation is intentionally literal: pointer arithmetic, integer widths,
// evaluation order, error-check ordering and even the (enabled) `assert()`
// behaviour of the original C are reproduced.  The C library is built by CMake
// without `NDEBUG` (the reference `.so` references `__assert_fail`), so the
// asserts are live and are reproduced by calling glibc's `__assert_fail`.
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_assignments)]

use core::ptr;
use core::ptr::{addr_of, addr_of_mut};
use std::ffi::{c_char, c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// libc bindings (kept identical to what the C code uses)
// ---------------------------------------------------------------------------

extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_uint,
        function: *const c_char,
    ) -> !;
}

/// The C `__FILE__` for `c_src/src/lib.c` as CMake passes it to the compiler
/// (an absolute path); computed by `build.rs`.
const CP_ASSERT_FILE: &str = concat!(env!("CP_ASSERT_FILE"), "\0");

/// Reproduces `assert(expr)` from `<assert.h>` (NDEBUG *not* defined).
macro_rules! cp_assert {
    ($cond:expr, $text:expr, $func:expr, $line:expr) => {
        if !($cond) {
            unsafe {
                __assert_fail(
                    concat!($text, "\0").as_ptr() as *const c_char,
                    CP_ASSERT_FILE.as_ptr() as *const c_char,
                    $line as c_uint,
                    concat!($func, "\0").as_ptr() as *const c_char,
                )
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Types from lib.c
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

// `static` in C -> internal linkage; not exported, but translated for fidelity.
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    let mut p = cp_pixel_t { r: 0, g: 0, b: 0, a: 0 };
    p.r = r;
    p.g = g;
    p.b = b;
    p.a = a;
    p
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    let mut p = cp_pixel_t { r: 0, g: 0, b: 0, a: 0 };
    p.r = r;
    p.g = g;
    p.b = b;
    p.a = 0xFF;
    p
}

// Keep the unused static helpers referenced so they are not warned about while
// still having no effect on the exported ABI.
#[allow(dead_code)]
const _CP_UNUSED_HELPERS: (
    fn(u8, u8, u8, u8) -> cp_pixel_t,
    fn(u8, u8, u8) -> cp_pixel_t,
) = (cp_make_pixel_a, cp_make_pixel);

// ---------------------------------------------------------------------------
// Exported globals (`nm -D`: cp_error_reason, cp_fixed_table,
// cp_permutation_order, cp_len_extra_bits, cp_len_base, cp_dist_extra_bits,
// cp_dist_base)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

const fn cp_make_fixed_table() -> [u8; 288 + 32] {
    let mut t = [0u8; 288 + 32];
    let mut i = 0usize;
    // 144 x 8
    while i < 144 {
        t[i] = 8;
        i += 1;
    }
    // 112 x 9
    while i < 256 {
        t[i] = 9;
        i += 1;
    }
    // 24 x 7
    while i < 280 {
        t[i] = 7;
        i += 1;
    }
    // 8 x 8
    while i < 288 {
        t[i] = 8;
        i += 1;
    }
    // 32 x 5
    while i < 288 + 32 {
        t[i] = 5;
        i += 1;
    }
    t
}

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

// Raw accessors so that (possible) out-of-range indices behave like C instead
// of panicking, and so that runtime mutation of the exported globals is seen.
#[inline(always)]
unsafe fn g_fixed_table() -> *mut u8 {
    addr_of_mut!(cp_fixed_table) as *mut u8
}
#[inline(always)]
unsafe fn g_permutation_order() -> *const u8 {
    addr_of!(cp_permutation_order) as *const u8
}
#[inline(always)]
unsafe fn g_len_extra_bits() -> *const u8 {
    addr_of!(cp_len_extra_bits) as *const u8
}
#[inline(always)]
unsafe fn g_len_base() -> *const u32 {
    addr_of!(cp_len_base) as *const u32
}
#[inline(always)]
unsafe fn g_dist_extra_bits() -> *const u8 {
    addr_of!(cp_dist_extra_bits) as *const u8
}
#[inline(always)]
unsafe fn g_dist_base() -> *const u32 {
    addr_of!(cp_dist_base) as *const u32
}

#[inline(always)]
unsafe fn set_error_reason(msg: &'static [u8]) {
    *addr_of_mut!(cp_error_reason) = msg.as_ptr() as *const c_char;
}

// Error strings, byte-for-byte as produced by the C string-literal
// concatenations.
static E_LEN_NLEN: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
static E_STORED_BEYOND: &[u8] = b"Stored block extends beyond end of input stream.\0";
static E_OUT_SYMBOL: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.\0";
static E_BACK_DIST: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
static E_OUT_STRING: &[u8] = b"Attempted to overwrite out buffer while outputting a string.\0";
static E_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";

// ---------------------------------------------------------------------------
// cp_state_t  (layout must match the C struct exactly: `cp_decode` reads
// `tree[-1]`, which aliases the preceding struct member.)
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

// Verified against the C compiler's layout (`offsetof`/`sizeof`).  `cp_decode`
// dereferences `tree[-1]`, so these offsets are observable behaviour.
const _: () = {
    assert!(core::mem::size_of::<cp_state_t>() == 2464);
    assert!(core::mem::offset_of!(cp_state_t, bits) == 0);
    assert!(core::mem::offset_of!(cp_state_t, count) == 8);
    assert!(core::mem::offset_of!(cp_state_t, words) == 16);
    assert!(core::mem::offset_of!(cp_state_t, word_count) == 24);
    assert!(core::mem::offset_of!(cp_state_t, word_index) == 28);
    assert!(core::mem::offset_of!(cp_state_t, bits_left) == 32);
    assert!(core::mem::offset_of!(cp_state_t, final_word_available) == 36);
    assert!(core::mem::offset_of!(cp_state_t, final_word) == 40);
    assert!(core::mem::offset_of!(cp_state_t, out) == 48);
    assert!(core::mem::offset_of!(cp_state_t, out_end) == 56);
    assert!(core::mem::offset_of!(cp_state_t, begin) == 64);
    assert!(core::mem::offset_of!(cp_state_t, lookup) == 72);
    assert!(core::mem::offset_of!(cp_state_t, lit) == 1096);
    assert!(core::mem::offset_of!(cp_state_t, dst) == 2248);
    assert!(core::mem::offset_of!(cp_state_t, len) == 2376);
    assert!(core::mem::offset_of!(cp_state_t, nlit) == 2452);
    assert!(core::mem::offset_of!(cp_state_t, ndst) == 2456);
    assert!(core::mem::offset_of!(cp_state_t, nlen) == 2460);
};

// ---------------------------------------------------------------------------
// Bit reader
// ---------------------------------------------------------------------------

#[inline]
unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

#[inline]
unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    cp_assert!(((*s).bits_left & 7) == 0, "!(s->bits_left & 7)", "cp_ptr", 95);
    ((*s).words.wrapping_offset((*s).word_index as isize) as *mut c_char)
        .wrapping_offset(-(((*s).count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word: u32 = *(*s).words.wrapping_offset((*s).word_index as isize);
            (*s).word_index = (*s).word_index.wrapping_add(1);
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add(32);
            cp_assert!(
                (*s).word_index <= (*s).word_count,
                "s->word_index <= s->word_count",
                "cp_peak_bits",
                104
            );
        } else if (*s).final_word_available != 0 {
            let word: u32 = (*s).final_word;
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
        "cp_consume_bits",
        115
    );
    let bits: u32 =
        ((*s).bits & (1u64.wrapping_shl(num_bits_to_read as u32)).wrapping_sub(1)) as u32;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count = (*s).count.wrapping_sub(num_bits_to_read);
    (*s).bits_left = (*s).bits_left.wrapping_sub(num_bits_to_read);
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    cp_assert!(num_bits_to_read <= 32, "num_bits_to_read <= 32", "cp_read_bits", 123);
    cp_assert!(num_bits_to_read >= 0, "num_bits_to_read >= 0", "cp_read_bits", 124);
    cp_assert!((*s).bits_left > 0, "s->bits_left > 0", "cp_read_bits", 125);
    cp_assert!((*s).count <= 64, "s->count <= 64", "cp_read_bits", 126);
    cp_assert!(
        cp_would_overflow(s, num_bits_to_read) == 0,
        "!cp_would_overflow(s, num_bits_to_read)",
        "cp_read_bits",
        127
    );
    cp_peak_bits(s, num_bits_to_read);
    let bits = cp_consume_bits(s, num_bits_to_read);
    bits
}

#[inline]
fn cp_rev16(a_in: u32) -> u32 {
    let mut a = a_in;
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

// ---------------------------------------------------------------------------
// Huffman table construction
// ---------------------------------------------------------------------------

unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    // C declares `counts[16]`; a length byte >= 16 would be an out-of-bounds
    // write there (immediately followed by the `len < 16` assert firing).  The
    // oversized array keeps this defined while preserving observable results.
    let mut counts = [0i32; 256];

    let mut n: c_int = 0;
    while n < sym_count {
        counts[*lens.offset(n as isize) as usize] += 1;
        n += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    let mut n: usize = 1;
    while n <= 15 {
        codes[n] = (codes[n - 1].wrapping_add(counts[n - 1])) << 1;
        first[n] = first[n - 1].wrapping_add(counts[n - 1]);
        n += 1;
    }
    if !s.is_null() {
        ptr::write_bytes((*s).lookup.as_mut_ptr() as *mut u8, 0, 2 * (1 << 9));
    }
    let mut i: c_int = 0;
    while i < sym_count {
        let len: c_int = *lens.offset(i as isize) as c_int;
        if len != 0 {
            cp_assert!(len < 16, "len < 16", "cp_build", 154);
            let code: u32 = codes[len as usize] as u32;
            codes[len as usize] = codes[len as usize].wrapping_add(1);
            let slot: u32 = first[len as usize] as u32;
            first[len as usize] = first[len as usize].wrapping_add(1);
            *tree.offset(slot as isize) = (code.wrapping_shl((32 - len) as u32))
                | ((i as u32) << 4)
                | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j: c_int = (cp_rev16(code).wrapping_shr((16 - len) as u32)) as c_int;
                while j < (1 << 9) {
                    *(*s).lookup.as_mut_ptr().offset(j as isize) =
                        (((len as u32) << 9) | (i as u32)) as u16;
                    j = j.wrapping_add(1 << len);
                }
            }
        }
        i += 1;
    }
    let max_index: c_int = first[15];
    max_index
}

// ---------------------------------------------------------------------------
// Block decoders
// ---------------------------------------------------------------------------

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    let p: *mut c_char;
    cp_read_bits(s, (*s).count & 7);
    let LEN: u16 = cp_read_bits(s, 16) as u16;
    let NLEN: u16 = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        set_error_reason(E_LEN_NLEN);
        return 0;
    }
    if !((*s).bits_left / 8 <= LEN as c_int) {
        set_error_reason(E_STORED_BEYOND);
        return 0;
    }
    p = cp_ptr(s);
    ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, LEN as usize);
    (*s).out = (*s).out.wrapping_offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), g_fixed_table(), 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        g_fixed_table().offset(288),
        32,
    ) as u32;
    1
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi_in: c_int) -> c_int {
    let bits: u64 = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    let mut hi: c_int = hi_in;
    while lo < hi {
        let guess: c_int = (lo.wrapping_add(hi)) >> 1;
        if search < *tree.wrapping_offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key: u32 = *tree.wrapping_offset((lo - 1) as isize);
    let len: u32 = 32u32.wrapping_sub(key & 0xF);
    cp_assert!(
        search.wrapping_shr(len) == key.wrapping_shr(len),
        "(search >> len) == (key >> len)",
        "cp_decode",
        217
    );
    let code: c_int = cp_consume_bits(s, (key & 0xF) as c_int) as c_int;
    let _ = code;
    ((key >> 4) & 0xFFF) as c_int
}

// --- emulated stack frame for cp_dynamic ------------------------------------
//
// `cp_dynamic` declares `uint8_t lens[288 + 32]` and then, for malformed input,
// happily writes `lens[n]` for n far beyond 319: the code-length repeat codes
// (16/17/18) can advance `n` past the end of the array without any bound check.
// Those writes land on the *neighbouring locals* of the same stack frame, so the
// behaviour of the original library depends on the compiler's frame layout.
//
// The reference library is built by CMake with no CMAKE_BUILD_TYPE, i.e. gcc
// -O0 with asserts enabled, whose frame for `cp_dynamic` is (relative to %rbp):
//
//     -0x188  s              (spilled parameter, 8 bytes)
//     -0x180  lens[288 + 32]
//     -0x40   lenlens[19]
//     -0x24   sym
//     -0x20   nlen
//     -0x1c   ndst
//     -0x18   nlit
//     -0x14   i   (case 18)
//     -0x10   i   (case 17)
//     -0x0c   i   (case 16)
//     -0x08   n
//     -0x04   i   (code-length permutation loop)
//
// Consequently `lens[348]` aliases `sym`, `lens[360]` aliases `nlit`,
// `lens[364..376]` alias the three repeat counters and `lens[376]` aliases `n`
// itself -- which is what makes the original loop *never terminate* for some
// corrupt streams (writing 0 over the low byte of n rewinds it to 256).
// Reproducing the frame reproduces all of that, so this translation hangs
// exactly where the C hangs instead of quietly doing something else.
const FR_SIZE: usize = 0x188;
// Slack so that reads through the `lens` pointer past the emulated frame (only
// reachable once `nlit`/`ndst` have themselves been clobbered) stay inside a
// real allocation.
const FR_PAD: usize = 4096;
const FR_S: usize = 0x188 - 0x188;
const FR_LENS: usize = 0x188 - 0x180;
const FR_LENLENS: usize = 0x188 - 0x40;
const FR_SYM: usize = 0x188 - 0x24;
const FR_NLEN: usize = 0x188 - 0x20;
const FR_NDST: usize = 0x188 - 0x1c;
const FR_NLIT: usize = 0x188 - 0x18;
const FR_I18: usize = 0x188 - 0x14;
const FR_I17: usize = 0x188 - 0x10;
const FR_I16: usize = 0x188 - 0x0c;
const FR_N: usize = 0x188 - 0x08;
const FR_IPERM: usize = 0x188 - 0x04;

struct Frame {
    m: [u8; FR_SIZE + FR_PAD],
}

impl Frame {
    #[inline]
    fn new() -> Frame {
        Frame { m: [0u8; FR_SIZE + FR_PAD] }
    }
    /// Byte access clamped into the buffer.  Every access the translated code
    /// actually performs is in range (`n` can never exceed 376 before it is
    /// rewound by the aliasing write), the clamp only rules out wild writes.
    #[inline]
    fn clamp(i: isize) -> usize {
        if i >= 0 && (i as usize) < FR_SIZE + FR_PAD {
            i as usize
        } else {
            FR_SIZE + FR_PAD - 1
        }
    }
    #[inline]
    fn get_u8(&self, i: isize) -> u8 {
        self.m[Frame::clamp(i)]
    }
    #[inline]
    fn set_u8(&mut self, i: isize, v: u8) {
        let k = Frame::clamp(i);
        self.m[k] = v;
    }
    #[inline]
    fn get_i32(&self, i: usize) -> c_int {
        c_int::from_le_bytes([self.m[i], self.m[i + 1], self.m[i + 2], self.m[i + 3]])
    }
    #[inline]
    fn set_i32(&mut self, i: usize, v: c_int) {
        self.m[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }
    #[inline]
    fn lens(&mut self) -> *mut u8 {
        unsafe { self.m.as_mut_ptr().add(FR_LENS) }
    }
    #[inline]
    fn lenlens(&self) -> *const u8 {
        unsafe { self.m.as_ptr().add(FR_LENLENS) }
    }
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    let mut fr = Frame::new();
    // The spilled `s` parameter sits just below `lens`, so `lens[-1]` (read by
    // the case-16 repeat code when n == 0) observes its most significant byte.
    fr.m[FR_S..FR_S + 8].copy_from_slice(&(s as usize).to_le_bytes());

    // uint8_t lenlens[19] = {0};   (already zeroed)
    fr.set_i32(FR_NLIT, 257i32.wrapping_add(cp_read_bits(s, 5) as c_int));
    fr.set_i32(FR_NDST, 1i32.wrapping_add(cp_read_bits(s, 5) as c_int));
    fr.set_i32(FR_NLEN, 4i32.wrapping_add(cp_read_bits(s, 4) as c_int));

    fr.set_i32(FR_IPERM, 0);
    while fr.get_i32(FR_IPERM) < fr.get_i32(FR_NLEN) {
        // gcc evaluates cp_read_bits() before indexing cp_permutation_order.
        let v = cp_read_bits(s, 3) as u8;
        let idx = *g_permutation_order().offset(fr.get_i32(FR_IPERM) as isize) as isize;
        fr.set_u8(FR_LENLENS as isize + idx, v);
        fr.set_i32(FR_IPERM, fr.get_i32(FR_IPERM).wrapping_add(1));
    }
    (*s).nlen = cp_build(ptr::null_mut(), (*s).len.as_mut_ptr(), fr.lenlens(), 19) as u32;

    fr.set_i32(FR_N, 0);
    while fr.get_i32(FR_N) < fr.get_i32(FR_NLIT).wrapping_add(fr.get_i32(FR_NDST)) {
        let sym: c_int = cp_decode(s, (*s).len.as_mut_ptr(), (*s).nlen as c_int);
        fr.set_i32(FR_SYM, sym);
        match fr.get_i32(FR_SYM) {
            16 => {
                fr.set_i32(FR_I16, 3i32.wrapping_add(cp_read_bits(s, 2) as c_int));
                while fr.get_i32(FR_I16) != 0 {
                    let n = fr.get_i32(FR_N);
                    let v = fr.get_u8(FR_LENS as isize + (n as isize) - 1);
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
                // gcc bumps `n` first, then stores to lens[old n].
                let old_n = fr.get_i32(FR_N);
                fr.set_i32(FR_N, old_n.wrapping_add(1));
                let v = fr.get_i32(FR_SYM) as u8;
                fr.set_u8(FR_LENS as isize + old_n as isize, v);
            }
        }
    }
    let nlit_final = fr.get_i32(FR_NLIT);
    let ndst_final = fr.get_i32(FR_NDST);
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), fr.lens(), nlit_final) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        fr.lens().wrapping_offset(nlit_final as isize),
        ndst_final,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    loop {
        let mut symbol: c_int = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.wrapping_offset(1) <= (*s).out_end) {
                set_error_reason(E_OUT_SYMBOL);
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.wrapping_offset(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length: c_int = cp_read_bits(s, *g_len_extra_bits().wrapping_offset(symbol as isize) as c_int)
                .wrapping_add(*g_len_base().wrapping_offset(symbol as isize)) as c_int;
            let distance_symbol: c_int =
                cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as c_int);
            let backwards_distance: c_int =
                cp_read_bits(s, *g_dist_extra_bits().wrapping_offset(distance_symbol as isize) as c_int)
                    .wrapping_add(*g_dist_base().wrapping_offset(distance_symbol as isize))
                    as c_int;
            if !((*s).out.wrapping_offset(-(backwards_distance as isize)) >= (*s).begin) {
                set_error_reason(E_BACK_DIST);
                return 0;
            }
            if !((*s).out.wrapping_offset(length as isize) <= (*s).out_end) {
                set_error_reason(E_OUT_STRING);
                return 0;
            }
            let src: *mut c_char = (*s).out.wrapping_offset(-(backwards_distance as isize));
            let dst: *mut c_char = (*s).out;
            (*s).out = (*s).out.wrapping_offset(length as isize);
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
                }
                _ => {
                    let mut sp = src;
                    let mut dp = dst;
                    let mut l = length;
                    while l != 0 {
                        l -= 1;
                        *dp = *sp;
                        dp = dp.offset(1);
                        sp = sp.offset(1);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pinflate(
    input: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let s: *mut cp_state_t = calloc(1, core::mem::size_of::<cp_state_t>()) as *mut cp_state_t;
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let first_bytes: c_int =
        ((((input as usize).wrapping_add(3)) & !3usize).wrapping_sub(input as usize)) as c_int;
    (*s).words = (input as *mut c_char).wrapping_offset(first_bytes as isize) as *mut u32;
    (*s).word_count = (in_bytes.wrapping_sub(first_bytes)) / 4;
    let last_bytes: c_int = (in_bytes.wrapping_sub(first_bytes)) & 3;
    let mut i: c_int = 0;
    while i < first_bytes {
        (*s).bits |= ((*(input as *const u8).wrapping_offset(i as isize)) as u64)
            .wrapping_shl((i.wrapping_mul(8)) as u32);
        i += 1;
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    let mut i: c_int = 0;
    while i < last_bytes {
        (*s).final_word |= ((*(input as *const u8)
            .wrapping_offset((in_bytes.wrapping_sub(last_bytes).wrapping_add(i)) as isize))
            as u32)
            .wrapping_shl((i.wrapping_mul(8)) as u32);
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
        let btype: c_int = cp_read_bits(s, 2) as c_int;
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
                set_error_reason(E_UNKNOWN_BLOCK);
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
