use crate::aes::{NB, NK, NR, RCON, SBOX};

/// AddRoundKey: XOR each byte of the state with the corresponding round-key byte.
pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

/// AES KeyExpansion (FIPS-197). Expands a 4*Nk byte key into a 4*Nb*(Nr+1) byte schedule.
pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Copy the original key into the first Nk*4 bytes of w.
    w[..4 * NK].copy_from_slice(key);

    let total_words = NB * (NR + 1);
    let mut i = NK;
    while i < total_words {
        // temp = previous word
        let mut temp = [
            w[4 * (i - 1)],
            w[4 * (i - 1) + 1],
            w[4 * (i - 1) + 2],
            w[4 * (i - 1) + 3],
        ];

        if i % NK == 0 {
            // RotWord: cyclic left shift by 1 byte
            let t0 = temp[0];
            temp[0] = temp[1];
            temp[1] = temp[2];
            temp[2] = temp[3];
            temp[3] = t0;
            // SubWord
            for b in temp.iter_mut() {
                *b = SBOX[*b as usize];
            }
            // XOR with round constant
            temp[0] ^= RCON[i / NK];
        } else if NK > 6 && i % NK == 4 {
            // For AES-256: SubWord on the temp
            for b in temp.iter_mut() {
                *b = SBOX[*b as usize];
            }
        }

        // w[i] = w[i - Nk] XOR temp
        let prev_idx = 4 * (i - NK);
        let cur_idx = 4 * i;
        for k in 0..4 {
            w[cur_idx + k] = w[prev_idx + k] ^ temp[k];
        }

        i += 1;
    }
}
