use crate::aes::{shift, inv_shift, NB, NR};
use crate::cipher_utils::{sub, inv_sub};
use crate::matrix::{columns, inv_columns};
use crate::keys::add;

fn round_key_from_w(w: &[u8; 4 * NB * (NR + 1)], offset: usize) -> [[u8; NB]; 4] {
    let mut rk = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        rk[i % 4][i / 4] = w[offset + i];
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
        add(&mut state, &round_key_from_w(w, round * 4 * NB));
    }

    sub(&mut state);
    shift(&mut state);
    add(&mut state, &round_key_from_w(w, NR * 4 * NB));

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}

pub fn inv_cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }

    add(&mut state, &round_key_from_w(w, NR * 4 * NB));

    for round in (1..NR).rev() {
        inv_shift(&mut state);
        inv_sub(&mut state);
        add(&mut state, &round_key_from_w(w, round * 4 * NB));
        inv_columns(&mut state);
    }

    inv_shift(&mut state);
    inv_sub(&mut state);
    add(&mut state, &round_key_from_w(w, 0));

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}
