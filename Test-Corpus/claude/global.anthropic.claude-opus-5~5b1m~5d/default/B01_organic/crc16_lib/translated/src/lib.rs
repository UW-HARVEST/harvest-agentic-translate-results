//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) plus one
//! public header (`include/lib.h`). The complete exported ABI is:
//!
//! ```c
//! typedef uint8_t  tflac_u8;
//! typedef uint16_t tflac_u16;
//! typedef uint32_t tflac_u32;
//!
//! tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);
//! ```
//!
//! The `tflac_crc16_tables` array in the header is declared `static`, so it has
//! internal linkage and contributes no dynamic symbol; it is reproduced as a
//! crate-private table in [`tables`].
//!
//! No namespace/renaming preprocessor macros are present in the header, so the
//! linker symbol is plain `crc16`.

#![deny(missing_docs)]

mod tables;

use tables::TFLAC_CRC16_TABLES as T;

/// `tflac_u8` — `uint8_t`.
#[allow(non_camel_case_types)]
pub type tflac_u8 = u8;

/// `tflac_u16` — `uint16_t`.
#[allow(non_camel_case_types)]
pub type tflac_u16 = u16;

/// `tflac_u32` — `uint32_t`.
#[allow(non_camel_case_types)]
pub type tflac_u32 = u32;

/// Safe core of [`crc16`], operating on a byte slice.
///
/// This mirrors the C loop structure exactly:
///
/// * a slice-by-8 stage that consumes whole 8-byte groups while at least 8
///   bytes remain, and
/// * a byte-at-a-time stage for the tail.
///
/// All arithmetic is done in `u16`, which reproduces the C behaviour: the C code
/// integer-promotes the operands to `int`, computes, then truncates back into
/// the `tflac_u16` variable on assignment. Every intermediate value in this
/// routine fits in 16 bits after that truncation, and the only shift that can
/// discard bits (`crc16 << 8`) discards exactly the bits the C truncation would.
fn crc16_impl(d: &[u8], mut crc: u16) -> u16 {
    let mut pos = 0usize;
    let mut len = d.len();

    // while (len >= 8) { ... d += 8; len -= 8; }
    while len >= 8 {
        let c = &d[pos..pos + 8];

        // crc16 ^= d[0] << 8 | d[1];
        crc ^= ((c[0] as u16) << 8) | (c[1] as u16);

        // crc16 = tables[7][crc16 >> 8] ^ tables[6][crc16 & 0xFF] ^ ...
        crc = T[7][(crc >> 8) as usize]
            ^ T[6][(crc & 0xFF) as usize]
            ^ T[5][c[2] as usize]
            ^ T[4][c[3] as usize]
            ^ T[3][c[4] as usize]
            ^ T[2][c[5] as usize]
            ^ T[1][c[6] as usize]
            ^ T[0][c[7] as usize];

        pos += 8;
        len -= 8;
    }

    // while (len--) { crc16 = (crc16 << 8) ^ tables[0][(crc16 >> 8) ^ *d++]; }
    //
    // Note the post-decrement in the C source also decrements `len` on the
    // failing test, wrapping it to UINT32_MAX; `len` is dead after the loop, so
    // that has no observable effect.
    while len > 0 {
        let idx = ((crc >> 8) as u8) ^ d[pos];
        crc = (crc << 8) ^ T[0][idx as usize];
        pos += 1;
        len -= 1;
    }

    crc
}

/// `tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16)`
///
/// Slice-by-8 CRC-16 (the FLAC frame CRC-16, polynomial `0x8005`) over `len`
/// bytes starting at `d`, seeded with `crc16`.
///
/// # Safety
///
/// `d` must point to at least `len` readable bytes, exactly as required by the
/// C original. When `len == 0` the pointer is never dereferenced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crc16(d: *const tflac_u8, len: tflac_u32, crc16: tflac_u16) -> tflac_u16 {
    // The C code reads exactly `len` bytes and touches `d` not at all when
    // `len == 0`, so only build a slice for the non-empty case.
    if len == 0 {
        return crc16;
    }

    let bytes = unsafe { core::slice::from_raw_parts(d, len as usize) };
    crc16_impl(bytes, crc16)
}
