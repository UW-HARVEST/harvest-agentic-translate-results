//! Rust translation of c_src/src/lib.c (cute_png style DEFLATE + PNG unfilter).
//!
//! The translation is intentionally literal: it reproduces the exact control
//! flow, arithmetic (including wrapping / truncation), error-check ordering and
//! out-of-bounds behaviour of the original C.
//!
//! `assert()` FIDELITY: `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and the
//! documented build passes no `-DNDEBUG`, so every `assert()` in `lib.c` is live
//! in the reference `.so` (`nm -D` shows `U __assert_fail`). A failing assert
//! calls `abort()` → SIGABRT, which is observable behaviour reachable from
//! trivial inputs (e.g. `cp_inflate(in, 0, out, n)`). `cp_assert!` below
//! reproduces each one, in source order, with `std::process::abort()`.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_assignments)]
#![allow(unused_variables)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

/// Mirrors glibc `assert()`: on failure `__assert_fail` calls `abort()`, i.e.
/// the process dies with `SIGABRT`. Nothing is printed, because the assertion
/// text/line number is not part of the ABI-observable behaviour we can match.
macro_rules! cp_assert {
    ($cond:expr) => {
        if !($cond) {
            std::process::abort();
        }
    };
}

// ---------------------------------------------------------------------------
// Public (exported) globals
// ---------------------------------------------------------------------------

/// `const char *cp_error_reason;`
#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

/// `uint8_t cp_fixed_table[288 + 32]`
#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = [
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5,
];

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

#[inline]
unsafe fn set_error(msg: &'static [u8]) {
    // The C code assigns string literals to cp_error_reason.
    *ptr::addr_of_mut!(cp_error_reason) = msg.as_ptr() as *const c_char;
}

// ---------------------------------------------------------------------------
// Internal types (mirroring the C layout exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
struct cp_pixel_t {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
struct cp_image_t {
    w: c_int,
    h: c_int,
    pix: *mut cp_pixel_t,
}

// `static` helpers in the C file; unused there as well (kept for fidelity).
#[allow(dead_code)]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

#[allow(dead_code)]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

/// Mirror of `cp_state_t`. `#[repr(C)]` guarantees the same field offsets as
/// the C struct, which matters because `cp_decode` reads `tree[-1]`.
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
    cp_assert!(((*s).bits_left & 7) == 0); // assert(!(s->bits_left & 7));
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
            cp_assert!((*s).word_index <= (*s).word_count);
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count += (*s).bits_left;
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    cp_assert!((*s).count >= num_bits_to_read);
    let mask = (1u64.wrapping_shl(num_bits_to_read as u32)).wrapping_sub(1);
    let bits = (*s).bits & mask;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits as u32
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

// ---------------------------------------------------------------------------
// Huffman table construction / decoding
// ---------------------------------------------------------------------------

unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    let mut codes: [c_int; 16] = [0; 16];
    let mut first: [c_int; 16] = [0; 16];
    // The C declares `int counts[16] = {0}` and then does `counts[lens[n]]++`
    // for every `n`, so a length byte >= 16 scribbles past the array. Padding to
    // 256 entries reproduces the part of that behaviour which is observable
    // (counts[0..=15] are unaffected, exactly as in C) without UB; the
    // `assert(len < 16)` below is what actually terminates such a call.
    let mut counts: [c_int; 256] = [0; 256];

    let mut n: c_int = 0;
    while n < sym_count {
        counts[*lens.offset(n as isize) as usize] += 1;
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
        ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, 1 << 9);
    }

    let mut i: c_int = 0;
    while i < sym_count {
        let len = *lens.offset(i as isize) as c_int;
        if len != 0 {
            cp_assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] = codes[len as usize].wrapping_add(1);
            let slot = first[len as usize] as u32;
            first[len as usize] = first[len as usize].wrapping_add(1);
            *tree.offset(slot as i32 as isize) =
                (code.wrapping_shl((32 - len) as u32)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j: c_int = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    *(*s).lookup.as_mut_ptr().offset(j as isize) =
                        (((len as u32) << 9) | (i as u32)) as u16;
                    j += 1 << len;
                }
            }
        }
        i += 1;
    }

    first[15]
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi: c_int) -> c_int {
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
    // C: `uint32_t len = (32 - (key & 0xF));` then
    //    `assert((search >> len) == (key >> len));`
    // When `key & 0xF == 0` the shift count is 32, which is UB in C; at -O0 on
    // x86-64 `shr %cl` masks the count to 5 bits, i.e. a shift by 0.
    // `wrapping_shr` reproduces exactly that.
    let len: u32 = 32u32.wrapping_sub(key & 0xF);
    cp_assert!(search.wrapping_shr(len) == key.wrapping_shr(len));
    let code = cp_consume_bits(s, (key & 0xF) as c_int);
    let _ = code;
    ((key >> 4) & 0xFFF) as c_int
}

// ---------------------------------------------------------------------------
// Block decoders
// ---------------------------------------------------------------------------

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
    ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, LEN as usize);
    (*s).out = (*s).out.offset(LEN as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    let table = ptr::addr_of_mut!(cp_fixed_table) as *mut u8;
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), table, 288) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        table.offset(288),
        32,
    ) as u32;
    1
}

/// Byte-exact model of `cp_dynamic`'s stack frame as generated by
/// `cc -O0 -fPIC` on x86-64 (verified against
/// `objdump -d c_src/build/CMakeFiles/*.dir/src/lib.c.o`).
///
/// Why this is necessary: the C declares `uint8_t lens[288 + 32]` and runs the
/// code-length loop while `n < nlit + ndst` (`nlit + ndst <= 320`). A single
/// `case 18` run adds up to 138, so `n` can go from 319 to 457 — up to 137 bytes
/// past the array. Those writes land on `cp_dynamic`'s *other* locals, and
/// because `-O0` keeps every local in memory and re-reads it on each use, the
/// overflow genuinely rewrites `ndst`, `nlit`, the `case 16/17/18` counters and
/// `n` itself, changing the loop's own bounds. Reproducing the frame layout is
/// the only way to match that.
///
/// `%rbp`-relative offsets taken from the disassembly:
///
/// | rbp offset | variable                | `lens[]` index |
/// |------------|------------------------|----------------|
/// | `-0x188`   | `s` (parameter spill)  | `-8 .. 0`      |
/// | `-0x180`   | `lens[320]`            | `0 .. 320`     |
/// | `-0x40`    | `lenlens[19]`          | `320 .. 339`   |
/// | `-0x2d`    | padding (9 bytes)      | `339 .. 348`   |
/// | `-0x24`    | `sym`                  | `348 .. 352`   |
/// | `-0x20`    | `nlen`                 | `352 .. 356`   |
/// | `-0x1c`    | `ndst`                 | `356 .. 360`   |
/// | `-0x18`    | `nlit`                 | `360 .. 364`   |
/// | `-0x14`    | `case 18` counter `i`  | `364 .. 368`   |
/// | `-0x10`    | `case 17` counter `i`  | `368 .. 372`   |
/// | `-0x0c`    | `case 16` counter `i`  | `372 .. 376`   |
/// | `-0x08`    | `n`                    | `376 .. 380`   |
/// | `-0x04`    | permutation loop `i`   | `380 .. 384`   |
/// | `+0x00`    | saved `%rbp`           | `384 .. 392`   |
/// | `+0x08`    | return address         | `392 .. 400`   |
///
/// Note `lens[-1]`, read by `case 16` when `n == 0`, is byte 7 of the spilled
/// `s` pointer, i.e. the most-significant byte of a heap address — reliably
/// `0x00` on x86-64 user space. So that read is deterministic, not garbage.
#[repr(C)]
struct cp_dyn_frame {
    s_slot: u64,        //   0  (rbp-0x188)
    lens: [u8; 320],    //   8  (rbp-0x180)
    lenlens: [u8; 19],  // 328  (rbp-0x40)
    pad_a: [u8; 9],     // 347  (rbp-0x2d)
    sym: c_int,         // 356  (rbp-0x24)
    nlen: c_int,        // 360  (rbp-0x20)
    ndst: c_int,        // 364  (rbp-0x1c)
    nlit: c_int,        // 368  (rbp-0x18)
    i18: c_int,         // 372  (rbp-0x14)
    i17: c_int,         // 376  (rbp-0x10)
    i16: c_int,         // 380  (rbp-0x0c)
    n: c_int,           // 384  (rbp-0x08)
    iperm: c_int,       // 388  (rbp-0x04)
    saved_rbp: u64,     // 392  (rbp+0x00)
    ret_addr: u64,      // 400  (rbp+0x08)
    /// Slack standing in for the caller's frame. Writes this far up mean the C
    /// has already destroyed its own return path; see `cp_lost_control`.
    beyond: [u8; 4096], // 408
}

const FRAME_LENS_OFF: isize = 8;
/// Largest `lens[]` index that still lands inside `cp_dyn_frame`.
const FRAME_LENS_MAX: isize = (472 + 4096 - 8) as isize;

/// The C has scribbled so far up its stack that its own return address (and the
/// caller's frame) are gone; control transfers to an address assembled from
/// code-length bytes (values `0..=18`), which is never mapped. Reproduce the
/// resulting fault rather than performing an unbounded wild write.
#[cold]
unsafe fn cp_lost_control() -> ! {
    // A deliberate access to an unmapped low address: same SIGSEGV the C takes
    // when it `ret`s into a small integer.
    ptr::write_volatile(8usize as *mut u8, 0);
    core::hint::unreachable_unchecked()
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    // Everything below mirrors the C frame; each local is read/written through
    // memory so that an out-of-range `lens[n]` aliases it exactly as in the C.
    let mut fr: cp_dyn_frame = core::mem::zeroed();
    let base = (&mut fr) as *mut cp_dyn_frame as *mut u8;
    fr.s_slot = s as u64;

    let lens: *mut u8 = base.offset(FRAME_LENS_OFF);
    let lenlens: *mut u8 = base.add(328);
    let p_sym = base.add(356) as *mut c_int;
    let p_nlen = base.add(360) as *mut c_int;
    let p_ndst = base.add(364) as *mut c_int;
    let p_nlit = base.add(368) as *mut c_int;
    let p_i18 = base.add(372) as *mut c_int;
    let p_i17 = base.add(376) as *mut c_int;
    let p_i16 = base.add(380) as *mut c_int;
    let p_n = base.add(384) as *mut c_int;
    let p_iperm = base.add(388) as *mut c_int;

    // Bounds-checked `&lens[idx]`; outside the modelled frame the C is writing
    // arbitrarily far up its stack and has lost its return path.
    // `hwm` records the highest index written so the saved-%rbp / return-address
    // smash can be detected even when the bytes written happen to be zero.
    let mut hwm: c_int = -1;
    let at = |idx: c_int, hwm: &mut c_int| -> *mut u8 {
        let i = idx as isize;
        if i < -FRAME_LENS_OFF || i > FRAME_LENS_MAX {
            cp_lost_control();
        }
        if idx > *hwm {
            *hwm = idx;
        }
        lens.offset(i)
    };

    // uint8_t lenlens[19] = {0};
    ptr::write_bytes(lenlens, 0, 19);

    *p_nlit = 257 + cp_read_bits(s, 5) as c_int;
    *p_ndst = 1 + cp_read_bits(s, 5) as c_int;
    *p_nlen = 4 + cp_read_bits(s, 4) as c_int;

    let perm = ptr::addr_of_mut!(cp_permutation_order) as *mut u8;
    *p_iperm = 0;
    while *p_iperm < *p_nlen {
        let idx = *perm.offset(*p_iperm as isize) as usize;
        *lenlens.add(idx) = cp_read_bits(s, 3) as u8;
        *p_iperm += 1;
    }
    (*s).nlen = cp_build(ptr::null_mut(), (*s).len.as_mut_ptr(), lenlens, 19) as u32;

    *p_n = 0;
    while *p_n < (*p_nlit).wrapping_add(*p_ndst) {
        *p_sym = cp_decode(s, (*s).len.as_mut_ptr(), (*s).nlen as c_int);
        match *p_sym {
            16 => {
                // for (int i = 3 + cp_read_bits(s, 2); i; --i, ++n)
                //     lens[n] = lens[n - 1];
                *p_i16 = 3 + cp_read_bits(s, 2) as c_int;
                while *p_i16 != 0 {
                    let v = *at((*p_n).wrapping_sub(1), &mut hwm);
                    *at(*p_n, &mut hwm) = v;
                    *p_i16 -= 1;
                    *p_n += 1;
                }
            }
            17 => {
                *p_i17 = 3 + cp_read_bits(s, 3) as c_int;
                while *p_i17 != 0 {
                    *at(*p_n, &mut hwm) = 0;
                    *p_i17 -= 1;
                    *p_n += 1;
                }
            }
            18 => {
                *p_i18 = 11 + cp_read_bits(s, 7) as c_int;
                while *p_i18 != 0 {
                    *at(*p_n, &mut hwm) = 0;
                    *p_i18 -= 1;
                    *p_n += 1;
                }
            }
            _ => {
                *at(*p_n, &mut hwm) = *p_sym as u8;
                *p_n += 1;
            }
        }
    }

    // The final two cp_build calls re-read the (possibly overwritten) nlit/ndst.
    let nlit = *p_nlit;
    let ndst = *p_ndst;
    let mut sink = hwm;
    if nlit > 0 {
        let _ = at(nlit - 1, &mut sink);
    }
    if ndst > 0 {
        let _ = at(nlit.wrapping_add(ndst).wrapping_sub(1), &mut sink);
    }
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), lens, nlit) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        at_const(lens, nlit),
        ndst,
    ) as u32;

    // If the overflow reached the saved frame pointer (lens[384]) or the return
    // address (lens[392]), the C's epilogue (`leave; ret`) resumes with a
    // destroyed frame pointer / at a bogus address assembled from code-length
    // bytes. Model that as a fault; see ERRORS.md section F.
    if hwm >= 384 {
        cp_lost_control();
    }
    1
}

#[inline]
unsafe fn at_const(lens: *mut u8, idx: c_int) -> *const u8 {
    let i = idx as isize;
    if i < -FRAME_LENS_OFF || i > FRAME_LENS_MAX {
        cp_lost_control();
    }
    lens.offset(i)
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    let len_extra = ptr::addr_of_mut!(cp_len_extra_bits) as *mut u8;
    let len_base = ptr::addr_of_mut!(cp_len_base) as *mut u32;
    let dist_extra = ptr::addr_of_mut!(cp_dist_extra_bits) as *mut u8;
    let dist_base = ptr::addr_of_mut!(cp_dist_base) as *mut u32;

    loop {
        let mut symbol = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.offset(1) <= (*s).out_end) {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.offset(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length: c_int = cp_read_bits(s, *len_extra.offset(symbol as isize) as c_int)
                .wrapping_add(*len_base.offset(symbol as isize)) as c_int;
            let distance_symbol = cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as c_int);
            let backwards_distance: c_int =
                cp_read_bits(s, *dist_extra.offset(distance_symbol as isize) as c_int)
                    .wrapping_add(*dist_base.offset(distance_symbol as isize)) as c_int;
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
            let mut length = length;
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
                }
                _ => loop {
                    let cond = length != 0;
                    length -= 1;
                    if !cond {
                        break;
                    }
                    *dst = *src;
                    dst = dst.offset(1);
                    src = src.offset(1);
                },
            }
        } else {
            break;
        }
    }
    1
}

// ---------------------------------------------------------------------------
// Public: cp_inflate
// ---------------------------------------------------------------------------

/// `int cp_inflate(void *in, int in_bytes, void *out, int out_bytes);`
#[unsafe(no_mangle)]
pub extern "C" fn cp_inflate(
    in_: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        let layout = core::alloc::Layout::new::<cp_state_t>();
        let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;
        if s.is_null() {
            // calloc failure in C would dereference NULL; nothing sensible to do.
            return 0;
        }

        (*s).bits = 0;
        (*s).count = 0;
        (*s).word_index = 0;
        (*s).bits_left = in_bytes.wrapping_mul(8);
        let addr = in_ as usize;
        let first_bytes = (((addr + 3) & !3usize).wrapping_sub(addr)) as c_int;
        (*s).words = (in_ as *mut c_char).offset(first_bytes as isize) as *mut u32;
        (*s).word_count = (in_bytes.wrapping_sub(first_bytes)) / 4;
        let last_bytes = (in_bytes.wrapping_sub(first_bytes)) & 3;
        let in_u8 = in_ as *const u8;
        let mut i: c_int = 0;
        while i < first_bytes {
            (*s).bits |= (*in_u8.offset(i as isize) as u64).wrapping_shl((i * 8) as u32);
            i += 1;
        }
        (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
        (*s).final_word = 0;
        i = 0;
        while i < last_bytes {
            (*s).final_word |= ((*in_u8.offset((in_bytes - last_bytes + i) as isize) as c_int)
                << (i * 8)) as u32;
            i += 1;
        }
        (*s).count = first_bytes.wrapping_mul(8);
        (*s).out = out as *mut c_char;
        (*s).out_end = (*s).out.offset(out_bytes as isize);
        (*s).begin = out as *mut c_char;

        let mut count: c_int = 0;
        let mut bfinal: c_int;
        let ok = loop {
            bfinal = cp_read_bits(s, 1) as c_int;
            let btype = cp_read_bits(s, 2) as c_int;
            match btype {
                0 => {
                    if cp_stored(s) == 0 {
                        break false;
                    }
                }
                1 => {
                    cp_fixed(s);
                    if cp_block(s) == 0 {
                        break false;
                    }
                }
                2 => {
                    cp_dynamic(s);
                    if cp_block(s) == 0 {
                        break false;
                    }
                }
                3 => {
                    set_error(b"Detected unknown block type within input stream.\0");
                    break false;
                }
                _ => {}
            }
            count += 1;
            if bfinal != 0 {
                break true;
            }
        };

        std::alloc::dealloc(s as *mut u8, layout);
        if ok {
            1
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// PNG helpers
// ---------------------------------------------------------------------------

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: c_int = (a as c_int) + (b as c_int) - (c as c_int);
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
    ((*s.offset(0) as u32) << 24)
        | ((*s.offset(1) as u32) << 16)
        | ((*s.offset(2) as u32) << 8)
        | (*s.offset(3) as u32)
}

#[allow(dead_code)]
unsafe fn cp_chunk(
    png: *mut cp_raw_png_t,
    chunk: *const c_char,
    minlen: u32,
) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if memcmp4(start.offset(4), chunk) == 0 && len >= minlen {
        let offset = len.wrapping_add(12) as c_int;
        if (*png).p.offset(offset as isize) <= (*png).end {
            (*png).p = (*png).p.offset(offset as isize);
            return start.offset(8);
        }
    }
    ptr::null()
}

#[allow(dead_code)]
unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.offset(len.wrapping_add(12) as isize);
        if memcmp4(start.offset(4), chunk) == 0 && len >= minlen && (*png).p <= (*png).end {
            return start.offset(8);
        }
    }
    ptr::null()
}

/// `memcmp(a, b, 4)` (only the zero / non-zero result is used).
unsafe fn memcmp4(a: *const u8, b: *const c_char) -> c_int {
    let mut i = 0isize;
    while i < 4 {
        let x = *a.offset(i);
        let y = *(b.offset(i) as *const u8);
        if x != y {
            return x as c_int - y as c_int;
        }
        i += 1;
    }
    0
}

// ---------------------------------------------------------------------------
// Public: unfilter
// ---------------------------------------------------------------------------

/// `int unfilter(int w, int h, int bpp, uint8_t *raw);`
#[unsafe(no_mangle)]
pub extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    unsafe {
        let len: c_int = w.wrapping_mul(bpp);
        let mut raw = raw;
        let mut prev: *mut u8;
        let mut x: c_int;

        if h > 0 {
            let filter = *raw;
            raw = raw.offset(1);
            match filter {
                0 => {}
                1 => {
                    x = bpp;
                    while x < len {
                        let v = *raw.offset((x - bpp) as isize);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                2 => {}
                3 => {
                    x = bpp;
                    while x < len {
                        let v = *raw.offset((x - bpp) as isize) / 2;
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                4 => {
                    x = bpp;
                    while x < len {
                        let v = cp_paeth(*raw.offset((x - bpp) as isize), 0, 0);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                _ => return 0,
            }
        }

        prev = raw;
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
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(0);
                        x += 1;
                    }
                    while x < len {
                        let v = *raw.offset((x - bpp) as isize);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                2 => {
                    x = 0;
                    while x < bpp {
                        let v = *prev.offset(x as isize);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                    while x < len {
                        let v = *prev.offset(x as isize);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                3 => {
                    x = 0;
                    while x < bpp {
                        let v = *prev.offset(x as isize) / 2;
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                    while x < len {
                        let v = ((*raw.offset((x - bpp) as isize) as c_int
                            + *prev.offset(x as isize) as c_int)
                            / 2) as u8;
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                }
                4 => {
                    x = 0;
                    while x < bpp {
                        let v = *prev.offset(x as isize);
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
                        x += 1;
                    }
                    while x < len {
                        let v = cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            *prev.offset(x as isize),
                            *prev.offset((x - bpp) as isize),
                        );
                        let t = raw.offset(x as isize);
                        *t = (*t).wrapping_add(v);
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

// ---------------------------------------------------------------------------
// Frame-layout guards
// ---------------------------------------------------------------------------

/// `cp_dyn_frame` must reproduce the C frame byte for byte; these offsets are
/// checked at compile time so a future edit cannot silently break the model.
const _: () = {
    macro_rules! off {
        ($f:ident) => {
            core::mem::offset_of!(cp_dyn_frame, $f)
        };
    }
    assert!(off!(s_slot) == 0);
    assert!(off!(lens) == 8);
    assert!(off!(lenlens) == 328); // lens[320]
    assert!(off!(pad_a) == 347); // lens[339]
    assert!(off!(sym) == 356); // lens[348]
    assert!(off!(nlen) == 360); // lens[352]
    assert!(off!(ndst) == 364); // lens[356]
    assert!(off!(nlit) == 368); // lens[360]
    assert!(off!(i18) == 372); // lens[364]
    assert!(off!(i17) == 376); // lens[368]
    assert!(off!(i16) == 380); // lens[372]
    assert!(off!(n) == 384); // lens[376]
    assert!(off!(iperm) == 388); // lens[380]
    assert!(off!(saved_rbp) == 392); // lens[384]
    assert!(off!(ret_addr) == 400); // lens[392]
    assert!(off!(beyond) == 408);
};
