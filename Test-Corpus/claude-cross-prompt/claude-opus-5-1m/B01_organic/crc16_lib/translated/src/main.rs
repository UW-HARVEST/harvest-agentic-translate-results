// The original C code in `c_src/` is a library (no `main` function).
// It defines a CRC16 routine and lookup tables but performs no I/O.
// The harness still expects a binary, so this entry point intentionally
// performs no work and produces no output, matching the behaviour of the
// (empty) C executable surface.
//
// The translated CRC16 logic is exposed via `lib.rs` for callers that link
// against this crate as a library.

mod tables;

use tables::TFLAC_CRC16_TABLES;

#[allow(dead_code)]
fn crc16(d: &[u8], len: u32, mut crc: u16) -> u16 {
    let mut len = len as usize;
    let total = d.len().min(len);
    let mut idx: usize = 0;

    while len >= 8 && idx + 8 <= total {
        let xor_val = ((d[idx] as u16) << 8) | (d[idx + 1] as u16);
        crc ^= xor_val;

        let hi = (crc >> 8) as usize;
        let lo = (crc & 0xFF) as usize;

        crc = TFLAC_CRC16_TABLES[7][hi]
            ^ TFLAC_CRC16_TABLES[6][lo]
            ^ TFLAC_CRC16_TABLES[5][d[idx + 2] as usize]
            ^ TFLAC_CRC16_TABLES[4][d[idx + 3] as usize]
            ^ TFLAC_CRC16_TABLES[3][d[idx + 4] as usize]
            ^ TFLAC_CRC16_TABLES[2][d[idx + 5] as usize]
            ^ TFLAC_CRC16_TABLES[1][d[idx + 6] as usize]
            ^ TFLAC_CRC16_TABLES[0][d[idx + 7] as usize];

        idx += 8;
        len -= 8;
    }

    while len > 0 && idx < total {
        let byte = d[idx];
        idx += 1;
        len -= 1;
        let hi = (crc >> 8) as u8;
        crc = (crc << 8) ^ TFLAC_CRC16_TABLES[0][(hi ^ byte) as usize];
    }

    crc
}

fn main() {
    // No-op: the original C source is a library and emits no output.
}
