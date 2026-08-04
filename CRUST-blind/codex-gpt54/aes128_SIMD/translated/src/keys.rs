use crate::aes::{NB, NK, NR};
use crate::cipher_utils::SBOX;

const RCON: [u8; 11] = [
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36,
];

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}
pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    w.fill(0);

    let mut i = 0usize;
    while i < (NK * 4) {
        let temp = load_16(key, i);
        store_16(w, i, &temp);
        store_16(w, i + 4, &temp);
        store_16(w, i + 8, &temp);
        store_16(w, i + 12, &temp);
        i += 16;
    }

    i = NK;
    while i < NB * (NR + 1) {
        let mut temp = load_16(w, 4 * (i - 1));
        if i % NK == 0 {
            temp = shuffle_lanes_3_0_1_2(temp);
            for byte in temp.iter_mut().take(4) {
                *byte = SBOX[*byte as usize];
            }
            temp[0] ^= RCON[i / NK];
        } else if NK > 6 && (i % NK == 4) {
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
                temp[j + 1] = SBOX[temp[j + 1] as usize];
                temp[j + 2] = SBOX[temp[j + 2] as usize];
                temp[j + 3] = SBOX[temp[j + 3] as usize];
            }
        }

        let w_i_nk = load_16(w, 4 * (i - NK));
        for j in 0..16 {
            temp[j] ^= w_i_nk[j];
        }
        store_16(w, 4 * i, &temp);
        i += 4;
    }
}

fn load_16(input: &[u8], start: usize) -> [u8; 16] {
    let mut out = [0u8; 16];
    out.copy_from_slice(&input[start..(start + 16)]);
    out
}

fn store_16(output: &mut [u8], start: usize, value: &[u8; 16]) {
    output[start..(start + 16)].copy_from_slice(value);
}

fn shuffle_lanes_3_0_1_2(bytes: [u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&bytes[8..12]);
    out[4..8].copy_from_slice(&bytes[4..8]);
    out[8..12].copy_from_slice(&bytes[0..4]);
    out[12..16].copy_from_slice(&bytes[12..16]);
    out
}
