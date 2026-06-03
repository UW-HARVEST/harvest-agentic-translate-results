use crate::aes::{NB, NK, NR, RCON, SBOX};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Standard AES key expansion. Mirrors the structure of the C code:
    //   - copy the original key into the first Nk words of `w`
    //   - for each subsequent word `i`, take the previous word and, when
    //     `i % Nk == 0`, rotate it left by one byte, apply S-box, and XOR
    //     with the round constant. Then XOR with `w[i - Nk]`.
    for byte in w.iter_mut() {
        *byte = 0;
    }

    // Copy the original key into the first Nk 32-bit words.
    w[..(4 * NK)].copy_from_slice(&key[..(4 * NK)]);

    let mut i = NK;
    while i < NB * (NR + 1) {
        // Read the previous word (4 bytes).
        let mut temp = [
            w[4 * (i - 1)],
            w[4 * (i - 1) + 1],
            w[4 * (i - 1) + 2],
            w[4 * (i - 1) + 3],
        ];

        if i % NK == 0 {
            // RotWord: rotate left by one byte.
            let t = temp[0];
            temp[0] = temp[1];
            temp[1] = temp[2];
            temp[2] = temp[3];
            temp[3] = t;
            // SubWord: apply S-box.
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
            }
            // XOR with round constant.
            temp[0] ^= RCON[i / NK];
        }

        // w[i] = w[i - Nk] XOR temp
        for k in 0..4 {
            w[4 * i + k] = w[4 * (i - NK) + k] ^ temp[k];
        }

        i += 1;
    }
}
