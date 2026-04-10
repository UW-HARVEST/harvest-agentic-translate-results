use crate::aes::NB;
use crate::cipher_utils::g_mult;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

const BASE64_TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output = Vec::with_capacity(output_len);
    let mut i = 0;
    while i < len {
        let octet_a = if i < len { let v = input[i]; i += 1; v as u32 } else { i += 1; 0 };
        let octet_b = if i < len { let v = input[i]; i += 1; v as u32 } else { i += 1; 0 };
        let octet_c = if i < len { let v = input[i]; i += 1; v as u32 } else { i += 1; 0 };
        let triple = (octet_a << 16) | (octet_b << 8) | octet_c;
        output.push(BASE64_TABLE[((triple >> 18) & 0x3F) as usize]);
        output.push(BASE64_TABLE[((triple >> 12) & 0x3F) as usize]);
        output.push(BASE64_TABLE[((triple >> 6) & 0x3F) as usize]);
        output.push(BASE64_TABLE[(triple & 0x3F) as usize]);
    }
    // Replace trailing bytes with '=' for padding
    let pad_count = (3 - (len % 3)) % 3;
    let final_len = output.len();
    for k in 0..pad_count {
        output[final_len - 1 - k] = b'=';
    }
    String::from_utf8(output).unwrap()
}

pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}

#[cfg(target_arch = "x86_64")]
pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    unsafe {
        let mut first_bytes = [0u8; 16];
        let mut second_bytes = [0u8; 16];
        _mm_storeu_si128(first_bytes.as_mut_ptr() as *mut __m128i, first);
        _mm_storeu_si128(second_bytes.as_mut_ptr() as *mut __m128i, second);
        let mut result = [0u8; 16];
        for i in 0..16 {
            result[i] = g_mult(first_bytes[i], second_bytes[i]);
        }
        _mm_loadu_si128(result.as_ptr() as *const __m128i)
    }
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
