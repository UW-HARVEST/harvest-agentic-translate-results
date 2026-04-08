use crate::aes::NB;
use crate::cipher_utils::g_mult;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output = vec![0u8; output_len];
    let mut i = 0usize;
    let mut j = 0usize;
    while i < len {
        let octet_a = if i < len { let v = input[i]; i += 1; v as u32 } else { 0 };
        let octet_b = if i < len { let v = input[i]; i += 1; v as u32 } else { 0 };
        let octet_c = if i < len { let v = input[i]; i += 1; v as u32 } else { 0 };
        let triple = (octet_a << 16) | (octet_b << 8) | octet_c;
        output[j] = BASE64_TABLE[((triple >> 18) & 0x3F) as usize]; j += 1;
        output[j] = BASE64_TABLE[((triple >> 12) & 0x3F) as usize]; j += 1;
        output[j] = BASE64_TABLE[((triple >> 6) & 0x3F) as usize]; j += 1;
        output[j] = BASE64_TABLE[(triple & 0x3F) as usize]; j += 1;
    }
    // Add padding
    let pad = (3 - (len % 3)) % 3;
    for _ in 0..pad {
        j -= 1;
        output[j] = b'=';
    }
    String::from_utf8(output).unwrap()
}

pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}

pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // Extract bytes, multiply each pair with g_mult, pack back
    let mut first_bytes = [0u8; 16];
    let mut second_bytes = [0u8; 16];
    let mut result_bytes = [0u8; 16];
    unsafe {
        std::arch::x86_64::_mm_storeu_si128(first_bytes.as_mut_ptr() as *mut __m128i, first);
        std::arch::x86_64::_mm_storeu_si128(second_bytes.as_mut_ptr() as *mut __m128i, second);
    }
    for i in 0..16 {
        result_bytes[i] = g_mult(first_bytes[i], second_bytes[i]);
    }
    unsafe { std::arch::x86_64::_mm_loadu_si128(result_bytes.as_ptr() as *const __m128i) }
}

pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    let mut temp = [[0u8; NB]; 4];
    for c in 0..NB {
        temp[0][c] = g_mult(0x02, state[0][c]) ^ g_mult(0x03, state[1][c]) ^ state[2][c] ^ state[3][c];
        temp[1][c] = state[0][c] ^ g_mult(0x02, state[1][c]) ^ g_mult(0x03, state[2][c]) ^ state[3][c];
        temp[2][c] = state[0][c] ^ state[1][c] ^ g_mult(0x02, state[2][c]) ^ g_mult(0x03, state[3][c]);
        temp[3][c] = g_mult(0x03, state[0][c]) ^ state[1][c] ^ state[2][c] ^ g_mult(0x02, state[3][c]);
    }
    *state = temp;
}
