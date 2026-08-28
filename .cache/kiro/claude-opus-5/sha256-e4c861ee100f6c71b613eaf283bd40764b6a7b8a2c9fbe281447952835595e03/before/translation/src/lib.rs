//! Rust translation of `c_src/src/lib.c` (tflac md5 digest serialization).
//!
//! Layout and behavior mirror the C original exactly:
//! the four 32-bit state words are written out little-endian into 16 bytes.

use std::ffi::c_void;

pub type TflacU8 = u8;
pub type TflacU32 = u32;

/// Mirrors `struct tflac_md5` from `include/lib.h`.
#[repr(C)]
pub struct TflacMd5 {
    pub a: TflacU32,
    pub b: TflacU32,
    pub c: TflacU32,
    pub d: TflacU32,
}

/// `void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);`
///
/// # Safety
/// `m` must point to a valid `tflac_md5` and `out` must point to a writable
/// buffer of at least 16 bytes. Both are dereferenced unconditionally, exactly
/// as the C code does (no NULL checks in the original).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const TflacMd5, out: *mut TflacU8) {
    // Match the C code's unconditional dereference of both pointers.
    let m = &*m;

    let bytes: [u8; 16] = {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&m.a.to_le_bytes());
        buf[4..8].copy_from_slice(&m.b.to_le_bytes());
        buf[8..12].copy_from_slice(&m.c.to_le_bytes());
        buf[12..16].copy_from_slice(&m.d.to_le_bytes());
        buf
    };

    std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_void, out as *mut c_void, 16);
}
