use crate::aes::NB;
use crate::cipher_utils::g_mult;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 encoding (with `=` padding) of the first `len` bytes of `input`.
pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output = vec![0u8; output_len];

    let mut i = 0usize;
    let mut j = 0usize;
    while i < len {
        let octet_a: u32 = input[i] as u32;
        i += 1;
        let octet_b: u32 = if i < len {
            let v = input[i] as u32;
            i += 1;
            v
        } else {
            0
        };
        let octet_c: u32 = if i < len {
            let v = input[i] as u32;
            i += 1;
            v
        } else {
            0
        };

        let triple = (octet_a << 16) | (octet_b << 8) | octet_c;

        output[j] = BASE64_TABLE[((triple >> 18) & 0x3F) as usize];
        output[j + 1] = BASE64_TABLE[((triple >> 12) & 0x3F) as usize];
        output[j + 2] = BASE64_TABLE[((triple >> 6) & 0x3F) as usize];
        output[j + 3] = BASE64_TABLE[(triple & 0x3F) as usize];
        j += 4;
    }

    let pad_count = (3 - (len % 3)) % 3;
    for _ in 0..pad_count {
        j -= 1;
        output[j] = b'=';
    }

    String::from_utf8(output).expect("base64 output is always ASCII")
}

/// Single-byte GF(2^8) multiply (functional equivalent to the C `g_mult_sse_byte`).
pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}

/// Lane-wise GF(2^8) multiply over a 16-byte vector.
pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // The C declaration of `g_mult_sse` is never implemented in the C sources,
    // so we provide a faithful lane-wise GF(2^8) multiplication here using
    // safe element extraction.
    let a_bytes: [u8; 16] = unsafe { std::mem::transmute(first) };
    let b_bytes: [u8; 16] = unsafe { std::mem::transmute(second) };
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = g_mult(a_bytes[i], b_bytes[i]);
    }
    unsafe { std::mem::transmute(out) }
}

/// SIMD-flavored MixColumns; result matches the scalar `columns` implementation.
pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    crate::matrix::columns(state);
}
