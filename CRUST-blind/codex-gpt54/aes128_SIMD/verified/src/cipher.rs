use crate::aes::{inv_shift, shift};
use crate::aes::{NB, NR};
use crate::cipher_utils::{inv_sub, sub};
use crate::keys::add;
use crate::matrix::{columns, inv_columns};

pub fn cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }

    let mut key = round_key(w, 0);
    add(&mut state, &key);
    for round in 1..NR {
        sub(&mut state);
        shift(&mut state);
        columns(&mut state);
        key = round_key(w, round);
        add(&mut state, &key);
    }
    sub(&mut state);
    shift(&mut state);
    key = round_key(w, NR);
    add(&mut state, &key);

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}
pub fn inv_cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }

    let mut key = round_key(w, NR);
    add(&mut state, &key);
    for round in (1..NR).rev() {
        inv_shift(&mut state);
        inv_sub(&mut state);
        key = round_key(w, round);
        add(&mut state, &key);
        inv_columns(&mut state);
    }
    inv_shift(&mut state);
    inv_sub(&mut state);
    key = round_key(w, 0);
    add(&mut state, &key);

    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}

fn round_key(w: &[u8; 4 * NB * (NR + 1)], round: usize) -> [[u8; NB]; 4] {
    let start = round * 4 * NB;
    let mut key = [[0u8; NB]; 4];
    for row in 0..4 {
        key[row].copy_from_slice(&w[(start + row * NB)..(start + (row + 1) * NB)]);
    }
    key
}
