use crate::aes::{shift, inv_shift, NB, NR};
use crate::cipher_utils::{sub, inv_sub};
use crate::keys::add;
use crate::matrix::{columns, inv_columns};

fn round_key_from_w(w: &[u8; 4 * NB * (NR + 1)], offset: usize) -> [[u8; NB]; 4] {
    let mut rk = [[0u8; NB]; 4];
    // The C reinterprets w starting at offset as a 4xNb byte matrix. Layout
    // is a contiguous row-major 4xNb block of bytes.
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
    let rk0 = round_key_from_w(w, 0);
    add(&mut state, &rk0);
    for round in 1..NR {
        sub(&mut state);
        shift(&mut state);
        columns(&mut state);
        let rk = round_key_from_w(w, round * 4 * NB);
        add(&mut state, &rk);
    }
    sub(&mut state);
    shift(&mut state);
    let rk_last = round_key_from_w(w, NR * 4 * NB);
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
    let rk_last = round_key_from_w(w, NR * 4 * NB);
    add(&mut state, &rk_last);
    let mut round = NR - 1;
    while round > 0 {
        inv_shift(&mut state);
        inv_sub(&mut state);
        let rk = round_key_from_w(w, round * 4 * NB);
        add(&mut state, &rk);
        inv_columns(&mut state);
        round -= 1;
    }
    inv_shift(&mut state);
    inv_sub(&mut state);
    let rk0 = round_key_from_w(w, 0);
    add(&mut state, &rk0);
    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}
