use crate::aes::{NB, NR, NK};
use crate::cipher_utils::{sbox_lookup, RCON};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Standard AES-128 key expansion
    // Copy key into first Nk words
    for i in 0..NK {
        w[4 * i] = key[4 * i];
        w[4 * i + 1] = key[4 * i + 1];
        w[4 * i + 2] = key[4 * i + 2];
        w[4 * i + 3] = key[4 * i + 3];
    }

    for i in NK..NB * (NR + 1) {
        let mut temp = [w[4 * (i - 1)], w[4 * (i - 1) + 1], w[4 * (i - 1) + 2], w[4 * (i - 1) + 3]];
        if i % NK == 0 {
            // RotWord
            temp.rotate_left(1);
            // SubWord
            for j in 0..4 {
                temp[j] = sbox_lookup(temp[j]);
            }
            temp[0] ^= RCON[i / NK];
        }
        for j in 0..4 {
            w[4 * i + j] = w[4 * (i - NK) + j] ^ temp[j];
        }
    }
}
