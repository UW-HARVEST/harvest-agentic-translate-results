use crate::aes::{NB, NK, NR, RCON, SBOX};

/// XORs the round key into the state matrix.
pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

/// AES-128 key expansion (FIPS-197 standard) operating on a flat `w` buffer
/// laid out one 32-bit word at a time (4 bytes per word, total of `Nb*(Nr+1)`
/// words).
pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Zero out the buffer first to mirror the C implementation's `memset`.
    for byte in w.iter_mut() {
        *byte = 0;
    }

    // Copy the original key as the first Nk words.
    for i in 0..(4 * NK) {
        w[i] = key[i];
    }

    // Generate the remaining words.
    let total_words = NB * (NR + 1);
    let mut i = NK;
    while i < total_words {
        let mut temp = [0u8; 4];
        // Load w[i-1].
        for k in 0..4 {
            temp[k] = w[4 * (i - 1) + k];
        }

        if i % NK == 0 {
            // RotWord: rotate left by one byte.
            let t0 = temp[0];
            temp[0] = temp[1];
            temp[1] = temp[2];
            temp[2] = temp[3];
            temp[3] = t0;
            // SubWord: apply S-box to each byte.
            for k in 0..4 {
                temp[k] = SBOX[temp[k] as usize];
            }
            // XOR with round constant.
            temp[0] ^= RCON[i / NK];
        } else if NK > 6 && (i % NK == 4) {
            for k in 0..4 {
                temp[k] = SBOX[temp[k] as usize];
            }
        }

        // w[i] = w[i - Nk] XOR temp.
        for k in 0..4 {
            w[4 * i + k] = w[4 * (i - NK) + k] ^ temp[k];
        }

        i += 1;
    }
}
