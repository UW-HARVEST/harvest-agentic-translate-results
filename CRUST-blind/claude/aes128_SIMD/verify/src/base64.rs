use crate::aes::NB;
use crate::cipher_utils::g_mult;
use std::arch::x86_64::__m128i;

const BASE64_TABLE: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8], len: usize) -> String {
    let output_len = 4 * ((len + 2) / 3);
    let mut output = vec![0u8; output_len];

    let mut i = 0usize;
    let mut j = 0usize;
    while i < len {
        let octet_a: u32 = if i < len {
            let v = input[i] as u32;
            i += 1;
            v
        } else {
            0
        };
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

    String::from_utf8(output).expect("base64 output is ASCII")
}

pub fn g_mult_sse_byte(first: u8, second: u8) -> u8 {
    // Simple GF(2^8) multiplication, identical semantics to g_mult
    g_mult(first, second)
}

pub fn g_mult_sse(first: __m128i, second: __m128i) -> __m128i {
    // Treat the two __m128i values as 16 bytes each and multiply them
    // element-wise in GF(2^8). This matches the byte-wise behavior implied
    // by g_mult_sse_byte.
    use std::arch::x86_64::{_mm_loadu_si128, _mm_storeu_si128};

    let mut a_bytes = [0u8; 16];
    let mut b_bytes = [0u8; 16];
    let mut out_bytes = [0u8; 16];

    // SAFETY: __m128i is a 128-bit SIMD register. Storing it into a
    // 16-byte aligned-or-unaligned buffer is well-defined via
    // _mm_storeu_si128. We then process the bytes purely in safe Rust.
    unsafe {
        _mm_storeu_si128(a_bytes.as_mut_ptr() as *mut __m128i, first);
        _mm_storeu_si128(b_bytes.as_mut_ptr() as *mut __m128i, second);
    }

    for i in 0..16 {
        out_bytes[i] = g_mult(a_bytes[i], b_bytes[i]);
    }

    // SAFETY: Loading 16 bytes into a __m128i is the inverse of the store.
    unsafe { _mm_loadu_si128(out_bytes.as_ptr() as *const __m128i) }
}

pub fn columns_sse(state: &mut [[u8; NB]; 4]) {
    // Same as the standard MixColumns transformation; the C header declares
    // this but no implementation is shipped, so we provide a faithful
    // reference implementation matching `Columns`.
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
