//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) plus the
//! public header (`include/lib.h`). Its only exported symbol is `crc16`; the
//! CRC lookup tables are `static const` in C and therefore have internal
//! linkage (they are not part of the ABI).
//!
//! The translation reproduces the C semantics exactly, including the integer
//! promotion / truncation behaviour of the original expressions.

#![allow(non_camel_case_types)]

mod tables;

use tables::TFLAC_CRC16_TABLES;

// Typedefs from `include/lib.h`.
pub type tflac_u8 = u8;
pub type tflac_u16 = u16;
pub type tflac_u32 = u32;

/// `tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);`
///
/// Note that in the C source the third parameter shadows the function name;
/// that has no effect on the ABI. The loop structure, table indices and
/// wrapping arithmetic mirror the C code one-for-one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crc16(d: *const tflac_u8, len: tflac_u32, crc16: tflac_u16) -> tflac_u16 {
    // Keep the parameters in mutable locals, exactly as C does.
    let mut d = d;
    let mut len = len;
    let mut crc16 = crc16;

    // Both loops below are no-ops when `len == 0`, so a NULL `d` is never
    // dereferenced in that case, matching the C code.
    while len >= 8 {
        // crc16 ^= d[0] << 8 | d[1];
        //
        // In C both operands are promoted to `int`; `d[0] << 8 | d[1]` fits in
        // 16 bits, and the result is truncated back into a tflac_u16.
        let b0 = *d.add(0) as u32;
        let b1 = *d.add(1) as u32;
        crc16 ^= ((b0 << 8) | b1) as tflac_u16;

        crc16 = TFLAC_CRC16_TABLES[7][(crc16 >> 8) as usize]
            ^ TFLAC_CRC16_TABLES[6][(crc16 & 0xFF) as usize]
            ^ TFLAC_CRC16_TABLES[5][*d.add(2) as usize]
            ^ TFLAC_CRC16_TABLES[4][*d.add(3) as usize]
            ^ TFLAC_CRC16_TABLES[3][*d.add(4) as usize]
            ^ TFLAC_CRC16_TABLES[2][*d.add(5) as usize]
            ^ TFLAC_CRC16_TABLES[1][*d.add(6) as usize]
            ^ TFLAC_CRC16_TABLES[0][*d.add(7) as usize];

        d = d.add(8);
        len -= 8;
    }

    // while (len--) { ... }
    //
    // The condition tests the pre-decrement value; `len` wraps on the final
    // iteration but the loop has already terminated by then.
    while len != 0 {
        len = len.wrapping_sub(1);

        // crc16 = (crc16 << 8) ^ tflac_crc16_tables[0][(crc16 >> 8) ^ *d++];
        let byte = *d;
        d = d.add(1);
        let idx = ((crc16 >> 8) ^ (byte as tflac_u16)) as usize;
        crc16 = crc16.wrapping_shl(8) ^ TFLAC_CRC16_TABLES[0][idx];
    }

    crc16
}
