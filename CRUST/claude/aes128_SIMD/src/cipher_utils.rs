use crate::aes::{NB, NR, RSBOX, SBOX};

// `NR` is re-exported through this module's import to keep parity with the
// original C header organization, even though it isn't directly referenced
// by the helper functions below.
#[allow(dead_code)]
const _NR: usize = NR;

/// Galois Field (2^8) multiplication used in AES MixColumns / InvMixColumns.
pub fn g_mult(first: u8, second: u8) -> u8 {
    let mut a = first;
    let mut b = second;
    let mut p: u8 = 0;
    for _ in 0..8 {
        if (b & 1) != 0 {
            p ^= a;
        }
        let hi_bit_set = (a & 0x80) != 0;
        a <<= 1;
        if hi_bit_set {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    p
}

/// AES forward S-Box substitution applied to every byte of the state.
pub fn sub(state: &mut [[u8; NB]; 4]) {
    for row in state.iter_mut() {
        for byte in row.iter_mut() {
            *byte = SBOX[*byte as usize];
        }
    }
    let _ = NR; // silence unused import warning in release builds
}

/// AES inverse S-Box substitution applied to every byte of the state.
pub fn inv_sub(state: &mut [[u8; NB]; 4]) {
    for row in state.iter_mut() {
        for byte in row.iter_mut() {
            *byte = RSBOX[*byte as usize];
        }
    }
}
