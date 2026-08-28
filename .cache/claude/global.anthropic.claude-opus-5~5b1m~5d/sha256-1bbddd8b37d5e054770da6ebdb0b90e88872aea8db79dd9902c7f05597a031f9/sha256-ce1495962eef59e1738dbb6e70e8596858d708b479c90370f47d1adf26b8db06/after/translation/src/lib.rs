//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   * `md5_digest`
//!
//! Header (`include/lib.h`) declares:
//! ```c
//! typedef uint8_t  tflac_u8;
//! typedef uint32_t tflac_u32;
//!
//! struct tflac_md5 {
//!     tflac_u32 a;
//!     tflac_u32 b;
//!     tflac_u32 c;
//!     tflac_u32 d;
//! };
//! typedef struct tflac_md5 tflac_md5;
//!
//! void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);
//! ```
//!
//! There are no namespace-renaming preprocessor macros in the header, so the
//! linker symbol name is exactly `md5_digest`.
//!
//! # Fidelity notes
//!
//! The C body is 16 straight-line statements of the form
//! `out[i] = (tflac_u8)(m->field >> shift);`. Reproducing its *observable*
//! behaviour byte-for-byte requires matching three properties that a naive
//! transcription gets wrong:
//!
//! 1. **The source field is re-read before every single byte store.** `out` has
//!    type `tflac_u8 *` (i.e. `unsigned char *`), and a character-typed store may
//!    alias an object of any type, so the C compiler is *required* to reload
//!    `m->field` after each store to `out[i]`. This was confirmed in the
//!    generated code at both `-O0` and `-O2`:
//!
//!    ```text
//!    -O2:  mov (%rdi),%ecx ; mov %cl,(%rsi)          <- load, store out[0]
//!          mov (%rdi),%ecx ; mov %ch,0x1(%rsi)       <- RELOADED, store out[1]
//!          mov (%rdi),%ecx ; shr $0x10,%ecx ; ...    <- RELOADED again
//!    ```
//!
//!    Neither parameter is `restrict`, so `out` overlapping `*m` is legal,
//!    well-defined input. Snapshotting `*m` into a local up front (`let md5 =
//!    *m;`) would produce different bytes for every overlapping call, because
//!    the C sees each store's effect on the source it subsequently reloads.
//!    The loads and stores below are therefore `volatile`, which pins both the
//!    per-store reload and the store ordering.
//!
//! 2. **Neither pointer is required to be aligned.** The C performs plain
//!    (unaligned-tolerant on x86-64) 32-bit loads and never checks alignment.
//!    A Rust `*m` dereference of a misaligned pointer is UB and aborts under
//!    debug assertions ("misaligned pointer dereference"), where the C quietly
//!    returns the right answer. The source is consequently read one byte at a
//!    time and reassembled with [`u32::from_ne_bytes`], which is exact for any
//!    alignment and correct on either endianness (a byte-wise read followed by
//!    native-order reassembly is by definition the value a native 32-bit load
//!    at that address would have produced).
//!
//! 3. **There is no validation whatsoever** — no null checks, no range checks,
//!    no asserts (see `ERRORS.md`). The port must not invent an error surface:
//!    a null or unmapped pointer has to fault exactly like the C does, rather
//!    than being turned into a quiet early return or a Rust panic.

#![allow(non_camel_case_types)]

use core::mem::offset_of;
use core::ptr;

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;

/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// `struct tflac_md5` — layout-compatible with the C struct (four
/// naturally-aligned 32-bit words, no padding).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct tflac_md5 {
    pub a: tflac_u32,
    pub b: tflac_u32,
    pub c: tflac_u32,
    pub d: tflac_u32,
}

/// Read the 32-bit field at `base + off` the way the C's `m->field` does:
/// freshly (not cached across the surrounding stores) and without requiring
/// 4-byte alignment.
///
/// # Safety
///
/// `base + off .. base + off + 4` must be readable.
#[inline(always)]
unsafe fn load_field(base: *const tflac_u8, off: usize) -> tflac_u32 {
    let p = unsafe { base.add(off) };
    // Byte-wise so any alignment is fine; volatile so the load is not hoisted
    // out of, or cached across, the interleaved stores to `out`.
    let b = [
        unsafe { ptr::read_volatile(p) },
        unsafe { ptr::read_volatile(p.add(1)) },
        unsafe { ptr::read_volatile(p.add(2)) },
        unsafe { ptr::read_volatile(p.add(3)) },
    ];
    tflac_u32::from_ne_bytes(b)
}

/// Store one byte of `out`, mirroring a single `out[i] = ...;` statement.
///
/// # Safety
///
/// `out + i` must be writable.
#[inline(always)]
unsafe fn store_byte(out: *mut tflac_u8, i: usize, v: tflac_u8) {
    unsafe { ptr::write_volatile(out.add(i), v) }
}

/// Serialize the four MD5 state words little-endian into `out[0..16]`.
///
/// Faithful translation of:
///
/// ```c
/// void md5_digest(const tflac_md5 *m, tflac_u8 out[16]) {
///     out[0] = (tflac_u8)(m->a);
///     out[1] = (tflac_u8)(m->a >> 8);
///     ...
///     out[15] = (tflac_u8)(m->d >> 24);
/// }
/// ```
///
/// Each statement below reloads its source field and then stores its byte, in
/// the same order as the C, so overlapping `m`/`out` buffers produce identical
/// bytes. Exactly like the C original, no null / alignment / length validation
/// is performed on either argument.
///
/// # Safety
///
/// `m` must point to 16 readable bytes and `out` to 16 writable bytes. The two
/// ranges *may* overlap. Passing invalid pointers faults, exactly as in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut tflac_u8) {
    // Byte offsets of the four fields, taken from the type itself rather than
    // hard-coded, so they track `#[repr(C)]` layout on any target.
    const A: usize = offset_of!(tflac_md5, a);
    const B: usize = offset_of!(tflac_md5, b);
    const C: usize = offset_of!(tflac_md5, c);
    const D: usize = offset_of!(tflac_md5, d);

    let base = m as *const tflac_u8;

    unsafe {
        store_byte(out, 0, load_field(base, A) as tflac_u8);
        store_byte(out, 1, (load_field(base, A) >> 8) as tflac_u8);
        store_byte(out, 2, (load_field(base, A) >> 16) as tflac_u8);
        store_byte(out, 3, (load_field(base, A) >> 24) as tflac_u8);
        store_byte(out, 4, load_field(base, B) as tflac_u8);
        store_byte(out, 5, (load_field(base, B) >> 8) as tflac_u8);
        store_byte(out, 6, (load_field(base, B) >> 16) as tflac_u8);
        store_byte(out, 7, (load_field(base, B) >> 24) as tflac_u8);
        store_byte(out, 8, load_field(base, C) as tflac_u8);
        store_byte(out, 9, (load_field(base, C) >> 8) as tflac_u8);
        store_byte(out, 10, (load_field(base, C) >> 16) as tflac_u8);
        store_byte(out, 11, (load_field(base, C) >> 24) as tflac_u8);
        store_byte(out, 12, load_field(base, D) as tflac_u8);
        store_byte(out, 13, (load_field(base, D) >> 8) as tflac_u8);
        store_byte(out, 14, (load_field(base, D) >> 16) as tflac_u8);
        store_byte(out, 15, (load_field(base, D) >> 24) as tflac_u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_c() {
        assert_eq!(core::mem::size_of::<tflac_md5>(), 16);
        assert_eq!(core::mem::align_of::<tflac_md5>(), 4);
        assert_eq!(offset_of!(tflac_md5, a), 0);
        assert_eq!(offset_of!(tflac_md5, b), 4);
        assert_eq!(offset_of!(tflac_md5, c), 8);
        assert_eq!(offset_of!(tflac_md5, d), 12);
    }

    #[test]
    fn little_endian_serialization() {
        let m = tflac_md5 {
            a: 0x04030201,
            b: 0x08070605,
            c: 0x0c0b0a09,
            d: 0x100f0e0d,
        };
        let mut out = [0u8; 16];
        unsafe { md5_digest(&m, out.as_mut_ptr()) };
        assert_eq!(
            out,
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
    }

    #[test]
    fn truncation_and_extremes() {
        let m = tflac_md5 {
            a: 0xFFFF_FFFF,
            b: 0x0000_0000,
            c: 0xDEAD_BEEF,
            d: 0x0000_00FF,
        };
        let mut out = [0xAAu8; 16];
        unsafe { md5_digest(&m, out.as_mut_ptr()) };
        assert_eq!(
            out,
            [
                0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xEF, 0xBE, 0xAD, 0xDE, 0xFF,
                0x00, 0x00, 0x00,
            ]
        );
    }

    /// The source pointer need not be 4-byte aligned; the C does unaligned
    /// 32-bit loads without complaint.
    #[test]
    fn misaligned_source_is_fine() {
        let mut buf = [0u8; 32];
        for i in 0..16 {
            buf[1 + i] = (i as u8) + 1;
        }
        let mut out = [0u8; 16];
        unsafe { md5_digest(buf.as_ptr().add(1) as *const tflac_md5, out.as_mut_ptr()) };
        assert_eq!(
            out,
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
    }

    /// Fully-overlapping buffers: a byte-wise ascending copy onto itself is the
    /// identity, because each field is reloaded before each store.
    #[test]
    fn exact_overlap_is_identity() {
        let mut buf = [0u8; 16];
        for i in 0..16 {
            buf[i] = (i as u8) + 1;
        }
        let p = buf.as_mut_ptr();
        unsafe { md5_digest(p as *const tflac_md5, p) };
        assert_eq!(
            buf,
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
    }
}
