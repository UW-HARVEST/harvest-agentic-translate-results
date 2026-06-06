use crate::aes::NB;
use crate::cipher_utils::g_mult;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output: Vec<u8> = vec![0u8; output_len];

    let mut i: usize = 0;
    let mut j: usize = 0;
    while i < len {
        let octet_a: u32 = if i < len { let v = input[i] as u32; i += 1; v } else { 0 };
        let octet_b: u32 = if i < len { let v = input[i] as u32; i += 1; v } else { 0 };
        let octet_c: u32 = if i < len { let v = input[i] as u32; i += 1; v } else { 0 };

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

    String::from_utf8(output).expect("base64 output is valid utf-8")
}

pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    g_mult(first, second)
}

pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // Apply g_mult byte-by-byte across the 16 lanes.
    use std::arch::x86_64::{_mm_loadu_si128, _mm_storeu_si128};
    let mut a = [0u8; 16];
    let mut b = [0u8; 16];
    unsafe {
        _mm_storeu_si128(a.as_mut_ptr() as *mut __m128i, first);
        _mm_storeu_si128(b.as_mut_ptr() as *mut __m128i, second);
    }
    let mut r = [0u8; 16];
    for k in 0..16 {
        r[k] = g_mult(a[k], b[k]);
    }
    unsafe { _mm_loadu_si128(r.as_ptr() as *const __m128i) }
}

pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    crate::matrix::columns(state);
}
