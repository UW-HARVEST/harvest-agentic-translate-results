use crate::aes::{NB, SBOX, RSBOX};

/// Multiply two bytes in GF(2^8) using the AES irreducible polynomial.
pub fn g_mult(first: u8, second: u8) -> u8 {
    let mut a = first;
    let mut b = second;
    let mut p: u8 = 0;
    for _ in 0..8 {
        if (b & 1) != 0 {
            p ^= a;
        }
        let hi_bit_set = a & 0x80;
        a = a.wrapping_shl(1);
        if hi_bit_set != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    p
}

/// AES SubBytes step using the forward S-box.
pub fn sub(state: &mut [[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] = SBOX[state[i][j] as usize];
        }
    }
}

/// AES InvSubBytes step using the inverse S-box.
pub fn inv_sub(state: &mut [[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] = RSBOX[state[i][j] as usize];
        }
    }
}
