use crate::aes::{NB, NR, NK, SBOX, RCON};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // zero w
    for byte in w.iter_mut() {
        *byte = 0;
    }

    // The C code does:
    //   for (i = 0; i < Nk * 4; i += 16)
    //       memcpy 16 bytes from key[i] into w[i], w[i+4], w[i+8], w[i+12]
    // With Nk=4, the loop runs once with i=0, so:
    //   w[0..16]  = key[0..16]
    //   w[4..20]  = key[0..16]
    //   w[8..24]  = key[0..16]
    //   w[12..28] = key[0..16]
    //
    // The end result in w[0..28]:
    //   - w[0..16] = key[0..16] (overwritten last by w[12..28] for indices 12..16)
    //   Actually each store writes 16 bytes, the later overwrites earlier.
    //   Final value of w[0..28]:
    //     w[0..4]  = key[0..4]    (only first store touched it)
    //     w[4..8]  = key[0..4]    (second store)
    //     w[8..12] = key[0..4]    (third store)
    //     w[12..16]= key[0..4]    (fourth store)
    //     w[16..20]= key[4..8]    (fourth store)
    //     w[20..24]= key[8..12]   (fourth store)
    //     w[24..28]= key[12..16]  (fourth store)

    // Replicate that exactly:
    for i in (0..NK * 4).step_by(16) {
        // simulate four 16-byte stores at offsets i, i+4, i+8, i+12
        // each store copies key[i..i+16] into the destination
        for offset in &[0usize, 4, 8, 12] {
            let dst = i + *offset;
            for k in 0..16 {
                if dst + k < w.len() {
                    w[dst + k] = key[i + k];
                }
            }
        }
    }

    let mut i = NK;
    while i < NB * (NR + 1) {
        // load 16 bytes from w[4*(i-1)..4*(i-1)+16]
        let mut temp = [0u8; 16];
        let load_start = 4 * (i - 1);
        for k in 0..16 {
            if load_start + k < w.len() {
                temp[k] = w[load_start + k];
            }
        }

        if i % NK == 0 {
            // _mm_shuffle_epi32(temp, _MM_SHUFFLE(3, 0, 1, 2))
            // _MM_SHUFFLE(3,0,1,2) = (3<<6)|(0<<4)|(1<<2)|2 = 0xC6
            // Result: dword[0] = src[2], dword[1] = src[1], dword[2] = src[0], dword[3] = src[3]
            let mut shuffled = [0u8; 16];
            // dword 0 <- src dword 2 (bytes 8..12)
            shuffled[0..4].copy_from_slice(&temp[8..12]);
            // dword 1 <- src dword 1 (bytes 4..8)
            shuffled[4..8].copy_from_slice(&temp[4..8]);
            // dword 2 <- src dword 0 (bytes 0..4)
            shuffled[8..12].copy_from_slice(&temp[0..4]);
            // dword 3 <- src dword 3 (bytes 12..16)
            shuffled[12..16].copy_from_slice(&temp[12..16]);
            temp = shuffled;

            // sbox the first 4 bytes
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
            }
            temp[0] ^= RCON[i / NK];
        } else if NK > 6 && (i % NK == 4) {
            // This branch is dead for AES-128 since NK=4, but include it for correctness
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
                temp[j + 1] = SBOX[temp[j + 1] as usize];
                temp[j + 2] = SBOX[temp[j + 2] as usize];
                temp[j + 3] = SBOX[temp[j + 3] as usize];
            }
        }

        // load w[4*(i-Nk)..+16]
        let mut w_i_nk = [0u8; 16];
        let nk_start = 4 * (i - NK);
        for k in 0..16 {
            if nk_start + k < w.len() {
                w_i_nk[k] = w[nk_start + k];
            }
        }

        // XOR
        for k in 0..16 {
            temp[k] ^= w_i_nk[k];
        }

        // store to w[4*i..4*i+16]
        let store_start = 4 * i;
        for k in 0..16 {
            if store_start + k < w.len() {
                w[store_start + k] = temp[k];
            }
        }

        i += 4;
    }
}
