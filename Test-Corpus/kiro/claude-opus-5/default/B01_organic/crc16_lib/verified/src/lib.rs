//! Rust translation of `c_src/src/lib.c` (tflac CRC-16).
//!
//! The C source exposes a single function, `crc16`, using the slice-by-8
//! table-driven variant of the FLAC CRC-16. No preprocessor namespace macro is
//! applied in `lib.h`, so the exported linker symbol is plain `crc16`.

mod tables;

use core::ffi::c_void;
use tables::TFLAC_CRC16_TABLES;

// typedef uint8_t  tflac_u8;
// typedef uint16_t tflac_u16;
// typedef uint32_t tflac_u32;
#[allow(non_camel_case_types)]
type tflac_u8 = u8;
#[allow(non_camel_case_types)]
type tflac_u16 = u16;
#[allow(non_camel_case_types)]
type tflac_u32 = u32;

/// `tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16)`
///
/// # Safety
/// `d` must point to at least `len` readable bytes when `len > 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crc16(
    d: *const tflac_u8,
    len: tflac_u32,
    crc16: tflac_u16,
) -> tflac_u16 {
    // The C code reads exactly `len` bytes: 8 per iteration of the first loop,
    // then one per iteration of the second. With len == 0 neither loop body
    // runs (`while (len--)` is false on the first test), and `d` is never
    // dereferenced, so avoid forming a slice from a possibly-null pointer.
    if len == 0 {
        return crc16;
    }

    debug_assert!(!d.cast::<c_void>().is_null());
    let data: &[tflac_u8] = unsafe { core::slice::from_raw_parts(d, len as usize) };
    crc16_impl(data, crc16)
}

/// Safe implementation mirroring the C control flow byte for byte.
fn crc16_impl(mut d: &[tflac_u8], mut crc: tflac_u16) -> tflac_u16 {
    let mut len: tflac_u32 = d.len() as tflac_u32;

    // while (len >= 8) { ... d += 8; len -= 8; }
    while len >= 8 {
        // crc16 ^= d[0] << 8 | d[1];
        crc ^= (tflac_u16::from(d[0]) << 8) | tflac_u16::from(d[1]);

        // crc16 = tflac_crc16_tables[7][crc16 >> 8] ^
        //         tflac_crc16_tables[6][crc16 & 0xFF] ^
        //         tflac_crc16_tables[5][d[2]] ^ tflac_crc16_tables[4][d[3]] ^
        //         tflac_crc16_tables[3][d[4]] ^ tflac_crc16_tables[2][d[5]] ^
        //         tflac_crc16_tables[1][d[6]] ^ tflac_crc16_tables[0][d[7]];
        crc = TFLAC_CRC16_TABLES[7][usize::from(crc >> 8)]
            ^ TFLAC_CRC16_TABLES[6][usize::from(crc & 0xFF)]
            ^ TFLAC_CRC16_TABLES[5][usize::from(d[2])]
            ^ TFLAC_CRC16_TABLES[4][usize::from(d[3])]
            ^ TFLAC_CRC16_TABLES[3][usize::from(d[4])]
            ^ TFLAC_CRC16_TABLES[2][usize::from(d[5])]
            ^ TFLAC_CRC16_TABLES[1][usize::from(d[6])]
            ^ TFLAC_CRC16_TABLES[0][usize::from(d[7])];

        d = &d[8..];
        len -= 8;
    }

    // while (len--) {
    //     crc16 = (crc16 << 8) ^ tflac_crc16_tables[0][(crc16 >> 8) ^ *d++];
    // }
    let mut i: usize = 0;
    while len != 0 {
        len -= 1;
        let byte = d[i];
        i += 1;
        let idx = ((crc >> 8) as tflac_u8) ^ byte;
        crc = (crc << 8) ^ TFLAC_CRC16_TABLES[0][usize::from(idx)];
    }

    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(data: &[u8], mut crc: u16) -> u16 {
        // Bitwise CRC-16/ANSI (poly 0x8005) used as an independent check of
        // the table-driven implementation.
        for &b in data {
            crc ^= u16::from(b) << 8;
            for _ in 0..8 {
                crc = if crc & 0x8000 != 0 {
                    (crc << 1) ^ 0x8005
                } else {
                    crc << 1
                };
            }
        }
        crc
    }

    #[test]
    fn matches_bitwise_reference() {
        let data: Vec<u8> = (0u32..1000).map(|i| (i * 37 + 11) as u8).collect();
        for len in 0..data.len() {
            let slice = &data[..len];
            let got = unsafe { crc16(slice.as_ptr(), len as u32, 0) };
            assert_eq!(got, reference(slice, 0), "len {len}");
            let got_seed = unsafe { crc16(slice.as_ptr(), len as u32, 0xBEEF) };
            assert_eq!(got_seed, reference(slice, 0xBEEF), "len {len} seeded");
        }
    }

    #[test]
    fn empty_input_returns_seed() {
        assert_eq!(unsafe { crc16(core::ptr::null(), 0, 0x1234) }, 0x1234);
    }
}
