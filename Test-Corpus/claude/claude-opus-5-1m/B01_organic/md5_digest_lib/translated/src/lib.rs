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

/// Serializes each of the four state words in little-endian order.
///
/// This is the safe, internal core of the translation: it performs exactly the
/// same 16 truncating stores as the C body, in the same order.
#[inline]
fn md5_digest_impl(m: &tflac_md5, out: &mut [tflac_u8; 16]) {
    // out[0..4]   <- m->a  (bytes 0, 8, 16, 24 shifts)
    out[0] = m.a as tflac_u8;
    out[1] = (m.a >> 8) as tflac_u8;
    out[2] = (m.a >> 16) as tflac_u8;
    out[3] = (m.a >> 24) as tflac_u8;

    // out[4..8]   <- m->b
    out[4] = m.b as tflac_u8;
    out[5] = (m.b >> 8) as tflac_u8;
    out[6] = (m.b >> 16) as tflac_u8;
    out[7] = (m.b >> 24) as tflac_u8;

    // out[8..12]  <- m->c
    out[8] = m.c as tflac_u8;
    out[9] = (m.c >> 8) as tflac_u8;
    out[10] = (m.c >> 16) as tflac_u8;
    out[11] = (m.c >> 24) as tflac_u8;

    // out[12..16] <- m->d
    out[12] = m.d as tflac_u8;
    out[13] = (m.d >> 8) as tflac_u8;
    out[14] = (m.d >> 16) as tflac_u8;
    out[15] = (m.d >> 24) as tflac_u8;
}

/// `void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);`
///
/// Writes the 128-bit MD5 state in little-endian byte order into `out`.
///
/// Like the C original, no NULL checks are performed on either argument: the C
/// code dereferences both pointers unconditionally, and that behavior is
/// reproduced exactly rather than "fixed".
///
/// # Safety
///
/// `m` must point to a valid, readable `tflac_md5`; `out` must point to a
/// writable buffer of at least 16 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut tflac_u8) {
    // Reborrow the raw C pointers, then do the work in safe Rust.
    let m_ref: &tflac_md5 = unsafe { &*m };
    let out_ref: &mut [tflac_u8; 16] = unsafe { &mut *(out as *mut [tflac_u8; 16]) };
    md5_digest_impl(m_ref, out_ref);
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
