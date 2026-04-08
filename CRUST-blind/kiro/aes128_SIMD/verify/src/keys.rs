use crate::aes::{NB, NR, NK};
use crate::cipher_utils::{SBOX, RCON};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Replicate C code's SIMD init: stores 16-byte key at w[0], w[4], w[8], w[12]
    // This means each 4-byte word in w[0..15] gets key[0..3]
    // and w[16..27] gets key[4..15]
    w.fill(0);
    for offset in &[0usize, 4, 8, 12] {
        for b in 0..16 {
            if offset + b < w.len() {
                w[offset + b] = key[b];
            }
        }
    }

    let mut i = NK; // i = 4, steps by 4
    while i < NB * (NR + 1) {
        // temp = 16 bytes from w[4*(i-1)]
        let base = 4 * (i - 1);
        let mut temp = [0u8; 16];
        for b in 0..16 {
            if base + b < w.len() {
                temp[b] = w[base + b];
            }
        }

        if i % NK == 0 {
            // _mm_shuffle_epi32(temp, _MM_SHUFFLE(3, 0, 1, 2))
            // Reorders 32-bit dwords: [dw0,dw1,dw2,dw3] -> [dw2,dw1,dw0,dw3]
            let mut shuffled = [0u8; 16];
            // dst dword 0 = src dword 2
            shuffled[0..4].copy_from_slice(&temp[8..12]);
            // dst dword 1 = src dword 1
            shuffled[4..8].copy_from_slice(&temp[4..8]);
            // dst dword 2 = src dword 0
            shuffled[8..12].copy_from_slice(&temp[0..4]);
            // dst dword 3 = src dword 3
            shuffled[12..16].copy_from_slice(&temp[12..16]);
            temp = shuffled;

            // SubWord on first 4 bytes only
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
            }
            temp[0] ^= RCON[i / NK];
        }

        // w_i_Nk = 16 bytes from w[4*(i-NK)]
        let nk_base = 4 * (i - NK);
        let mut w_i_nk = [0u8; 16];
        for b in 0..16 {
            if nk_base + b < w.len() {
                w_i_nk[b] = w[nk_base + b];
            }
        }

        // temp = temp XOR w_i_nk
        for b in 0..16 {
            temp[b] ^= w_i_nk[b];
        }

        // Store 16 bytes at w[4*i]
        let store_base = 4 * i;
        for b in 0..16 {
            if store_base + b < w.len() {
                w[store_base + b] = temp[b];
            }
        }

        i += 4;
    }
}
