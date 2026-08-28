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

#![allow(non_camel_case_types)]

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
/// Exactly like the C original, no null / alignment validation is performed on
/// either argument; passing invalid pointers is undefined behaviour in both
/// versions.
///
/// # Safety
///
/// `m` must point to a valid, initialized `tflac_md5`, and `out` must point to
/// at least 16 writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut tflac_u8) {
    // Read the state words, then write the 16 output bytes. Using a local copy
    // plus a slice keeps the body safe Rust while producing byte-identical
    // output to the C store-by-store sequence.
    let md5: tflac_md5 = unsafe { *m };
    let dst: &mut [tflac_u8] = unsafe { core::slice::from_raw_parts_mut(out, 16) };

    dst[0] = md5.a as tflac_u8;
    dst[1] = (md5.a >> 8) as tflac_u8;
    dst[2] = (md5.a >> 16) as tflac_u8;
    dst[3] = (md5.a >> 24) as tflac_u8;
    dst[4] = md5.b as tflac_u8;
    dst[5] = (md5.b >> 8) as tflac_u8;
    dst[6] = (md5.b >> 16) as tflac_u8;
    dst[7] = (md5.b >> 24) as tflac_u8;
    dst[8] = md5.c as tflac_u8;
    dst[9] = (md5.c >> 8) as tflac_u8;
    dst[10] = (md5.c >> 16) as tflac_u8;
    dst[11] = (md5.c >> 24) as tflac_u8;
    dst[12] = md5.d as tflac_u8;
    dst[13] = (md5.d >> 8) as tflac_u8;
    dst[14] = (md5.d >> 16) as tflac_u8;
    dst[15] = (md5.d >> 24) as tflac_u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_c() {
        assert_eq!(core::mem::size_of::<tflac_md5>(), 16);
        assert_eq!(core::mem::align_of::<tflac_md5>(), 4);
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
}
