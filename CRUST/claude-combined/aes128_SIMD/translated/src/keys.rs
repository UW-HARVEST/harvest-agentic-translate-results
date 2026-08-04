use crate::aes::{NB, NK, NR, RCON, SBOX};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

/// Performs AES key expansion. The expanded key is stored in `w` in a
/// transposed layout so that the cipher's `Add` operation
/// (which reads `RoundKey[i][j] = w[offset + i*4 + j]`)
/// matches standard AES, where `state[r][c]` is XORed with byte `r + 4c`
/// of the round key.
pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Standard AES key expansion in 4-byte words.
    let total_words = NB * (NR + 1);
    let mut words: Vec<[u8; 4]> = vec![[0u8; 4]; total_words];
    for i in 0..NK {
        words[i] = [key[4 * i], key[4 * i + 1], key[4 * i + 2], key[4 * i + 3]];
    }
    for i in NK..total_words {
        let mut temp = words[i - 1];
        if i % NK == 0 {
            // RotWord: rotate bytes left by 1.
            temp = [temp[1], temp[2], temp[3], temp[0]];
            // SubWord: apply S-box to each byte.
            for k in 0..4 {
                temp[k] = SBOX[temp[k] as usize];
            }
            // XOR with round constant.
            temp[0] ^= RCON[i / NK];
        } else if NK > 6 && i % NK == 4 {
            for k in 0..4 {
                temp[k] = SBOX[temp[k] as usize];
            }
        }
        for k in 0..4 {
            words[i][k] = words[i - NK][k] ^ temp[k];
        }
    }

    // Now `words[r*4 + j]` contains the j-th 4-byte word of round-key r.
    // Standard AES byte k of round key r is: words[r*4 + k/4][k%4].
    //
    // The cipher reads `RoundKey[i][j] = w[r*16 + i*4 + j]`, and we need
    // this to equal byte (i + 4j) of round key r.
    // byte (i + 4j) of round key r = words[r*4 + (i + 4j)/4][(i + 4j) % 4]
    //                              = words[r*4 + j][i]   (since 0 <= i < 4)
    // So set w[r*16 + i*4 + j] = words[r*4 + j][i].
    for r in 0..=NR {
        for i in 0..4 {
            for j in 0..4 {
                w[r * 16 + i * 4 + j] = words[r * 4 + j][i];
            }
        }
    }
}
