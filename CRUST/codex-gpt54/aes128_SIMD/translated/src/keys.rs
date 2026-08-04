use crate::aes::{NB, NR, NK};
use crate::cipher_utils::SBOX;

const RCON: [u8; 11] = [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36];

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}
pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    w.fill(0);
    w[..(4 * NK)].copy_from_slice(key);

    let total_words = NB * (NR + 1);
    for i in NK..total_words {
        let mut temp = [
            w[4 * (i - 1)],
            w[4 * (i - 1) + 1],
            w[4 * (i - 1) + 2],
            w[4 * (i - 1) + 3],
        ];

        if i % NK == 0 {
            temp.rotate_left(1);
            for byte in &mut temp {
                *byte = SBOX[*byte as usize];
            }
            temp[0] ^= RCON[i / NK];
        }

        for j in 0..4 {
            w[4 * i + j] = w[4 * (i - NK) + j] ^ temp[j];
        }
    }
}
