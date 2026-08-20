//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) with a
//! single public header (`include/lib.h`) that exports exactly one public
//! symbol: `md5_digest`.
//!
//! C header (`include/lib.h`):
//! ```c
//! #include <stdint.h>
//!
//! typedef uint8_t tflac_u8;
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
//! There are no namespace/renaming preprocessor macros in the header, so the
//! final linker symbol is plain `md5_digest`.

#![allow(non_camel_case_types)]

// The C typedefs, mirrored so the FFI signature matches exactly.
pub type tflac_u8 = u8;
pub type tflac_u32 = u32;

/// Mirrors `struct tflac_md5` from `include/lib.h`.
///
/// `#[repr(C)]` guarantees the same layout/alignment as the C struct
/// (four consecutive `uint32_t` fields, no padding).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct tflac_md5 {
    pub a: tflac_u32,
    pub b: tflac_u32,
    pub c: tflac_u32,
    pub d: tflac_u32,
}

/// `void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);`
///
/// Writes the 128-bit MD5 state in little-endian byte order into `out`.
///
/// Like the C original, no NULL checks and no length/alignment validation are
/// performed on either argument: the C code dereferences both pointers
/// unconditionally, and that behavior is reproduced exactly rather than
/// "fixed".
///
/// Fidelity notes (why this is written with raw pointers instead of Rust
/// references):
///
/// * The C compiles the body as 16 independent "load field, shift, truncate,
///   store byte" sequences. `const` on the `m` parameter does *not* grant the C
///   compiler a no-alias guarantee, so `m` and `out` are permitted to overlap
///   (e.g. `md5_digest(m, (tflac_u8 *)m)`), and each store is observed by the
///   following load. Materializing a `&tflac_md5` and a `&mut [u8; 16]` over
///   overlapping memory would be Rust UB (`noalias`), so the fields are re-read
///   from raw pointers before every byte store, exactly mirroring the C.
/// * `m` is not required by the C to be 4-byte aligned (x86-64 handles the
///   unaligned loads transparently), so each field is loaded through a
///   `#[repr(C, packed)]` view (alignment 1) rather than through an aligned
///   reference.
/// * A NULL/unmapped `m` or `out` must fault in the load/store itself with
///   SIGSEGV, exactly as in C — it must NOT be turned into a Rust panic, which
///   in an `extern "C"` function aborts the process with SIGABRT instead. Two
///   primitives had to be avoided to get that:
///   - `ptr::read_unaligned` / `copy_nonoverlapping`: their
///     `assert_unsafe_precondition!` is a *library*-UB check, which IS enabled
///     whenever `debug-assertions` are on, so a NULL `m` panicked → SIGABRT.
///   - plain place accesses (`*p`, `*p = v`): rustc emits a codegen-level null
///     check for raw-pointer derefs under `debug-assertions`, so a NULL `m` or
///     `out` panicked → SIGABRT.
///
///   `ptr::read_volatile` / `ptr::write_volatile` only carry *language*-UB
///   preconditions (off unless `-Zub-checks`), so they compile to the bare `mov`
///   the C emits and fault with SIGSEGV. Verified against the C in both the
///   `dev` and `release` profiles by `tests/error_paths.rs` (rows E1–E7).
/// * The accesses are *volatile* so that the optimiser cannot narrow a 4-byte
///   field load to the one byte actually used, nor merge the 16 one-byte stores
///   into wider stores, nor drop the repeated loads. In a `release` build the
///   non-volatile version narrowed the load of `m->a` to a single byte, which
///   made the Rust commit `out[0]` in a case where the C's full 4-byte load
///   faults first and commits nothing (`ERRORS.md` row E7, k=1..3).
/// * Offsets use `wrapping_add`, not `add`: `add` asserts in-bounds address
///   arithmetic, which a NULL/wild pointer does not satisfy, whereas the C just
///   emits `add $0x1,%rax`.
///
/// # Safety
///
/// `m` must point to 16 readable bytes; `out` must point to 16 writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut tflac_u8) {
    /// Alignment-1 view of a `tflac_u32` field, so a field load is a plain
    /// unaligned `mov` with no alignment assumption and no runtime check.
    #[repr(C, packed)]
    struct U32Raw(tflac_u32);

    /// One 4-byte load from an arbitrary address, mirroring the C's
    /// `mov (%rax),%eax`.
    ///
    /// `read_volatile` (not `read`) so the optimiser can neither narrow the
    /// 4-byte load to the single byte that is used, nor merge/hoist the repeated
    /// loads: both are observable through faults and through overlapping
    /// `m`/`out` (see the fidelity notes above).
    ///
    /// # Safety
    /// `p` must point to 4 readable bytes (no alignment requirement).
    #[inline(always)]
    unsafe fn load(p: *const tflac_u8) -> tflac_u32 {
        unsafe { core::ptr::read_volatile(p as *const U32Raw).0 }
    }

    /// One byte store, mirroring the C's `mov %dl,(%rax)`.
    ///
    /// `write_volatile` so the 16 byte stores cannot be merged into wider
    /// stores or reordered; the C commits them one at a time, in index order,
    /// which is observable when `out` is only partially writable.
    ///
    /// # Safety
    /// `p` must point to 1 writable byte.
    #[inline(always)]
    unsafe fn store(p: *mut tflac_u8, v: tflac_u8) {
        unsafe { core::ptr::write_volatile(p, v) }
    }

    unsafe {
        // Field addresses: offsets 0/4/8/12 of the `#[repr(C)]` struct.
        // `wrapping_add` performs the plain address arithmetic the C does.
        let base = m as *const tflac_u8;
        let a = base;
        let b = base.wrapping_add(4);
        let c = base.wrapping_add(8);
        let d = base.wrapping_add(12);

        // The field is re-loaded before every byte store, exactly as the C's
        // emitted code does, so overlapping `m`/`out` behaves identically.

        // out[0..4]   <- m->a  (shifts 0, 8, 16, 24)
        store(out.wrapping_add(0), load(a) as tflac_u8);
        store(out.wrapping_add(1), (load(a) >> 8) as tflac_u8);
        store(out.wrapping_add(2), (load(a) >> 16) as tflac_u8);
        store(out.wrapping_add(3), (load(a) >> 24) as tflac_u8);

        // out[4..8]   <- m->b
        store(out.wrapping_add(4), load(b) as tflac_u8);
        store(out.wrapping_add(5), (load(b) >> 8) as tflac_u8);
        store(out.wrapping_add(6), (load(b) >> 16) as tflac_u8);
        store(out.wrapping_add(7), (load(b) >> 24) as tflac_u8);

        // out[8..12]  <- m->c
        store(out.wrapping_add(8), load(c) as tflac_u8);
        store(out.wrapping_add(9), (load(c) >> 8) as tflac_u8);
        store(out.wrapping_add(10), (load(c) >> 16) as tflac_u8);
        store(out.wrapping_add(11), (load(c) >> 24) as tflac_u8);

        // out[12..16] <- m->d
        store(out.wrapping_add(12), load(d) as tflac_u8);
        store(out.wrapping_add(13), (load(d) >> 8) as tflac_u8);
        store(out.wrapping_add(14), (load(d) >> 16) as tflac_u8);
        store(out.wrapping_add(15), (load(d) >> 24) as tflac_u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn little_endian_layout() {
        let m = tflac_md5 {
            a: 0x03020100,
            b: 0x07060504,
            c: 0x0b0a0908,
            d: 0x0f0e0d0c,
        };
        let mut out = [0u8; 16];
        unsafe { md5_digest(&m, out.as_mut_ptr()) };
        assert_eq!(
            out,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    #[test]
    fn truncation_matches_c_casts() {
        let m = tflac_md5 {
            a: 0xdead_beef,
            b: 0x0000_00ff,
            c: 0xff00_0000,
            d: 0x1234_5678,
        };
        let mut out = [0xAAu8; 16];
        unsafe { md5_digest(&m, out.as_mut_ptr()) };
        assert_eq!(&out[0..4], &[0xef, 0xbe, 0xad, 0xde]);
        assert_eq!(&out[4..8], &[0xff, 0x00, 0x00, 0x00]);
        assert_eq!(&out[8..12], &[0x00, 0x00, 0x00, 0xff]);
        assert_eq!(&out[12..16], &[0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn struct_layout_is_16_bytes() {
        assert_eq!(core::mem::size_of::<tflac_md5>(), 16);
        assert_eq!(core::mem::align_of::<tflac_md5>(), 4);
    }
}
