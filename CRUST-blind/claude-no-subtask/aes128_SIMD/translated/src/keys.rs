use crate::aes::{NB, NR, NK, SBOX, RCON};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

// Helper: simulate _mm_storeu_si128 of 16 bytes at offset `pos` in buffer `w`.
// If `pos + 16 > w.len()`, only writes what fits (but in the C code, this would be UB
// — the loop bounds make it not occur in practice for the configured Nk/Nr).
fn store16(w: &mut [u8], pos: usize, src: &[u8; 16]) {
    let end = (pos + 16).min(w.len());
    if pos < w.len() {
        w[pos..end].copy_from_slice(&src[..end - pos]);
    }
}

// Helper: simulate _mm_loadu_si128 of 16 bytes from offset `pos` in `w`.
// If pos + 16 > w.len(), the remaining bytes are read as 0.
fn load16(w: &[u8], pos: usize) -> [u8; 16] {
    let mut out = [0u8; 16];
    if pos < w.len() {
        let end = (pos + 16).min(w.len());
        out[..end - pos].copy_from_slice(&w[pos..end]);
    }
    out
}

pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Zero out the entire buffer.
    for b in w.iter_mut() {
        *b = 0;
    }

    // Initial fill loop:
    //   for (i = 0; i < Nk * 4; i += 16) {
    //       temp = load(&Key[i]);            // 16 bytes
    //       store(&w[i],     temp);
    //       store(&w[i + 4], temp);
    //       store(&w[i + 8], temp);
    //       store(&w[i + 12], temp);
    //   }
    //
    // For Nk = 4 (AES-128), this loop runs exactly once with i = 0.
    let mut i = 0;
    while i < NK * 4 {
        // Load 16 bytes from key starting at index i. The C code does an unaligned
        // 16-byte load, which for Nk=4 reads exactly key[0..16].
        let mut temp = [0u8; 16];
        let key_end = (i + 16).min(key.len());
        temp[..key_end - i].copy_from_slice(&key[i..key_end]);

        store16(w, i, &temp);
        store16(w, i + 4, &temp);
        store16(w, i + 8, &temp);
        store16(w, i + 12, &temp);

        i += 16;
    }

    // Main expansion loop. i is a 32-bit word index here (not byte index).
    let mut i = NK;
    while i < NB * (NR + 1) {
        // temp = load 16 bytes from w[4*(i-1)]
        let mut temp = load16(w, 4 * (i - 1));

        if i % NK == 0 {
            // _mm_shuffle_epi32(temp, _MM_SHUFFLE(3, 0, 1, 2))
            //   imm = 11_00_01_10b = 0xC6
            //   result lane 0 = src lane 2
            //   result lane 1 = src lane 1
            //   result lane 2 = src lane 0
            //   result lane 3 = src lane 3
            let mut shuffled = [0u8; 16];
            shuffled[0..4].copy_from_slice(&temp[8..12]);
            shuffled[4..8].copy_from_slice(&temp[4..8]);
            shuffled[8..12].copy_from_slice(&temp[0..4]);
            shuffled[12..16].copy_from_slice(&temp[12..16]);
            temp = shuffled;

            // SubWord on first 4 bytes
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
            }
            // XOR with Rcon
            temp[0] ^= RCON[i / NK];
        } else if NK > 6 && (i % NK == 4) {
            // SubWord on first 4 bytes (the redundant inner overwrites in C
            // are no-ops in effect since SBOX is idempotent? No, SBOX is not
            // idempotent — but the `for j in 0..4` loop in the C code applies
            // sbox to indices [j, j+1, j+2, j+3] for each j, which means
            // bytes 1, 2, 3 get sbox'd 4 times. We need to replicate this exactly.
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
                temp[j + 1] = SBOX[temp[j + 1] as usize];
                temp[j + 2] = SBOX[temp[j + 2] as usize];
                temp[j + 3] = SBOX[temp[j + 3] as usize];
            }
        }

        // XOR with w[4*(i-Nk) .. 4*(i-Nk)+16]
        let w_i_nk = load16(w, 4 * (i - NK));
        for j in 0..16 {
            temp[j] ^= w_i_nk[j];
        }

        // Store at w[4*i]
        store16(w, 4 * i, &temp);

        i += 4;
    }
}
