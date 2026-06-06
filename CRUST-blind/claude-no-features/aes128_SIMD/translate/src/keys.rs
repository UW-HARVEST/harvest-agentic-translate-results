use crate::aes::{NB, NK, NR, RCON, SBOX};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Zero the output buffer.
    for byte in w.iter_mut() {
        *byte = 0;
    }

    // Initial fill mimicking the C code's overlapping 16-byte SSE stores.
    // For Nk = 4 the outer loop runs once at i = 0, producing 4 stores of
    // key[0..16] at offsets 0, 4, 8, 12 (each store writes 16 bytes).
    let total_len = w.len();
    let mut i = 0usize;
    while i < NK * 4 {
        for off in [0usize, 4, 8, 12] {
            for k in 0..16 {
                let idx = i + off + k;
                if idx < total_len {
                    w[idx] = key[k];
                }
            }
        }
        i += 16;
    }

    // Main expansion loop, also mirrored from the C version.
    let mut i = NK;
    while i < NB * (NR + 1) {
        // Load 16 bytes from w[4*(i-1) ..]
        let mut temp = [0u8; 16];
        let src_base = 4 * (i - 1);
        for k in 0..16 {
            let idx = src_base + k;
            if idx < total_len {
                temp[k] = w[idx];
            }
        }

        if i % NK == 0 {
            // _mm_shuffle_epi32(temp, _MM_SHUFFLE(3,0,1,2))
            // result dwords are temp[2], temp[1], temp[0], temp[3]
            let mut shuffled = [0u8; 16];
            shuffled[0..4].copy_from_slice(&temp[8..12]);
            shuffled[4..8].copy_from_slice(&temp[4..8]);
            shuffled[8..12].copy_from_slice(&temp[0..4]);
            shuffled[12..16].copy_from_slice(&temp[12..16]);
            temp = shuffled;

            // SubWord on the first 4 bytes only (matches C inner loop).
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
            }
            temp[0] ^= RCON[i / NK];
        } else if NK > 6 && i % NK == 4 {
            // Dead code for NK = 4 but kept for fidelity.
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
            }
        }

        // Load w[4*(i-Nk) ..]
        let mut w_i_nk = [0u8; 16];
        let src = 4 * (i - NK);
        for k in 0..16 {
            let idx = src + k;
            if idx < total_len {
                w_i_nk[k] = w[idx];
            }
        }

        // XOR
        for k in 0..16 {
            temp[k] ^= w_i_nk[k];
        }

        // Store 16 bytes at w[4*i ..]
        let dst = 4 * i;
        for k in 0..16 {
            let idx = dst + k;
            if idx < total_len {
                w[idx] = temp[k];
            }
        }

        i += 4;
    }
}
