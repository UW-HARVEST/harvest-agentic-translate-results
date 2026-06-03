use crate::aes::{shift, inv_shift, NB, NR};
use crate::cipher_utils::{inv_sub, sub};
use crate::keys::add;
use crate::matrix::{columns, inv_columns};

fn round_key(w: &[u8; 4 * NB * (NR + 1)], round: usize) -> [[u8; NB]; 4] {
    let mut rk = [[0u8; NB]; 4];
    let off = round * 4 * NB;
    for i in 0..4 {
        for j in 0..NB {
            rk[i][j] = w[off + i * NB + j];
        }
    }
    rk
}

pub fn cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }

    let rk0 = round_key(w, 0);
    add(&mut state, &rk0);

    for round in 1..NR {
        sub(&mut state);
        shift(&mut state);
        columns(&mut state);
        let rk = round_key(w, round);
        add(&mut state, &rk);
    }

    sub(&mut state);
    shift(&mut state);
    let rk_last = round_key(w, NR);
    add(&mut state, &rk_last);

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}

pub fn inv_cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }

    let rk_last = round_key(w, NR);
    add(&mut state, &rk_last);

    for round in (1..NR).rev() {
        inv_shift(&mut state);
        inv_sub(&mut state);
        let rk = round_key(w, round);
        add(&mut state, &rk);
        inv_columns(&mut state);
    }

    inv_shift(&mut state);
    inv_sub(&mut state);
    let rk0 = round_key(w, 0);
    add(&mut state, &rk0);

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}
