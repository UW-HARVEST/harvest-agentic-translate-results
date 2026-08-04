use crate::aes::{NB, NR};
use crate::cipher_utils::{sub, inv_sub};
use crate::aes::{shift, inv_shift};
use crate::matrix::{columns, inv_columns};
use crate::keys::add;

fn slice_round_key(w: &[u8; 4 * NB * (NR + 1)], offset: usize) -> [[u8; NB]; 4] {
    let mut rk = [[0u8; NB]; 4];
    for i in 0..4 {
        for j in 0..NB {
            rk[i][j] = w[offset + i * NB + j];
        }
    }
    rk
}

pub fn cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }
    let rk = slice_round_key(w, 0);
    add(&mut state, &rk);

    for round in 1..NR {
        sub(&mut state);
        shift(&mut state);
        columns(&mut state);
        let rk = slice_round_key(w, round * 4 * NB);
        add(&mut state, &rk);
    }
    sub(&mut state);
    shift(&mut state);
    let rk = slice_round_key(w, NR * 4 * NB);
    add(&mut state, &rk);

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}

pub fn inv_cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }
    let rk = slice_round_key(w, NR * 4 * NB);
    add(&mut state, &rk);

    for round in (1..NR).rev() {
        inv_shift(&mut state);
        inv_sub(&mut state);
        let rk = slice_round_key(w, round * 4 * NB);
        add(&mut state, &rk);
        inv_columns(&mut state);
    }
    inv_shift(&mut state);
    inv_sub(&mut state);
    let rk = slice_round_key(w, 0);
    add(&mut state, &rk);

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}
