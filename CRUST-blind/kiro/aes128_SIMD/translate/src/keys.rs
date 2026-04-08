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
    // Replicate C SIMD behavior exactly.
    // Init: 16-byte overlapping stores fill w[0..27]
    *w = [0u8; 4 * NB * (NR + 1)];
    for base in (0..4 * NK).step_by(16) {
        let chunk: [u8; 16] = {
            let mut c = [0u8; 16];
            let end = (base + 16).min(4 * NK);
            c[..end - base].copy_from_slice(&key[base..end]);
            c
        };
        for off in (0..16).step_by(4) {
            let dst = base + off;
            if dst + 16 <= w.len() {
                w[dst..dst + 16].copy_from_slice(&chunk);
            }
        }
    }

    // Main loop: i counts words, steps by 4 (SIMD processes 16 bytes at a time)
    let mut i = NK;
    while i < NB * (NR + 1) {
        // Load 16 bytes from w[4*(i-1)]
        let src = 4 * (i - 1);
        let mut temp = [0u8; 16];
        temp.copy_from_slice(&w[src..src + 16]);

        if i % NK == 0 {
            // _mm_shuffle_epi32 with _MM_SHUFFLE(3,0,1,2):
            // dst[0]=src[2], dst[1]=src[1], dst[2]=src[0], dst[3]=src[3] (32-bit lanes)
            let mut shuffled = [0u8; 16];
            shuffled[0..4].copy_from_slice(&temp[8..12]);
            shuffled[4..8].copy_from_slice(&temp[4..8]);
            shuffled[8..12].copy_from_slice(&temp[0..4]);
            shuffled[12..16].copy_from_slice(&temp[12..16]);
            temp = shuffled;
            // SubBytes only first 4 bytes
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
            }
            temp[0] ^= RCON[i / NK];
        }

        // XOR with 16 bytes from w[4*(i-NK)]
        let nk_src = 4 * (i - NK);
        for j in 0..16 {
            temp[j] ^= w[nk_src + j];
        }

        // Store 16 bytes to w[4*i]
        let dst = 4 * i;
        if dst + 16 <= w.len() {
            w[dst..dst + 16].copy_from_slice(&temp);
        } else {
            let remaining = w.len() - dst;
            w[dst..dst + remaining].copy_from_slice(&temp[..remaining]);
        }

        i += 4;
    }
}
