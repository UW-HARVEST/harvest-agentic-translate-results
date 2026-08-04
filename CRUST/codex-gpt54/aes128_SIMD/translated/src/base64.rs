use crate::aes::NB;
use crate::cipher_utils::g_mult;
use crate::matrix::columns;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8], len: usize) -> String {
    let len = len.min(input.len());
    let output_len = 4 * len.div_ceil(3);
    let mut output = String::with_capacity(output_len);

    let mut i = 0usize;
    while i < len {
        let octet_a = input[i] as u32;
        i += 1;
        let octet_b = if i < len {
            let value = input[i] as u32;
            i += 1;
            value
        } else {
            0
        };
        let octet_c = if i < len {
            let value = input[i] as u32;
            i += 1;
            value
        } else {
            0
        };

        let triple = (octet_a << 16) | (octet_b << 8) | octet_c;
        output.push(BASE64_TABLE[((triple >> 18) & 0x3f) as usize] as char);
        output.push(BASE64_TABLE[((triple >> 12) & 0x3f) as usize] as char);
        output.push(BASE64_TABLE[((triple >> 6) & 0x3f) as usize] as char);
        output.push(BASE64_TABLE[(triple & 0x3f) as usize] as char);
    }

    for _ in 0..((3 - (len % 3)) % 3) {
        output.pop();
        output.push('=');
    }

    output
}
pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}
pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // __m128i has no safe bytewise API; transmuting to fixed-size arrays keeps
    // the operation local and mirrors the C helper's lane-wise intent.
    let first_bytes: [u8; 16] = unsafe { std::mem::transmute(first) };
    let second_bytes: [u8; 16] = unsafe { std::mem::transmute(second) };
    let mut out = [0u8; 16];

    for i in 0..16 {
        out[i] = g_mult_sse_byte(first_bytes[i], second_bytes[i]);
    }

    unsafe { std::mem::transmute(out) }
}
pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    columns(state)
}
