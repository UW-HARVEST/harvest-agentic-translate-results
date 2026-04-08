use crate::aes::NB;
use crate::cipher_utils::g_mult;

pub fn columns(state: &mut [[u8; NB]; 4]) {
    let mut temp = [[0u8; NB]; 4];
    for c in 0..NB {
        temp[0][c] = g_mult(0x02, state[0][c]) ^ g_mult(0x03, state[1][c]) ^ state[2][c] ^ state[3][c];
        temp[1][c] = state[0][c] ^ g_mult(0x02, state[1][c]) ^ g_mult(0x03, state[2][c]) ^ state[3][c];
        temp[2][c] = state[0][c] ^ state[1][c] ^ g_mult(0x02, state[2][c]) ^ g_mult(0x03, state[3][c]);
        temp[3][c] = g_mult(0x03, state[0][c]) ^ state[1][c] ^ state[2][c] ^ g_mult(0x02, state[3][c]);
    }
    *state = temp;
}

pub fn inv_columns(state: &mut [[u8; NB]; 4]) {
    let mut temp = [[0u8; NB]; 4];
    for c in 0..NB {
        temp[0][c] = g_mult(0x0e, state[0][c]) ^ g_mult(0x0b, state[1][c]) ^ g_mult(0x0d, state[2][c]) ^ g_mult(0x09, state[3][c]);
        temp[1][c] = g_mult(0x09, state[0][c]) ^ g_mult(0x0e, state[1][c]) ^ g_mult(0x0b, state[2][c]) ^ g_mult(0x0d, state[3][c]);
        temp[2][c] = g_mult(0x0d, state[0][c]) ^ g_mult(0x09, state[1][c]) ^ g_mult(0x0e, state[2][c]) ^ g_mult(0x0b, state[3][c]);
        temp[3][c] = g_mult(0x0b, state[0][c]) ^ g_mult(0x0d, state[1][c]) ^ g_mult(0x09, state[2][c]) ^ g_mult(0x0e, state[3][c]);
    }
    *state = temp;
}
