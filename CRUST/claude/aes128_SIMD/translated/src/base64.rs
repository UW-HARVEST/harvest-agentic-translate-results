use crate::aes::NB;
use crate::cipher_utils::g_mult;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64-encode `input[..len]` using the standard alphabet and `=` padding.
pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output: Vec<u8> = Vec::with_capacity(output_len);

    let mut i = 0usize;
    while i < len {
        let octet_a = input[i] as u32;
        i += 1;
        let octet_b = if i < len {
            let v = input[i] as u32;
            i += 1;
            v
        } else {
            0
        };
        let octet_c = if i < len {
            let v = input[i] as u32;
            i += 1;
            v
        } else {
            0
        };

        let triple = (octet_a << 16) | (octet_b << 8) | octet_c;

        output.push(BASE64_TABLE[((triple >> 18) & 0x3F) as usize]);
        output.push(BASE64_TABLE[((triple >> 12) & 0x3F) as usize]);
        output.push(BASE64_TABLE[((triple >> 6) & 0x3F) as usize]);
        output.push(BASE64_TABLE[(triple & 0x3F) as usize]);
    }

    // Replace trailing characters with `=` according to padding length.
    let pad_count = (3 - (len % 3)) % 3;
    let total = output.len();
    for k in 0..pad_count {
        output[total - 1 - k] = b'=';
    }

    // SAFETY: Base64 alphabet and `=` are all valid ASCII so the result is
    // valid UTF-8.
    String::from_utf8(output).expect("base64 output is always valid UTF-8")
}

/// Galois Field multiplication (used by the SSE-flavored helpers). The
/// implementation is identical to the scalar `g_mult` in `cipher_utils`.
pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}

/// Vectorized Galois Field multiplication. The original C version operated
/// on a 16-byte SSE register; we reproduce the same semantics in safe Rust by
/// performing the per-lane multiplication manually.
pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // SAFETY: `__m128i` is `#[repr(transparent)]` over a 16-byte SIMD value
    // and is always 16 bytes wide. We only read/write the bytes via aligned
    // memory operations.
    let first_bytes: [u8; 16] = unsafe { std::mem::transmute(first) };
    let second_bytes: [u8; 16] = unsafe { std::mem::transmute(second) };
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = g_mult(first_bytes[i], second_bytes[i]);
    }
    unsafe { std::mem::transmute(out) }
}

/// SSE-flavored MixColumns over the AES state. Uses the same MixColumns
/// transform as the scalar implementation, but written here for parity with
/// the original C API.
pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    let mut temp_state = [[0u8; NB]; 4];
    for c in 0..NB {
        temp_state[0][c] = g_mult_sse_byte(0x02, state[0][c])
            ^ g_mult_sse_byte(0x03, state[1][c])
            ^ state[2][c]
            ^ state[3][c];
        temp_state[1][c] = state[0][c]
            ^ g_mult_sse_byte(0x02, state[1][c])
            ^ g_mult_sse_byte(0x03, state[2][c])
            ^ state[3][c];
        temp_state[2][c] = state[0][c]
            ^ state[1][c]
            ^ g_mult_sse_byte(0x02, state[2][c])
            ^ g_mult_sse_byte(0x03, state[3][c]);
        temp_state[3][c] = g_mult_sse_byte(0x03, state[0][c])
            ^ state[1][c]
            ^ state[2][c]
            ^ g_mult_sse_byte(0x02, state[3][c]);
    }
    *state = temp_state;
}
