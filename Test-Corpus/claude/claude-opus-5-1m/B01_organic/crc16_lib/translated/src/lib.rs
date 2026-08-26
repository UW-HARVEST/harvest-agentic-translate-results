//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`c_src/src/lib.c`) plus
//! its public header (`c_src/include/lib.h`). The only symbol with external
//! linkage — and therefore the entire public ABI — is `crc16`; the
//! `tflac_crc16_tables` array is `static` and is not exported.
//!
//! C declaration (from `c_src/include/lib.h`):
//!
//! ```c
//! typedef uint8_t  tflac_u8;
//! typedef uint16_t tflac_u16;
//! typedef uint32_t tflac_u32;
//!
//! tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);
//! ```

#![allow(non_camel_case_types)]

mod tables;

use tables::TFLAC_CRC16_TABLES;

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;
/// `typedef uint16_t tflac_u16;`
pub type tflac_u16 = u16;
/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// Translation of:
///
/// ```c
/// tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16) {
///     while (len >= 8) {
///         crc16 ^= d[0] << 8 | d[1];
///         crc16 = tflac_crc16_tables[7][crc16 >> 8] ^
///                 tflac_crc16_tables[6][crc16 & 0xFF] ^
///                 tflac_crc16_tables[5][d[2]] ^ tflac_crc16_tables[4][d[3]] ^
///                 tflac_crc16_tables[3][d[4]] ^ tflac_crc16_tables[2][d[5]] ^
///                 tflac_crc16_tables[1][d[6]] ^ tflac_crc16_tables[0][d[7]];
///         d += 8;
///         len -= 8;
///     }
///     while (len--) {
///         crc16 = (crc16 << 8) ^ tflac_crc16_tables[0][(crc16 >> 8) ^ *d++];
///     }
///     return crc16;
/// }
/// ```
///
/// All intermediate arithmetic in the C code is performed on `int` (via the
/// usual integer promotions) and truncated back to `tflac_u16` on assignment,
/// which is exactly what the wrapping 16-bit arithmetic below reproduces.
///
/// # Safety
///
/// `d` must point to at least `len` readable bytes, matching the contract of
/// the original C function. When `len` is zero the pointer is never
/// dereferenced (the C code also never touches it in that case).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crc16(d: *const tflac_u8, len: tflac_u32, crc16: tflac_u16) -> tflac_u16 {
    let mut crc: tflac_u16 = crc16;
    let mut len: tflac_u32 = len;

    // The C code dereferences `d` only while there are bytes left to process,
    // so a zero length short-circuits before the pointer is ever inspected.
    if len == 0 {
        return crc;
    }

    // SAFETY: the caller guarantees `len` readable bytes at `d` (checked
    // non-empty above), so this borrow covers exactly the bytes the C loops
    // would walk over via pointer increments.
    let data: &[tflac_u8] = unsafe { core::slice::from_raw_parts(d, len as usize) };
    let mut pos: usize = 0;

    // Slice-by-8 body: consume 8 bytes per iteration.
    while len >= 8 {
        crc ^= (u16::from(data[pos]) << 8) | u16::from(data[pos + 1]);
        crc = TFLAC_CRC16_TABLES[7][usize::from(crc >> 8)]
            ^ TFLAC_CRC16_TABLES[6][usize::from(crc & 0xFF)]
            ^ TFLAC_CRC16_TABLES[5][usize::from(data[pos + 2])]
            ^ TFLAC_CRC16_TABLES[4][usize::from(data[pos + 3])]
            ^ TFLAC_CRC16_TABLES[3][usize::from(data[pos + 4])]
            ^ TFLAC_CRC16_TABLES[2][usize::from(data[pos + 5])]
            ^ TFLAC_CRC16_TABLES[1][usize::from(data[pos + 6])]
            ^ TFLAC_CRC16_TABLES[0][usize::from(data[pos + 7])];
        pos += 8;
        len -= 8;
    }

    // Tail: one byte at a time. `crc16 << 8` is truncated to 16 bits on
    // assignment in C, which `wrapping_shl` mirrors here.
    while len != 0 {
        len -= 1;
        let idx = usize::from((crc >> 8) ^ u16::from(data[pos]));
        pos += 1;
        crc = crc.wrapping_shl(8) ^ TFLAC_CRC16_TABLES[0][idx];
    }

    crc
}
