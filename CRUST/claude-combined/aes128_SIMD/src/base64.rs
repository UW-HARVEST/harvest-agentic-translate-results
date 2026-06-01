use crate::aes::NB;
use crate::cipher_utils::g_mult;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output = vec![0u8; output_len];

    let mut i = 0usize;
    let mut j = 0usize;
    while i < len {
        let octet_a: u32 = if i < len { input[i] as u32 } else { 0 };
        i += 1;
        let octet_b: u32 = if i < len { input[i] as u32 } else { 0 };
        i += 1;
        let octet_c: u32 = if i < len { input[i] as u32 } else { 0 };
        i += 1;

        let triple: u32 = (octet_a << 16) | (octet_b << 8) | octet_c;

        output[j] = BASE64_TABLE[((triple >> 18) & 0x3F) as usize];
        j += 1;
        output[j] = BASE64_TABLE[((triple >> 12) & 0x3F) as usize];
        j += 1;
        output[j] = BASE64_TABLE[((triple >> 6) & 0x3F) as usize];
        j += 1;
        output[j] = BASE64_TABLE[(triple & 0x3F) as usize];
        j += 1;
    }

    let pad_count = (3 - (len % 3)) % 3;
    for _ in 0..pad_count {
        j -= 1;
        output[j] = b'=';
    }

    // Safe because BASE64_TABLE only contains ASCII characters and '=' is ASCII.
    String::from_utf8(output).expect("base64 output should be valid UTF-8")
}

/// Multiplies two bytes in GF(2^8). Equivalent to g_mult.
pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}

/// Multiplies each pair of corresponding bytes in two 128-bit vectors in GF(2^8).
pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // Extract bytes, multiply each pair, then pack back into __m128i.
    // SAFETY: __m128i is a 16-byte SIMD type. We can transmute to/from [u8; 16]
    // because the layout is well-defined as 16 bytes.
    let a: [u8; 16] = unsafe { std::mem::transmute(first) };
    let b: [u8; 16] = unsafe { std::mem::transmute(second) };
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = g_mult(a[i], b[i]);
    }
    unsafe { std::mem::transmute(out) }
}

/// SSE version of MixColumns. Same logical behavior as `columns`.
pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    let mut temp_state = [[0u8; NB]; 4];
    for c in 0..NB {
        temp_state[0][c] = g_mult(0x02, state[0][c])
            ^ g_mult(0x03, state[1][c])
            ^ state[2][c]
            ^ state[3][c];
        temp_state[1][c] = state[0][c]
            ^ g_mult(0x02, state[1][c])
            ^ g_mult(0x03, state[2][c])
            ^ state[3][c];
        temp_state[2][c] = state[0][c]
            ^ state[1][c]
            ^ g_mult(0x02, state[2][c])
            ^ g_mult(0x03, state[3][c]);
        temp_state[3][c] = g_mult(0x03, state[0][c])
            ^ state[1][c]
            ^ state[2][c]
            ^ g_mult(0x02, state[3][c]);
    }
    *state = temp_state;
}
