use crate::aes::{NB, NR};
use crate::keys::add;
use crate::matrix::{columns, inv_columns};
use crate::aes::{shift, inv_shift};
use crate::cipher_utils::{sub, inv_sub};

fn round_key_from_w(w: &[u8; 4 * NB * (NR + 1)], round: usize) -> [[u8; NB]; 4] {
    // The expanded key buffer stores one 32-bit word per column in row-major
    // form where `w[round*16 + col*4 + row]` is the byte at (row, col) of the
    // round-key matrix for the given round.
    let base = round * 4 * NB;
    let mut rk = [[0u8; NB]; 4];
    for r in 0..4 {
        for c in 0..NB {
            rk[r][c] = w[base + c * 4 + r];
        }
    }
    rk
}

pub fn cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }

    add(&mut state, &round_key_from_w(w, 0));

    for round in 1..NR {
        sub(&mut state);
        shift(&mut state);
        columns(&mut state);
        add(&mut state, &round_key_from_w(w, round));
    }

    sub(&mut state);
    shift(&mut state);
    add(&mut state, &round_key_from_w(w, NR));

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}

pub fn inv_cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }

    add(&mut state, &round_key_from_w(w, NR));

    let mut round = NR - 1;
    while round > 0 {
        inv_shift(&mut state);
        inv_sub(&mut state);
        add(&mut state, &round_key_from_w(w, round));
        inv_columns(&mut state);
        round -= 1;
    }

    inv_shift(&mut state);
    inv_sub(&mut state);
    add(&mut state, &round_key_from_w(w, 0));

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}
