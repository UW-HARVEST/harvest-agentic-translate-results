use crate::aes::NB;
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

    // SAFETY: All bytes are ASCII, so this is valid UTF-8.
    String::from_utf8(output).expect("base64 output should be valid UTF-8")
}

pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    // Byte-level GF(2^8) multiplication, mirroring g_mult.
    let mut p: u8 = 0;
    let mut a = first;
    let mut b = second;
    for _ in 0..8 {
        if b & 1 != 0 {
            p ^= a;
        }
        let hi_bit_set = a & 0x80;
        a = a.wrapping_shl(1);
        if hi_bit_set != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    p
}

pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // Perform GF(2^8) multiplication for each of the 16 bytes in parallel.
    use std::arch::x86_64::{_mm_loadu_si128, _mm_storeu_si128};

    unsafe {
        let mut a_bytes = [0u8; 16];
        let mut b_bytes = [0u8; 16];
        _mm_storeu_si128(a_bytes.as_mut_ptr() as *mut __m128i, first);
        _mm_storeu_si128(b_bytes.as_mut_ptr() as *mut __m128i, second);

        let mut result = [0u8; 16];
        for i in 0..16 {
            result[i] = g_mult_sse_byte(a_bytes[i], b_bytes[i]);
        }

        _mm_loadu_si128(result.as_ptr() as *const __m128i)
    }
}

pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    // SSE-equivalent MixColumns (mirrors the scalar `columns`).
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
