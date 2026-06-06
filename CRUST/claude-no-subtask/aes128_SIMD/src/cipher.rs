use crate::aes::{NB, NR};
use crate::cipher_utils::{inv_sub, sub};
use crate::keys::add;
use crate::matrix::{columns, inv_columns};
use crate::aes::{inv_shift, shift};

/// Build a state matrix from the input bytes (column-major: state[row][col]).
fn load_state(in_data: &[u8; 4 * NB]) -> [[u8; NB]; 4] {
    let mut state = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        state[i % 4][i / 4] = in_data[i];
    }
    state
}

/// Write the state matrix back to a flat output buffer (column-major).
fn store_state(state: &[[u8; NB]; 4], out: &mut [u8; 4 * NB]) {
    for i in 0..(4 * NB) {
        out[i] = state[i % 4][i / 4];
    }
}

/// Extract a 4x4 round key from the expanded key schedule starting at `offset` bytes.
fn round_key(w: &[u8; 4 * NB * (NR + 1)], offset: usize) -> [[u8; NB]; 4] {
    let mut rk = [[0u8; NB]; 4];
    for i in 0..(4 * NB) {
        rk[i % 4][i / 4] = w[offset + i];
    }
    rk
}

/// AES Cipher: encrypts a 16-byte block using the expanded key schedule.
pub fn cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = load_state(in_data);

    let rk0 = round_key(w, 0);
    add(&mut state, &rk0);

    for round in 1..NR {
        sub(&mut state);
        shift(&mut state);
        columns(&mut state);
        let rk = round_key(w, round * 4 * NB);
        add(&mut state, &rk);
    }

    sub(&mut state);
    shift(&mut state);
    let rk_final = round_key(w, NR * 4 * NB);
    add(&mut state, &rk_final);

    store_state(&state, out);
}

/// AES Inverse Cipher: decrypts a 16-byte block using the expanded key schedule.
pub fn inv_cipher(in_data: &[u8; 4 * NB], out: &mut [u8; 4 * NB], w: &[u8; 4 * NB * (NR + 1)]) {
    let mut state = load_state(in_data);

    let rk_final = round_key(w, NR * 4 * NB);
    add(&mut state, &rk_final);

    for round in (1..NR).rev() {
        inv_shift(&mut state);
        inv_sub(&mut state);
        let rk = round_key(w, round * 4 * NB);
        add(&mut state, &rk);
        inv_columns(&mut state);
    }

    inv_shift(&mut state);
    inv_sub(&mut state);
    let rk0 = round_key(w, 0);
    add(&mut state, &rk0);

    store_state(&state, out);
}
