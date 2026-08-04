use crate::aes::NB;
use crate::cipher_utils::g_mult;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output = vec![0u8; output_len];

    let mut i = 0;
    let mut j = 0;
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

    let pad = (3 - (len % 3)) % 3;
    for _ in 0..pad {
        j -= 1;
        output[j] = b'=';
    }

    String::from_utf8(output).unwrap()
}

/// Multiply two bytes in GF(2^8) - mirrors the byte-wise behavior used in
/// the (header-declared) SSE g_mult routines.
pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}

/// SIMD GF(2^8) multiply lanewise on 16-byte vectors.
/// Header declares this in C but the C source never defines it; we provide
/// a safe-equivalent implementation that performs the lanewise GF multiply.
pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    use std::arch::x86_64::{_mm_loadu_si128, _mm_storeu_si128};
    let mut a_bytes = [0u8; 16];
    let mut b_bytes = [0u8; 16];
    unsafe {
        _mm_storeu_si128(a_bytes.as_mut_ptr() as *mut __m128i, first);
        _mm_storeu_si128(b_bytes.as_mut_ptr() as *mut __m128i, second);
    }
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = g_mult(a_bytes[i], b_bytes[i]);
    }
    unsafe { _mm_loadu_si128(out.as_ptr() as *const __m128i) }
}

/// Equivalent of MixColumns using GF(2^8) multiplications.
/// Header declares this in C but the C source never defines it; we provide
/// a safe-equivalent implementation matching the standard MixColumns
/// (same as `columns` in `matrix.rs`).
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
