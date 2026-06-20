use crate::aes::{NB, NR};
use crate::aes::{inv_shift, shift};
use crate::cipher_utils::{inv_sub, sub};
use crate::keys::add;
use crate::matrix::{columns, inv_columns};
pub fn cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }

    let mut current_round_key = load_round_key(w, 0);
    add(&mut state, &current_round_key);
    for round in 1..NR {
        sub(&mut state);
        shift(&mut state);
        columns(&mut state);
        current_round_key = load_round_key(w, round);
        add(&mut state, &current_round_key);
    }
    sub(&mut state);
    shift(&mut state);
    current_round_key = load_round_key(w, NR);
    add(&mut state, &current_round_key);

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}
pub fn inv_cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }

    let mut current_round_key = load_round_key(w, NR);
    add(&mut state, &current_round_key);
    for round in (1..NR).rev() {
        inv_shift(&mut state);
        inv_sub(&mut state);
        current_round_key = load_round_key(w, round);
        add(&mut state, &current_round_key);
        inv_columns(&mut state);
    }
    inv_shift(&mut state);
    inv_sub(&mut state);
    current_round_key = load_round_key(w, 0);
    add(&mut state, &current_round_key);

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}

fn load_round_key(w: &[u8; 4 * NB * (NR + 1)], round: usize) -> [[u8; NB]; 4] {
    let mut round_key = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        round_key[i % 4][i / 4] = w[round * 4 * NB + i];
    }
    round_key
}
