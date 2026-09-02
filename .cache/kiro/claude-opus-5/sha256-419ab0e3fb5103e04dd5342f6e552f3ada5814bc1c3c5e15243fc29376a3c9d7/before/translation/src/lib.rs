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
    // Borrow the inputs as safe slices/references, then do the byte
    // extraction in safe Rust.
    let m = unsafe { &*m };
    let out = unsafe { core::slice::from_raw_parts_mut(out, 16) };

    // Each word is written low byte first, matching the C shifts:
    //   out[n+0] = (u8)(w);       out[n+1] = (u8)(w >> 8);
    //   out[n+2] = (u8)(w >> 16); out[n+3] = (u8)(w >> 24);
    out[0..4].copy_from_slice(&m.a.to_le_bytes());
    out[4..8].copy_from_slice(&m.b.to_le_bytes());
    out[8..12].copy_from_slice(&m.c.to_le_bytes());
    out[12..16].copy_from_slice(&m.d.to_le_bytes());
}
