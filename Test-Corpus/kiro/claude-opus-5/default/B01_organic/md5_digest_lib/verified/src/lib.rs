//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C `.so`):
//!   * `md5_digest`
//!
//! The header declares no namespace-renaming macros, so the source-level names
//! are also the final linker symbols.

#![allow(non_camel_case_types)]

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;

/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// `struct tflac_md5` — four 32-bit words, `#[repr(C)]` to match the C layout.
#[repr(C)]
pub struct tflac_md5 {
    pub a: tflac_u32,
    pub b: tflac_u32,
    pub c: tflac_u32,
    pub d: tflac_u32,
}

/// Serialize the four MD5 state words into `out` as 16 little-endian bytes.
///
/// C signature: `void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);`
///
/// An array parameter in C decays to a pointer, so `out` is `*mut tflac_u8`.
/// The C code performs no NULL checks; this translation reproduces that
/// behavior exactly (dereferencing NULL is UB in both languages).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut tflac_u8) {
    // Three subtleties are reproduced here that a naive slice-based
    // implementation gets wrong:
    //
    // (1) Unaligned tolerance. We must never form a `&tflac_md5` reference
    //     nor a `&mut [u8]` slice from these raw pointers: Rust would assert
    //     4-byte alignment for the struct and abort on a misaligned address,
    //     whereas the C simply performs (unaligned-capable) loads. So we work
    //     purely through raw pointers. Reading only a single byte at a time
    //     makes the alignment requirement vacuous (`u8` has alignment 1), so
    //     any misaligned `m` is handled exactly as the C handles it.
    //
    // (2) Per-byte reload under aliasing. Neither pointer is `restrict`, so
    //     `out` may overlap `*m`. The unoptimized C reloads the struct field
    //     from memory immediately before each single-byte store, so a store
    //     into an overlapping byte changes the value a later store observes.
    //     To match, we FRESHLY read the source byte inside every iteration
    //     (never hoisted) and emit exactly one single-byte write per step, so
    //     within each iteration the read precedes that iteration's store and
    //     later iterations observe bytes written by earlier ones.
    //
    // (3) NULL faults like the C. We deliberately avoid
    //     `core::ptr::read_unaligned` / `copy_nonoverlapping`: with
    //     debug-assertions those emit a precondition check that aborts
    //     (SIGABRT) on a NULL pointer *before* the faulting load. A plain
    //     single-byte raw dereference emits no such check, so a NULL `m`
    //     produces a genuine faulting load and dies with SIGSEGV, matching C.
    //
    // Equivalence proof. The C statement for output index `i` is
    // `out[i] = (tflac_u8)(<field> >> shift)`, where the field starts at byte
    // offset `4*(i/4)` and `shift == 8*(i%4)`. On this little-endian target,
    // byte `k` of a `u32` is the byte at `+k` from the field's start, so the
    // byte the shift selects is at absolute offset `4*(i/4) + (i%4) == i`.
    // Reading that one byte observes exactly the same memory state as the C's
    // full 4-byte load followed by the shift.
    let base = m as *const u8;
    let mut i = 0usize;
    while i < 16 {
        let field_off = 4 * (i / 4); // 0 for a, 4 for b, 8 for c, 12 for d
        let byte_in_field = i % 4; // == shift / 8
        // SAFETY: raw single-byte load/store, mirroring the C statement
        // `out[i] = (tflac_u8)(<field> >> 8*byte_in_field);`
        unsafe { out.add(i).write(*base.add(field_off + byte_in_field)) };
        i += 1;
    }
}
