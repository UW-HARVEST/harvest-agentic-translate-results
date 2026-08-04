use crate::aes::{NB, NK, NR, RCON, SBOX};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

/// AES key expansion. Produces a key schedule in `w` such that, for round
/// `r`, the bytes `w[r*16 + row*4 + col]` correspond to the standard-AES
/// round-key matrix entry at (row, col). This is a transposed layout vs.
/// the typical "sequence of 4-byte words" view, which makes it convenient
/// for the C-style `add` function (which does `state[i][j] ^= rk[i][j]`)
/// to produce correct results when the state is loaded with
/// `state[i % 4][i / 4] = in[i]`.
pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Compute the standard AES key schedule into a temporary array of words.
    let total_words = NB * (NR + 1);
    let mut w_words = [[0u8; 4]; 4 * (NR + 1)];

    // The first Nk words are the cipher key itself.
    for k in 0..NK {
        for b in 0..4 {
            w_words[k][b] = key[k * 4 + b];
        }
    }

    // Generate the remaining words.
    for k in NK..total_words {
        let mut temp = w_words[k - 1];
        if k % NK == 0 {
            // RotWord: rotate left by one byte.
            temp = [temp[1], temp[2], temp[3], temp[0]];
            // SubWord.
            for b in 0..4 {
                temp[b] = SBOX[temp[b] as usize];
            }
            // XOR with the round constant.
            temp[0] ^= RCON[k / NK];
        } else if NK > 6 && k % NK == 4 {
            // SubWord (used in AES-256).
            for b in 0..4 {
                temp[b] = SBOX[temp[b] as usize];
            }
        }
        for b in 0..4 {
            w_words[k][b] = w_words[k - NK][b] ^ temp[b];
        }
    }

    // Zero w and lay it out so that w[round*16 + row*4 + col] is the byte at
    // (row, col) of the round-r key matrix in standard AES. Since standard
    // AES stores word `m` as the column `m mod Nb` of round `m / Nb`, we
    // have:  byte_at(row, col) of round r = w_words[r * Nb + col][row].
    for byte in w.iter_mut() {
        *byte = 0;
    }
    for round in 0..=NR {
        for col in 0..NB {
            for row in 0..4 {
                w[round * 4 * NB + row * 4 + col] = w_words[round * NB + col][row];
            }
        }
    }
}
