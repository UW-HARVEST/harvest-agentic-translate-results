use crate::aes::{NB, NR};
use crate::cipher_utils::{sub, inv_sub};
use crate::aes::{shift, inv_shift};
use crate::matrix::{columns, inv_columns};
use crate::keys::add;

pub fn cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..4 * NB {
        state[i % 4][i / 4] = in_data[i];
    }

    // AddRoundKey with round 0 key - but the C code casts w directly as [4][Nb]
    // w[0..15] laid out as bytes, cast to [4][4] means row-major: state[i][j] ^= w[i*4+j]
    // But the standard AES uses column-major. Let me check the C code...
    // C: Add(state, (uint8_t(*)[Nb])&w[0]) - this casts w as a [4][4] array
    // So w[0..3] = row 0, w[4..7] = row 1, etc.
    // This means round_key[i][j] = w[round*16 + i*4 + j]
    let mut rk = [[0u8; NB]; 4];
    for i in 0..4 {
        for j in 0..NB {
            rk[i][j] = w[i * NB + j];
        }
    }
    add(&mut state, &rk);

    for round in 1..NR {
        sub(&mut state);
        shift(&mut state);
        columns(&mut state);
        let mut rk = [[0u8; NB]; 4];
        for i in 0..4 {
            for j in 0..NB {
                rk[i][j] = w[round * 4 * NB + i * NB + j];
            }
        }
        add(&mut state, &rk);
    }

    sub(&mut state);
    shift(&mut state);
    let mut rk = [[0u8; NB]; 4];
    for i in 0..4 {
        for j in 0..NB {
            rk[i][j] = w[NR * 4 * NB + i * NB + j];
        }
    }
    add(&mut state, &rk);

    for i in 0..4 * NB {
        out[i] = state[i % 4][i / 4];
    }
}

pub fn inv_cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..4 * NB {
        state[i % 4][i / 4] = in_data[i];
    }

    let mut rk = [[0u8; NB]; 4];
    for i in 0..4 {
        for j in 0..NB {
            rk[i][j] = w[NR * 4 * NB + i * NB + j];
        }
    }
    add(&mut state, &rk);

    for round in (1..NR).rev() {
        inv_shift(&mut state);
        inv_sub(&mut state);
        let mut rk = [[0u8; NB]; 4];
        for i in 0..4 {
            for j in 0..NB {
                rk[i][j] = w[round * 4 * NB + i * NB + j];
            }
        }
        add(&mut state, &rk);
        inv_columns(&mut state);
    }

    inv_shift(&mut state);
    inv_sub(&mut state);
    let mut rk = [[0u8; NB]; 4];
    for i in 0..4 {
        for j in 0..NB {
            rk[i][j] = w[i * NB + j];
        }
    }
    add(&mut state, &rk);

    for i in 0..4 * NB {
        out[i] = state[i % 4][i / 4];
    }
}
