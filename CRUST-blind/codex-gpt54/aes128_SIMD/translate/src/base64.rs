use crate::aes::NB;
use std::arch::x86_64::__m128i;
pub fn base64_encode(input: &[u8], len: usize) -> String {
    const BASE64_TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let len = len.min(input.len());
    let output_len = 4 * len.div_ceil(3);
    let mut output = vec![0u8; output_len];

    let mut i = 0usize;
    let mut j = 0usize;
    while i < len {
        let octet_a = input[i] as u32;
        i += 1;
        let octet_b = if i < len {
            let byte = input[i] as u32;
            i += 1;
            byte
        } else {
            0
        };
        let octet_c = if i < len {
            let byte = input[i] as u32;
            i += 1;
            byte
        } else {
            0
        };

        let triple = (octet_a << 16) | (octet_b << 8) | octet_c;
        output[j] = BASE64_TABLE[((triple >> 18) & 0x3f) as usize];
        output[j + 1] = BASE64_TABLE[((triple >> 12) & 0x3f) as usize];
        output[j + 2] = BASE64_TABLE[((triple >> 6) & 0x3f) as usize];
        output[j + 3] = BASE64_TABLE[(triple & 0x3f) as usize];
        j += 4;
    }

    let pad_count = (3 - (len % 3)) % 3;
    for _ in 0..pad_count {
        j -= 1;
        output[j] = b'=';
    }

    match String::from_utf8(output) {
        Ok(encoded) => encoded,
        Err(err) => String::from_utf8_lossy(&err.into_bytes()).into_owned(),
    }
}
pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    crate::cipher_utils::g_mult(first, second)
}
pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    let first_bytes: [u8; 16] = unsafe { std::mem::transmute(first) };
    let second_bytes: [u8; 16] = unsafe { std::mem::transmute(second) };
    let mut out = [0u8; 16];

    for i in 0..16 {
        out[i] = g_mult_sse_byte(first_bytes[i], second_bytes[i]);
    }

    unsafe { std::mem::transmute(out) }
}
pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    crate::matrix::columns(state)
}
